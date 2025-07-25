use anyhow::Result;
use anyhow::anyhow;
use chrono::DateTime;
use chrono::TimeZone;
use chrono::Utc;
use rusqlite::Row;
use rusqlite::named_params;
use rusqlite::{Connection, OptionalExtension, Params, Statement};

use crate::types::CpuUsageDataPoint;
use crate::types::Interval;
use crate::types::MemoryUsageDataPoint;
use crate::types::{CpuUsage, MemoryUsage};

#[derive(Debug)]
pub struct DbManager<'conn> {
    connection: &'conn Connection,
    insert_usage_stmt: Statement<'conn>,
    get_last_cpu_container_stmt: Statement<'conn>,
    get_last_cpu_host_stmt: Statement<'conn>,
    get_last_memory_container_stmt: Statement<'conn>,
    get_last_memory_host_stmt: Statement<'conn>,
    get_history_cpu_host_stmt: Statement<'conn>,
    get_history_cpu_container_stmt: Statement<'conn>,
    get_history_memory_host_stmt: Statement<'conn>,
    get_history_memory_container_stmt: Statement<'conn>,
}

impl<'conn> DbManager<'conn> {
    pub fn new(connection: &'conn Connection) -> Result<Self> {
        connection.execute_batch(include_str!("./queries/init.sql"))?;

        Ok(Self {
            connection,
            insert_usage_stmt: connection.prepare(include_str!("./queries/insert_usage.sql"))?,
            get_last_cpu_container_stmt: connection
                .prepare(include_str!("./queries/container_cpu_last.sql"))?,
            get_last_memory_container_stmt: connection
                .prepare(include_str!("./queries/container_memory_last.sql"))?,
            get_last_cpu_host_stmt: connection
                .prepare(include_str!("./queries/host_cpu_last.sql"))?,
            get_last_memory_host_stmt: connection
                .prepare(include_str!("./queries/host_memory_last.sql"))?,
            get_history_cpu_host_stmt: connection
                .prepare(include_str!("./queries/host_cpu_history.sql"))?,
            get_history_cpu_container_stmt: connection
                .prepare(include_str!("./queries/container_cpu_history.sql"))?,
            get_history_memory_host_stmt: connection
                .prepare(include_str!("./queries/host_memory_history.sql"))?,
            get_history_memory_container_stmt: connection
                .prepare(include_str!("./queries/container_memory_history.sql"))?,
        })
    }

    pub fn insert_resource_usage(
        &mut self,
        timestamp: DateTime<Utc>,
        memory_usage: MemoryUsage,
        cpu_usage: CpuUsage,
        container: Option<String>,
    ) -> Result<()> {
        self.insert_usage_stmt.execute(named_params!(
            ":timestamp": timestamp,
            ":container": container,
            ":cpu_percentage": cpu_usage.percentage.round(),
            ":memory_total": memory_usage.total,
            ":memory_used": memory_usage.used,
            ":memory_percentage": memory_usage.percentage.round(),
        ))?;

        Ok(())
    }

    pub fn get_last_cpu_usage(
        &mut self,
        container: Option<String>,
    ) -> Result<Option<CpuUsageDataPoint>> {
        match container {
            Some(container) => Self::query_last_cpu_usage(
                &mut self.get_last_cpu_container_stmt,
                named_params! {":container": container},
            ),
            None => Self::query_last_cpu_usage(&mut self.get_last_cpu_host_stmt, []),
        }
    }

    pub fn get_last_memory_usage(
        &mut self,
        container: Option<String>,
    ) -> Result<Option<MemoryUsageDataPoint>> {
        match container {
            Some(container) => Self::query_last_memory_usage(
                &mut self.get_last_memory_container_stmt,
                named_params! {":container": container},
            ),
            None => Self::query_last_memory_usage(&mut self.get_last_memory_host_stmt, []),
        }
    }

    pub fn get_interval_cpu_usage(
        &mut self,
        interval: Interval,
        container: Option<String>,
    ) -> Result<Vec<CpuUsageDataPoint>> {
        self.query_interval(
            interval,
            container,
            |group_column, container_cond| {
                format!(
                    "SELECT AVG(cpu_percentage) as percentage, {group_column} as timestamp
                    FROM usage 
                    WHERE container {container_cond} AND timestamp BETWEEN :from AND :to
                    GROUP BY {group_column}
                    ORDER BY {group_column} ASC"
                )
            },
            |row| {
                Ok(CpuUsageDataPoint {
                    percentage: row.get(0)?,
                    timestamp: row.get(1)?,
                })
            },
        )
    }

