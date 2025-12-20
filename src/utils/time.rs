use std::time;

pub fn unix_timestamp() -> Result<i64, time::SystemTimeError> {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
}