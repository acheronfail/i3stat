//! We use `humantime_serde` for intervals defined in the configuration file, but we want to disallow
//! any interval that's too low. So we hook into it here to override any intervals.

use std::time::Duration;

pub use humantime_serde::option::serialize;
use humantime_serde::Serde;
use serde::{Deserialize, Deserializer};

use super::validate;

pub fn deserialize<'a, D>(d: D) -> Result<Option<Duration>, D::Error>
where
    Serde<Duration>: Deserialize<'a>,
    D: Deserializer<'a>,
{
    let got: Option<Serde<Duration>> = Deserialize::deserialize(d)?;
    Ok(got.map(|d| validate(Serde::into_inner(d))))
}
