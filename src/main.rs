mod errors;
mod structures;

use crate::errors::KrakenError;
use crate::errors::KrakenError::Error;
use crate::structures::{ClientAccount, Transaction, TransactionType};
use anyhow::Result;
use itertools::multizip;
use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::{env, thread};

// I debated between this LazyFrame implementation and streaming with `csv-async`. This was far less
// verbose and might actually tolerate very-large datasets.
// Docs: https://docs.pola.rs/user-guide/io/csv/#read-write
fn parse_csv(file_in: &str) -> Result<LazyFrame> {
    let schema = Schema::from_iter(vec![
        Field::new("type".into(), DataType::String),
        Field::new("client".into(), DataType::UInt32), // Using U32 due to limitations on the CSV reader's functionality
        Field::new("tx".into(), DataType::UInt32),
        Field::new("amount".into(), DataType::Float64),
    ]);
    Ok(LazyCsvReader::new(PlPath::new(file_in))
        .with_schema(Some(SchemaRef::from(schema)))
        .with_has_header(false)
        .with_skip_rows(1)
        .finish()?) // Skipping rows in order to compensate for the lack of a `with_clean_column_names` method for lazy readers
}

fn compute_account_totals(path: &str) -> Result<Arc<Mutex<HashMap<u32, ClientAccount>>>> {
    // Don't need to drop, since it's lazy and is memory-light
    let lazy_data: LazyFrame = parse_csv(path)?;

    // Partition by client to simplify downstream logic. Not required, and may not yield any performance improvement.
    let parts = Arc::new(lazy_data.collect()?.partition_by(["client"], true)?);

    // Wrap the HashMap in an multi-threaded ref counter and simple lock
    let client_accounts: Arc<Mutex<HashMap<u32, ClientAccount>>> = Arc::new(Mutex::new(HashMap::new())); // Master collection of accounts

    // Collect a list of thread handles to join and prevent dangling threads from dying as main is terminated
    let mut handles = vec![];

    for df in &*parts {

        // Clone the ref counter
        let accounts = client_accounts.clone();
        let handle = thread::spawn(move || {

            // Use individual synchronized iterators for each column. Iterating by row is a discouraged
            // antipattern, as the docs/stackoverflow made abundantly clear.

            let columns = df.columns(["type", "client", "tx", "amount"]).unwrap();

            let type_col_iter = columns[0].str().unwrap().iter();
            let client_col_iter = columns[1].u32().unwrap().iter(); // Using U32 due to limitations on the CSV reader's functionality
            let tx_col_iter = columns[2].u32().unwrap().iter();
            let amount_col_iter = columns[3].f64().unwrap().iter();

            let full_row_iter =
                multizip((type_col_iter, client_col_iter, tx_col_iter, amount_col_iter));

            let transaction_objects: Vec<Transaction> = full_row_iter
                .map(|(kind, client, tx, amount)| Transaction {
                    kind: TransactionType::try_from(kind.expect("Type may not be null"))
                        .expect(format!("Invalid transaction type: {:#?}", kind).as_str()),
                    client: client.expect("client may not be null"),
                    amount,
                    tx: tx.expect(""),
                    state: None,
                })
                .collect();

            let client_id = transaction_objects[0].client;
            let mut account: ClientAccount = Default::default();

            for transaction in transaction_objects {
                // Swallow results since we aren't tracking them
                match account.apply_transaction(transaction) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }

            let mut accounts_lock = accounts.lock().unwrap();
            accounts_lock.insert(client_id, account);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let account_lock = client_accounts.lock().unwrap();
    println!("client, available, held, total, locked");
    for key in account_lock.keys() {
        if let Some(account) = account_lock.get(key) {
            println!("{}", account.to_str_row(*key))
        }
    }

    Ok(client_accounts.clone())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Invalid arguments: Must supply path to data csv");
        Err(Error)?
    }

    let path = Path::new(&args[1]);
    if !path.exists() {
        Err(KrakenError::IO)?
    }

    compute_account_totals(path.to_str().unwrap()).expect("");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::compute_account_totals;

    const TEST_DIR: &str = "./test/";
    const TEST_CASES: [(&str, &str); 5] = [
        ("0-trivial.csv", "1, 1.5000, 0.0000, 1.5000, false"),
        ("1-dispute-after-withdraw.csv", "1, -9.5000, 10.0000, 0.5000, false"),
        ("2-chargeback-after-withdraw.csv", "1, -9.5000, 0.0000, -9.5000, true"),
        ("3-resolve-without-dispute.csv", "1, 11.0000, 0.0000, 11.0000, false"),
        ("4-oversized-withdrawal.csv", "1, 100.0000, 0.0000, 100.0000, false")
    ];
    #[test]
    fn test_csv() {
        for (file_name, expected) in TEST_CASES {
            let totals = compute_account_totals((String::from(TEST_DIR) + file_name).as_str()).unwrap();
            assert_eq!(String::from(expected), totals.get(&1).expect("").to_str_row(1))
        }
    }
}
