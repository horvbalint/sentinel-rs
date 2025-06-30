use rusqlite::{Connection, OptionalExtension, Params, Statement};

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
    pub fn new(connection: &'conn Connection) -> Self {
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS cpu_usage (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    usage REAL NOT NULL,
                    timestamp DATETIME NOT NULL,
                    container CHAR(64)
                )",
                (),
            )
            .expect("Failed to create cpu_usage table");

        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS memory_usage (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    total INTEGER NOT NULL,
                    used INTEGER NOT NULL,
                    timestamp DATETIME NOT NULL,
                    container CHAR(64)
                )",
                (),
            )
            .expect("Failed to create memory_usage table");

        Self {
            insert_cpu_stmt: connection
                .prepare("INSERT INTO cpu_usage (usage, timestamp, container) VALUES (?1, ?2, ?3)")
                .expect("Failed to prepare CPU insert statement"),
            insert_memory_stmt: connection
                .prepare(
                    "INSERT INTO memory_usage (total, used, timestamp, container) VALUES (?1, ?2, ?3, ?4)",
                )
                .expect("Failed to prepare memory insert statement"),
            get_last_cpu_container_stmt: connection
                .prepare("SELECT usage FROM cpu_usage WHERE container LIKE (?1 || '%') ORDER BY timestamp DESC LIMIT 1")
                .expect("Failed to prepare container CPU select statement"),
            get_last_memory_container_stmt: connection
                .prepare("SELECT total, used FROM memory_usage WHERE container LIKE (?1 || '%') ORDER BY timestamp DESC LIMIT 1")
                .expect("Failed to prepare container memory select statement"),
            get_last_cpu_host_stmt: connection
                .prepare("SELECT usage FROM cpu_usage WHERE container IS NULL ORDER BY timestamp DESC LIMIT 1")
                .expect("Failed to prepare host CPU select statement"),
            get_last_memory_host_stmt: connection
                .prepare("SELECT total, used FROM memory_usage WHERE container IS NULL ORDER BY timestamp DESC LIMIT 1")
                .expect("Failed to prepare host memory select statement"),
        }
    }

    pub fn insert_resource_usage(
        &mut self,
        timestamp: chrono::NaiveDateTime,
        memory_usage: MemoryUsage,
        cpu_usage: CpuUsage,
        container: Option<String>,
    ) {
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        self.insert_cpu_stmt
            .execute((cpu_usage.usage, timestamp_str.clone(), container.clone()))
            .expect("Failed to insert CPU usage");

        self.insert_memory_stmt
            .execute((
                memory_usage.total as i64,
                memory_usage.used as i64,
                timestamp_str,
                container,
            ))
            .expect("Failed to insert memory usage");
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
            let usage: f32 = row.get(0)?;
            Ok(CpuUsage { usage })
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
            let total: u64 = row.get(0)?;
            let used: u64 = row.get(1)?;
            Ok(MemoryUsage { total, used })
        })
        .optional()
        .expect("Failed to get last memory usage")
    }
}
