use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, Params, Statement};
use tokio::{
    sync::{mpsc::UnboundedSender, oneshot},
    task::JoinHandle,
};

use crate::types::{CpuUsage, MemoryUsage};

#[derive(Debug)]
pub struct Db<'conn> {
    insert_cpu_stmt: Statement<'conn>,
    insert_memory_stmt: Statement<'conn>,
    get_last_cpu_container_stmt: Statement<'conn>,
    get_last_cpu_host_stmt: Statement<'conn>,
    get_last_memory_container_stmt: Statement<'conn>,
    get_last_memory_host_stmt: Statement<'conn>,
}

impl<'conn> Db<'conn> {
    pub fn new(connection: &'conn Connection) -> Result<Self> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS cpu_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                percentage REAL NOT NULL,
                timestamp DATETIME NOT NULL,
                container CHAR(64)
            )",
            (),
        )?;

        connection.execute(
            "CREATE TABLE IF NOT EXISTS memory_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                total INTEGER NOT NULL,
                used INTEGER NOT NULL,
                percentage REAL NOT NULL,
                timestamp DATETIME NOT NULL,
                container CHAR(64)
            )",
            (),
        )?;

        Ok(Self {
            insert_cpu_stmt: connection
                .prepare("INSERT INTO cpu_usage (percentage, timestamp, container) VALUES (?1, ?2, ?3)")?,
            insert_memory_stmt: connection
                .prepare("INSERT INTO memory_usage (total, used, percentage, timestamp, container) VALUES (?1, ?2, ?3, ?4, ?5)",)?,
            get_last_cpu_container_stmt: connection
                .prepare("SELECT percentage FROM cpu_usage WHERE container LIKE (?1 || '%') ORDER BY timestamp DESC LIMIT 1")?,
            get_last_memory_container_stmt: connection
                .prepare("SELECT total, used, percentage FROM memory_usage WHERE container LIKE (?1 || '%') ORDER BY timestamp DESC LIMIT 1")?,
            get_last_cpu_host_stmt: connection
                .prepare("SELECT percentage FROM cpu_usage WHERE container IS NULL ORDER BY timestamp DESC LIMIT 1")?,
            get_last_memory_host_stmt: connection
                .prepare("SELECT total, used, percentage FROM memory_usage WHERE container IS NULL ORDER BY timestamp DESC LIMIT 1")?,
        })
    }

    pub fn insert_resource_usage(
        &mut self,
        timestamp: chrono::NaiveDateTime,
        memory_usage: MemoryUsage,
        cpu_usage: CpuUsage,
        container: Option<String>,
    ) -> Result<()> {
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        self.insert_cpu_stmt.execute((
            cpu_usage.percentage,
            timestamp_str.clone(),
            container.clone(),
        ))?;

        self.insert_memory_stmt.execute((
            memory_usage.total,
            memory_usage.used,
            memory_usage.percentage,
            timestamp_str,
            container,
        ))?;

        Ok(())
    }

    pub fn get_last_cpu_usage(&mut self, container: Option<String>) -> Option<CpuUsage> {
        match container {
            Some(container) => {
                Self::query_last_cpu_usage(&mut self.get_last_cpu_container_stmt, [container])
            }
            None => Self::query_last_cpu_usage(&mut self.get_last_cpu_host_stmt, []),
        }
    }

    fn query_last_cpu_usage(stmt: &mut Statement, params: impl Params) -> Option<CpuUsage> {
        stmt.query_row(params, |row| {
            Ok(CpuUsage {
                percentage: row.get(0)?,
            })
        })
        .optional()
        .expect("Failed to get last CPU usage")
    }

    pub fn get_last_memory_usage(&mut self, container: Option<String>) -> Option<MemoryUsage> {
        match container {
            Some(container) => {
                Self::query_last_memory_usage(&mut self.get_last_memory_container_stmt, [container])
            }
            None => Self::query_last_memory_usage(&mut self.get_last_memory_host_stmt, []),
        }
    }

    fn query_last_memory_usage(stmt: &mut Statement, params: impl Params) -> Option<MemoryUsage> {
        stmt.query_row(params, |row| {
            Ok(MemoryUsage {
                total: row.get(0)?,
                used: row.get(1)?,
                percentage: row.get(2)?,
            })
        })
        .optional()
        .expect("Failed to get last memory usage")
    }
}

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

pub type DbCommandChannel = UnboundedSender<DbCommand>;

pub fn task() -> (DbCommandChannel, JoinHandle<Result<()>>) {
    let (db_tx, mut db_rx) = tokio::sync::mpsc::unbounded_channel::<DbCommand>();

    let task = tokio::task::spawn_blocking(move || {
        let connection = Connection::open("./test.db")?;
        let mut db = Db::new(&connection)?;

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
                    respond_to
                        .send(db.get_last_cpu_usage(container))
                        .expect("failed to send response - receiver dropped");
                }
                DbCommand::GetLastMemoryUsage {
                    container,
                    respond_to,
                } => {
                    respond_to
                        .send(db.get_last_memory_usage(container))
                        .expect("failed to send response - receiver dropped");
                }
            };
        }

        Ok(())
    });

    (db_tx, task)
}
