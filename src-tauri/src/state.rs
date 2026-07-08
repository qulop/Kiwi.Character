//! Application state: the SQLite-backed database handle plus small helpers.
//!
//! The DB connection lives behind a `Mutex`. Never hold that guard across an
//! `.await` — snapshot what you need, drop the guard, await, then re-lock.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::Db;

pub struct AppState {
    pub db: Mutex<Db>,
}

/// Current time in milliseconds since the Unix epoch.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A fresh random id.
pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}
