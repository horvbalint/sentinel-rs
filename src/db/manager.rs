use anyhow::Result;
use anyhow::anyhow;
use rusqlite::{Connection, OptionalExtension, Params, Statement};

use crate::types::{CpuUsage, MemoryUsage};

#[derive(Debug)]
pub struct DbManager<'conn> {
    insert_cpu_stmt: Statement<'conn>,
    insert_memory_stmt: Statement<'conn>,
    get_last_cpu_container_stmt: Statement<'conn>,
    get_last_cpu_host_stmt: Statement<'conn>,
    get_last_memory_container_stmt: Statement<'conn>,
    get_last_memory_host_stmt: Statement<'conn>,
}

impl<'conn> DbManager<'conn> {
    pub fn new(connection: &'conn Connection) -> Result<Self> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS cpu_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                percentage REAL NOT NULL,
                timestamp DATETIME NOT NULL,
                container CHAR(64)
            );
            
            CREATE TABLE IF NOT EXISTS memory_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                total INTEGER NOT NULL,
                used INTEGER NOT NULL,
                percentage REAL NOT NULL,
                timestamp DATETIME NOT NULL,
                container CHAR(64)
            );

            CREATE INDEX IF NOT EXISTS idx_cpu_timestamp_container ON cpu_usage(timestamp DESC, container);
            CREATE INDEX IF NOT EXISTS idx_memory_timestamp_container ON memory_usage(timestamp DESC, container);",
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

    pub fn get_last_cpu_usage(&mut self, container: Option<String>) -> Result<Option<CpuUsage>> {
        match container {
            Some(container) => {
                Self::query_last_cpu_usage(&mut self.get_last_cpu_container_stmt, [container])
            }
            None => Self::query_last_cpu_usage(&mut self.get_last_cpu_host_stmt, []),
        }
    }

    fn query_last_cpu_usage(stmt: &mut Statement, params: impl Params) -> Result<Option<CpuUsage>> {
        stmt.query_row(params, |row| {
            Ok(CpuUsage {
                percentage: row.get(0)?,
            })
        })
        .optional()
        .map_err(|e| anyhow!("Failed to get last CPU usage: {e}"))
    }

    pub fn get_last_memory_usage(
        &mut self,
        container: Option<String>,
    ) -> Result<Option<MemoryUsage>> {
        match container {
            Some(container) => {
                Self::query_last_memory_usage(&mut self.get_last_memory_container_stmt, [container])
            }
            None => Self::query_last_memory_usage(&mut self.get_last_memory_host_stmt, []),
        }
    }

    fn query_last_memory_usage(
        stmt: &mut Statement,
        params: impl Params,
    ) -> Result<Option<MemoryUsage>> {
        stmt.query_row(params, |row| {
            Ok(MemoryUsage {
                total: row.get(0)?,
                used: row.get(1)?,
                percentage: row.get(2)?,
            })
        })
        .optional()
        .map_err(|e| anyhow!("Failed to get last memory usage: {e}"))
    }
}
