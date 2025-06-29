use rusqlite::{Connection, OptionalExtension};

use crate::types::{CpuUsage, MemoryUsage};

#[derive(Debug)]
pub struct Db<'conn> {
    insert_cpu_stmt: rusqlite::Statement<'conn>,
    insert_memory_stmt: rusqlite::Statement<'conn>,
    get_last_cpu_stmt: rusqlite::Statement<'conn>,
    get_last_memory_stmt: rusqlite::Statement<'conn>,
}

impl<'conn> Db<'conn> {
    pub fn new(connection: &'conn Connection) -> Self {
        connection
            .execute(
                "CREATE TABLE cpu_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                usage REAL NOT NULL,
                timestamp DATETIME
            )",
                (), // empty list of parameters.
            )
            .expect("Failed to create cpu_usage table");

        connection
            .execute(
                "CREATE TABLE memory_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                total INTEGER NOT NULL,
                used INTEGER NOT NULL,
                timestamp DATETIME
            )",
                (), // empty list of parameters.
            )
            .expect("Failed to create memory_usage table");

        Self {
            insert_cpu_stmt: connection
                .prepare("INSERT INTO cpu_usage (usage, timestamp) VALUES (?1, ?2)")
                .expect("Failed to prepare CPU insert statement"),
            insert_memory_stmt: connection
                .prepare("INSERT INTO memory_usage (total, used, timestamp) VALUES (?1, ?2, ?3)")
                .expect("Failed to prepare memory insert statement"),
            get_last_cpu_stmt: connection
                .prepare("SELECT usage FROM cpu_usage ORDER BY id DESC LIMIT 1")
                .expect("Failed to prepare CPU select statement"),
            get_last_memory_stmt: connection
                .prepare("SELECT total, used FROM memory_usage ORDER BY id DESC LIMIT 1")
                .expect("Failed to prepare memory select statement"),
        }
    }

    pub fn insert_usage(
        &mut self,
        timestamp: chrono::NaiveDateTime,
        memory_usage: MemoryUsage,
        cpu_usage: CpuUsage,
    ) {
        let timestamp_str = timestamp.format("%Y-%m-%d %H:%M:%S").to_string();

        self.insert_cpu_stmt
            .execute((cpu_usage.usage, timestamp_str.clone()))
            .expect("Failed to insert CPU usage");

        self.insert_memory_stmt
            .execute((
                memory_usage.total as i64,
                memory_usage.used as i64,
                timestamp_str,
            ))
            .expect("Failed to insert memory usage");
    }

    pub fn get_last_cpu_usage(&mut self) -> Option<CpuUsage> {
        self.get_last_cpu_stmt
            .query_row([], |row| {
                let usage: f32 = row.get(0)?;
                Ok(CpuUsage { usage })
            })
            .optional()
            .expect("Failed to get last CPU usage")
    }

    pub fn get_last_memory_usage(&mut self) -> Option<MemoryUsage> {
        self.get_last_memory_stmt
            .query_row([], |row| {
                let total: u64 = row.get(0)?;
                let used: u64 = row.get(1)?;
                Ok(MemoryUsage { total, used })
            })
            .optional()
            .expect("Failed to get last memory usage")
    }
}
