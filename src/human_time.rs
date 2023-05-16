use std::time::Duration;

pub use humantime_serde::serialize;
use humantime_serde::Serde;
use serde::{Deserialize, Deserializer};

/// We use `humantime_serde` for intervals defined in the configuration file, but we want to disallow
// any interval that's too low. So we hook into it here to override any intervals.
pub fn deserialize<'a, D>(d: D) -> Result<Duration, D::Error>
where
    Serde<Duration>: Deserialize<'a>,
    D: Deserializer<'a>,
{
    let duration = Serde::deserialize(d).map(Serde::into_inner)?;
    if duration.as_secs() == 0 {
        log::warn!(
            "invalid duration {}, interval must be >= 1: defaulting to 1s",
            format!("{:?}", duration)
        );
        Ok(Duration::from_secs(1))
    } else {
        Ok(duration)
    }
}
