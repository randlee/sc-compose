//! A synchronization fixture that inspects the timeout result.

use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub fn wait_until_ready(condvar: &Condvar, state: &Mutex<bool>) {
    let guard = state.lock().expect("lock");
    let (_guard, wait) = condvar
        .wait_timeout(guard, Duration::from_secs(1))
        .expect("wait");
    if wait.timed_out() {
        return;
    }
}