    pub fn get_interval_memory_usage(
        &mut self,
        interval: Interval,
        container: Option<String>,
    ) -> Result<Vec<MemoryUsageDataPoint>> {
        self.query_interval(
            interval,
            container,
            |group_column, container_cond| {
                format!(
                    "SELECT AVG(memory_total) as total, AVG(memory_used) as used, AVG(memory_percentage) as percentage, {group_column} as timestamp
                    FROM usage 
                    WHERE container {container_cond} AND timestamp BETWEEN :from AND :to
                    GROUP BY {group_column}
                    ORDER BY {group_column} ASC"
                )
            },
            |row| {
                Ok(MemoryUsageDataPoint {
                    total: row.get::<_, f64>(0)? as u64,
                    used: row.get::<_, f64>(1)? as u64,
                    percentage: row.get(2)?,
                    timestamp: row.get(3)?,
                })
            }
        )
    }

    pub fn get_cpu_usage_history(
        &mut self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        container: Option<String>,
    ) -> Result<Vec<CpuUsageDataPoint>> {
        let from = from.unwrap_or(Utc.timestamp_opt(0, 0).unwrap());
        let to = to.unwrap_or(Utc::now());

        match container {
            Some(container) => Self::query_cpu_usages(
                &mut self.get_history_cpu_container_stmt,
                named_params! {":container": container, ":from": from, ":to": to},
            ),
            None => Self::query_cpu_usages(
                &mut self.get_history_cpu_host_stmt,
                named_params! {":from": from, ":to": to},
            ),
        }
    }

    pub fn get_memory_usage_history(
        &mut self,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        container: Option<String>,
    ) -> Result<Vec<MemoryUsageDataPoint>> {
        let from = from.unwrap_or(Utc.timestamp_opt(0, 0).unwrap());
        let to = to.unwrap_or(Utc::now());

        match container {
            Some(container) => Self::query_memory_usages(
                &mut self.get_history_memory_container_stmt,
                named_params! {":container": container, ":from": from, ":to": to},
            ),
            None => Self::query_memory_usages(
                &mut self.get_history_memory_host_stmt,
                named_params! {":from": from, ":to": to},
            ),
        }
    }

    fn query_last_memory_usage(
        stmt: &mut Statement,
        params: impl Params,
    ) -> Result<Option<MemoryUsageDataPoint>> {
        stmt.query_one(params, |row| {
            Ok(MemoryUsageDataPoint {
                total: row.get(0)?,
                used: row.get(1)?,
                percentage: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })
        .optional()
        .map_err(|e| anyhow!("Failed to get last memory usage: {e}"))
    }

    fn query_last_cpu_usage(
        stmt: &mut Statement,
        params: impl Params,
    ) -> Result<Option<CpuUsageDataPoint>> {
        stmt.query_one(params, |row| {
            Ok(CpuUsageDataPoint {
                percentage: row.get(0)?,
                timestamp: row.get(1)?,
            })
        })
        .optional()
        .map_err(|e| anyhow!("Failed to get last CPU usage: {e}"))
    }

    fn query_interval<T>(
        &self,
        interval: Interval,
        container: Option<String>,
        get_sql: impl FnOnce(&str, String) -> String,
        fun: impl FnMut(&Row) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>> {
        let group_column = interval.to_group_column_name();
        let container_cond = if let Some(container) = &container {
            format!("LIKE {container} || '%'")
        } else {
            "IS NULL".to_string()
        };

        let sql = get_sql(group_column, container_cond);
        let mut stmt = self
            .connection
            .prepare_cached(&sql)
            .map_err(|e| anyhow!("Failed to prepare statement: {e}"))?;

        let to = Utc::now();
        let from = to - interval.to_duration();

        stmt.query_map(named_params! {":from": from, ":to": to}, fun)
            .and_then(|result| result.collect())
            .map_err(|e| anyhow!("Failed to run query_map: {e}"))
    }

    fn query_memory_usages(
        stmt: &mut Statement,
        params: impl Params,
    ) -> Result<Vec<MemoryUsageDataPoint>> {
        stmt.query_map(params, |row| {
            Ok(MemoryUsageDataPoint {
                total: row.get(0)?,
                used: row.get(1)?,
                percentage: row.get(2)?,
                timestamp: row.get(3)?,
            })
        })
        .and_then(|result| result.collect())
        .map_err(|e| anyhow!("Failed to query memory usage: {e}"))
    }

    fn query_cpu_usages(
        stmt: &mut Statement,
        params: impl Params,
    ) -> Result<Vec<CpuUsageDataPoint>> {
        stmt.query_map(params, |row| {
            Ok(CpuUsageDataPoint {
                percentage: row.get(0)?,
                timestamp: row.get(1)?,
            })
        })
        .and_then(|result| result.collect())
        .map_err(|e| anyhow!("Failed to query CPU usage: {e}"))
    }
}
