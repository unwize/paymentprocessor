mod errors;
mod structures;

use crate::errors::KrakenError;
use crate::errors::KrakenError::Error;
use crate::structures::{ClientAccount, Transaction, TransactionType};
use anyhow::Result;
use itertools::multizip;
use polars::prelude::*;
use std::collections::HashMap;
use std::env;
use std::path::Path;

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

    // Don't need to drop, since it's lazy and is memory-light
    let lazy_data: LazyFrame = parse_csv(path.to_str().unwrap())?;

    // Partition by client to simplify downstream logic. Not required, and may not yield any performance improvement.
    let parts = lazy_data.collect()?.partition_by(["client"], true)?;

    let mut client_accounts: HashMap<u32, ClientAccount> = HashMap::new(); // Master collection of accounts

    for df in &parts {
        // Use individual synchronized iterators for each column. Iterating by row is a discouraged
        // antipattern, as the docs/stackoverflow made abundantly clear.

        let columns = df.columns(["type", "client", "tx", "amount"])?;

        let type_col_iter = columns[0].str()?.iter();
        let client_col_iter = columns[1].u32()?.iter(); // Using U32 due to limitations on the CSV reader's functionality
        let tx_col_iter = columns[2].u32()?.iter();
        let amount_col_iter = columns[3].f64()?.iter();

        let full_row_iter =
            multizip((type_col_iter, client_col_iter, tx_col_iter, amount_col_iter));

        let transaction_objects: Vec<Transaction> = full_row_iter
            .map(|(kind, client, tx, amount)| Transaction {
                kind: TransactionType::try_from(kind.expect("Type may not be null"))
                    .expect(format!("Invalid transaction type: {:#?}", kind).as_str()),
                client: client.expect("client may not be null"),
                amount: amount,
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

        client_accounts.insert(client_id, account);
    }

    println!("client, available, held, total, locked");
    for key in client_accounts.keys() {
        if let Some(account) = client_accounts.get(key) {
            println!(
                "{}, {:.4}, {:.4}, {:.4}, {}",
                key,
                account.available,
                account.held,
                account.total(),
                account.locked
            )
        }
    }

    Ok(())
}
