use std::time;
use chrono::{DateTime, Local, TimeZone, Utc};

pub fn unix_timestamp() -> Result<i64, time::SystemTimeError> {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
}

/// Format Unix timestamp as human-readable string. Format: "Mon Dec 21 14:30:45 2024 +0000"
pub fn format_timestamp(timestamp: i64) -> String {
    let datetime = Utc.timestamp_opt(timestamp, 0);
    
    match datetime.single() {
        Some(dt) => {
            let local: DateTime<Local> = dt.into();
            local.format("%a %b %e %H:%M:%S %Y %z").to_string()
        }
        None => {
            format!("Invalid timestamp: {}", timestamp)
        }
    }
}