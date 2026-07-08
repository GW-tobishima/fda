use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::ports::Clock;

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_seconds(&self) -> u64 {
        system_unix_seconds()
    }
}

pub(crate) fn system_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
