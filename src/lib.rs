#![feature(ip)]

// NOTE: this exists only so `/bin/*.rs` files can access the same modules

#[macro_use]
pub mod macros;

pub mod bar_items;
pub mod cli;
pub mod config;
pub mod context;
pub mod dbus;
pub mod dispatcher;
pub mod error;
pub mod human_time;
pub mod i3;
pub mod ipc;
pub mod signals;
pub mod theme;
pub mod util;

#[cfg(test)]
pub mod test_utils;
