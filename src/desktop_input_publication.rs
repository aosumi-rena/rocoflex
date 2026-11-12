//! Nonblocking UI admission for one dedicated, non-audio MIDI producer.
use std::sync::{Mutex, MutexGuard, TryLockError};
#[derive(Default)]
pub(super) struct Publication { state: Mutex<bool> }
impl Publication {
    // The worker may briefly wait while the UI drains at most 128 messages.
    pub fn worker(&self) -> Result<MutexGuard<'_,bool>,()> { self.state.lock().map_err(|_|()) }
    // The UI never waits and must leave the channel untouched while busy.
    pub fn ui(&self) -> Result<Option<MutexGuard<'_,bool>>,()> {
        match self.state.try_lock() {
            Ok(guard)=>Ok(Some(guard)),
            Err(TryLockError::WouldBlock)=>Ok(None),
            Err(TryLockError::Poisoned(_))=>Err(()),
        }
    }
    #[allow(dead_code)]
    pub fn overflowed(&self) -> bool { self.state.lock().map(|state|*state).unwrap_or(true) }
}
