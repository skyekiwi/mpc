use std::time::{SystemTime, UNIX_EPOCH};
use crate::types::Timestamp;

pub fn get_time(time: Timestamp) -> Timestamp {
    if time == 0 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            / 30
    } else {
        time
    }
}
