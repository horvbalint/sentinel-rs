use crate::db::DbCommand;

mod api;
mod db;
mod host;
mod info_collector;
mod types;

#[tokio::main]
async fn main() {
    let (db_tx, db_rx) = tokio::sync::mpsc::unbounded_channel::<DbCommand>();

    let db_future = db::task(db_rx);
    let server_future = api::task(db_tx.clone());
    let info_collector_future = info_collector::task(db_tx);

    let (db_result, server_result, _) =
        tokio::join!(db_future, server_future, info_collector_future);

    if let Err(e) = db_result {
        eprintln!("Error in database thread: {:#?}", e);
    }

    if let Err(e) = server_result {
        eprintln!("Error in server thread: {:#?}", e);
    }
}
