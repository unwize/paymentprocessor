# paymentprocessor

A toy payment processor that reads transactions from a CSV, handles the exchange logic, and then prints the state of each client's account.

## How it works
- Read a `.csv` file with the following columns: `type, client, tx, amount`
- Break it into smaller chunks (`polars` `DataFrame`s in this case)
- For each transaction, convert it to a `Transaction` struct
- Create a `ClientAccount` for that given client, the iterate over each `Transaction` and apply it to the `ClientAccount`
  - In this case, I chose to `impl` a function on the `ClientAccount` to handle this, though it would have been similarly-easy to do this functionally by creating a function that accepts both a `ClientAccount` and `Transaction` to perform the logic.
- The finalized ClientAccount is then stored in a `HashMap`
- Once all `ClientAccounts` are finalized, the `HashMap` is printed in CSV format to `stdout`.

## Performance

This is a trivial implementation of a single-threaded, naive processor. There's many, many areas for improvement.

For starters, the inclusion of `Polars` was entirely-optional. My usage of it wasn't exactly optimal either, as this was my first time ever looking at it. In general, we don't benefit much from batch-processing queries via `polars-lazy`.

Secondly, there's a big missed-opportunity to parallelize the processing for each client. This is done via `for`-loop, but it should really have been `StarMap`ed into a bunch of threads. 

## Streaming?

This process is begging to be streamable. You can stream for a csv, or really any streamable source in an `async` context like `tokio` (or via std-rust, which is entirely usable) to save on memory. Streaming also allows you to utilize more-sophisticated techniques like task-queueing, load-balancing consumers, and utilizing distributed storage systems. They're entirely pointless in this contrived scenario, though.

## Assumptions

- An account has two types of _normal_ transactions: `deposit` and `withdrawal`.
- `Normal` transactions may only be performed on accounts that are not locked.
- An account has three types of _abnormal_ transactions: `dispute`, `resolve`, and `chargeback`.
- `Abormal` transactions are affected by (hypothetical) regulations and may still be performed on locked accounts
  - For example, a locked account may still be disputed and charged-back against
- An account may have a negative balance. A negative balance **will not** occur as a result of a withdrawal. Negative balances only occur when the size of a dispute (or chargeback) is larger than the account's available balance.
- `Abnormal` transactions submitted out of order against a `normal` transaction are considered invalid.
  - `Resolve` and `Chargeback` transactions are only valid if a `Dispute` was performed previously.
- `Dispute` transactions may not be opened against `Normal` transactions that have already been `Resolve`d.

## Dependencies
This project's top-level dependencies are:

- Polars: DataFrame and CSV support
- Anyhow: Error-wrangling
- ThisError: Error defining
- IterTools: Columnar-format wrangling
- Crossbeam: Scoped threads

## Tested On, Tested With
Supported and tested on the following triples:

- stable-x86_64-pc-windows-gnu
- stable-x86_64-pc-windows-msvc
- stable-x86_64-unknown-linux-gnu

You can find bite-sized test files in `/test`. Test them with `cargo test`. There's a rudimentary CI pipeline in place via Github Actions that checks, builds, and tests `main`.

There were more minor edge cases (duplicate chargeback, resolve, dispute, etc) that I tested by hand. 

## Safety and Robustness

IO is always unsafe, to a certain degree. I have very little robustness, beyond verifying that the target `csv` exists.

I'm generating, but swallowing errors. As it stands, I _could_ record and trace errors for all sorts of things, from flow errors in the dispute process, to rejected withdrawals, and more. I don't log them since you're automating the process by watching stdout.
