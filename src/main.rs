mod api;
mod db;
mod types;
mod usage_collector;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let (db_tx, db_rx) = db::create_command_channel();
    let db_handle = db::start(db_rx);
    let api_future = api::start(db_tx.clone());
    let usage_collector_future = usage_collector::start(db_tx);

    tokio::select! {
        Ok(Err(e)) = db_handle => {
            log::error!("Error in database process: {e}");
        }
        Err(e) = api_future => {
            log::error!("Error in api process: {e}");
        }
        Err(e) = usage_collector_future => {
            log::error!("Error in usage collector process: {e}");
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received Ctrl+C, shutting down gracefully...");
        }
        else => {
            log::error!("Unexpected termination, shutting down...");
        }
    }

    Ok(())
}
