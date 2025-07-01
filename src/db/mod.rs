use anyhow::Result;
use rusqlite::Connection;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::types::{CpuUsage, MemoryUsage};
use manager::DbManager;

mod manager;

pub enum DbCommand {
    InsertResourceUsage {
        timestamp: chrono::NaiveDateTime,
        cpu_usage: CpuUsage,
        memory_usage: MemoryUsage,
        container: Option<String>,
    },
    GetLastCpuUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<CpuUsage>>,
    },
    GetLastMemoryUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<MemoryUsage>>,
    },
}

pub type DbChannelTx = UnboundedSender<DbCommand>;
pub type DbChannelRx = UnboundedReceiver<DbCommand>;

pub fn create_command_channel() -> (DbChannelTx, DbChannelRx) {
    tokio::sync::mpsc::unbounded_channel::<DbCommand>()
}

pub fn start(mut db_rx: DbChannelRx) -> JoinHandle<Result<()>> {
    tokio::task::spawn_blocking(move || {
        let connection = Connection::open("./test.db")?;
        let mut db = DbManager::new(&connection)?;

        while let Some(command) = db_rx.blocking_recv() {
            match command {
                DbCommand::InsertResourceUsage {
                    timestamp,
                    memory_usage,
                    cpu_usage,
                    container,
                } => {
                    db.insert_resource_usage(timestamp, memory_usage, cpu_usage, container)?;
                }
                DbCommand::GetLastCpuUsage {
                    container,
                    respond_to,
                } => {
                    let result = db.get_last_cpu_usage(container).unwrap_or(None);
                    let _ = respond_to.send(result);
                }
                DbCommand::GetLastMemoryUsage {
                    container,
                    respond_to,
                } => {
                    let result = db.get_last_memory_usage(container).unwrap_or(None);
                    let _ = respond_to.send(result);
                }
            };
        }

        Ok(())
    })
}
