use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MemoryUsage {
    pub total: u64,
    pub used: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct CpuUsage {
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct MemoryUsageDataPoint {
    pub timestamp: DateTime<Utc>,
    pub total: u64,
    pub used: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct CpuUsageDataPoint {
    pub timestamp: DateTime<Utc>,
    pub percentage: f64,
}
