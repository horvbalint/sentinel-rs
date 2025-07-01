mod api;
mod db;
mod types;
mod usage_collector;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let (db_tx, db_rx) = db::create_command_channel();
    let db_handle = db::start(db_rx);
    let server_future = api::start(db_tx.clone());
    let usage_collector_future = usage_collector::start(db_tx);

    tokio::select! {
        Ok(Err(e)) = db_handle => {
            log::error!("Error in database process: {e}");
        }
        Err(e) = server_future => {
            log::error!("Error in server process: {e}");
        }
        Err(e) = usage_collector_future => {
            log::error!("Error in usage collector process: {e}");
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("Received Ctrl+C, shutting down gracefully...");
        }
        else => {
            log::warn!("Unexpected termination, shutting down...");
        }
    }

    Ok(())
}
