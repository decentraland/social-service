use sqlx::{Postgres, Transaction};

use crate::components::database::Executor;

pub fn get_transaction_result_from_executor(
    executor_result: Option<Executor>,
) -> Option<Transaction<'_, Postgres>> {
    match executor_result {
        Some(executor) => match executor {
            Executor::Transaction(transaction) => Some(transaction),
            Executor::Pool(_) => None,
        },
        None => None,
    }
}
