mod errors;
mod structures;

use crate::errors::KrakenError;
use crate::errors::KrakenError::Error;
use anyhow::Result;
use polars::prelude::*;
use std::env;
use std::path::Path;
use itertools::multizip;
use crate::structures::{Transaction, TransactionType};

// I debated between this LazyFrame implementation and streaming with `csv-async`. This was far less
// verbose and might actually tolerate very-large datasets.
// Docs: https://docs.pola.rs/user-guide/io/csv/#read-write
fn parse_csv(file_in: &str) -> Result<LazyFrame> {
    let schema = Schema::from_iter(vec![
        Field::new("type".into(), DataType::String),
        Field::new("client".into(), DataType::UInt32),
        Field::new("tx".into(), DataType::UInt32),
        Field::new("amount".into(), DataType::Float64),
    ]);
    Ok(LazyCsvReader::new(PlPath::new(file_in)).with_schema(Some(SchemaRef::from(schema))).with_has_header(false).with_skip_rows(1).finish()?)
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
    let parts = lazy_data
        .collect()?
        .partition_by(["client"], true)?;


    for df in &parts {
        println!("{:?}", df);
        // Use individual synchronized iterators for each column. Iterating by row is a discouraged
        // antipattern, as the docs/stackoverflow made abundantly clear.

        let columns = df.columns(["type", "client", "tx", "amount"])?;

        let type_col_iter = columns[0].str()?.iter();
        let client_col_iter = columns[1].u32()?.iter();
        let tx_col_iter = columns[2].u32()?.iter();
        let amount_col_iter = columns[3].f64()?.iter();

        let full_row_iter = multizip((type_col_iter, client_col_iter, tx_col_iter, amount_col_iter));

        let transaction_objects: Vec<Transaction> = full_row_iter
            .map(|(kind, client, tx, amount) | Transaction {
                kind: TransactionType::try_from(kind.expect("")).expect(format!("Invalid transaction type: {:#?}", kind).as_str()),
                client: client.expect(""),
                amount: amount.expect(""),
                tx: tx.expect(""),
                state: None,
            }).collect();

        println!("{:#?}", transaction_objects)
    }

    Ok(())
}
