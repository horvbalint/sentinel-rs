use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
pub enum Interval {
    #[serde(rename = "5m")]
    Minute5,
    #[serde(rename = "1h")]
    Hour,
    #[serde(rename = "1d")]
    Day,
    #[serde(rename = "1w")]
    Week,
    #[serde(rename = "30d")]
    Day30,
}

impl Interval {
    pub fn to_group_column_name(&self) -> &'static str {
        match self {
            Interval::Minute5 => "timestamp_5m",
            Interval::Hour => "timestamp_1h",
            Interval::Day => "timestamp_1d",
            Interval::Week => "timestamp_1w",
            Interval::Day30 => "timestamp_30d",
        }
    }

    pub fn to_duration(&self) -> Duration {
        match self {
            Interval::Minute5 => Duration::minutes(5),
            Interval::Hour => Duration::hours(1),
            Interval::Day => Duration::days(1),
            Interval::Week => Duration::weeks(1),
            Interval::Day30 => Duration::days(30),
        }
    }
}
