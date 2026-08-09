//! A synchronization fixture containing an unsafe bare wait pattern.

use std::sync::{Condvar, Mutex};

pub fn block_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
    let guard = state.lock().expect("lock");
    let _guard = condvar.wait(guard).expect("wait");
}
