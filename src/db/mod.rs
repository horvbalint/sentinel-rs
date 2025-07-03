use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use tokio::{
    sync::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::types::{CpuUsage, CpuUsageDataPoint, MemoryUsage, MemoryUsageDataPoint};
use manager::DbManager;

mod manager;

pub enum DbCommand {
    InsertResourceUsage {
        timestamp: DateTime<Utc>,
        cpu_usage: CpuUsage,
        memory_usage: MemoryUsage,
        container: Option<String>,
    },
    GetLastCpuUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<CpuUsageDataPoint>>,
    },
    GetLastMemoryUsage {
        container: Option<String>,
        respond_to: oneshot::Sender<Option<MemoryUsageDataPoint>>,
    },
    GetCpuUsageHistory {
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        container: Option<String>,
        respond_to: oneshot::Sender<Vec<CpuUsageDataPoint>>,
    },
    GetMemoryUsageHistory {
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        container: Option<String>,
        respond_to: oneshot::Sender<Vec<MemoryUsageDataPoint>>,
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
                DbCommand::GetCpuUsageHistory {
                    from,
                    to,
                    container,
                    respond_to,
                } => {
                    let result = db
                        .get_cpu_usage_history(from, to, container)
                        .unwrap_or_default();
                    let _ = respond_to.send(result);
                }
                DbCommand::GetMemoryUsageHistory {
                    from,
                    to,
                    container,
                    respond_to,
                } => {
                    let result = db
                        .get_memory_usage_history(from, to, container)
                        .unwrap_or_default();
                    let _ = respond_to.send(result);
                }
            };
        }

        Ok(())
    })
}
