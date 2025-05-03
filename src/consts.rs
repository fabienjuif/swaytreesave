use std::time::Duration;

pub const MAX_WAIT_DURATION: Duration = Duration::from_secs(5);

pub fn default_timeout() -> Option<Duration> {
    Some(MAX_WAIT_DURATION)
}
