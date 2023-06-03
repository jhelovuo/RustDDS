use std::thread;

use mio_extras::channel::{SyncSender, TrySendError};

use crate::structure::duration::Duration;

const TIMEOUT_EPSILON_NS: i64 = 1000; // 1µs

// By default, give background thread 20 ms to react
pub const TIMEOUT_FALLBACK :Duration = Duration::from_nanos(20_000_000); // 20 ms

pub fn try_send_timeout<T>(
  sender: &SyncSender<T>,
  t: T,
  timeout_opt: Option<Duration>,
) -> Result<(), TrySendError<T>> {

  match sender.try_send(t) {
    Ok(()) => Ok(()), // This is expected to be the common case

    Err(TrySendError::Full(tt)) => {
      let mut mt = tt;
      let timeout = timeout_opt.unwrap_or(TIMEOUT_FALLBACK).to_nanoseconds();      
      let mut time_left = timeout;
      let mut delay = TIMEOUT_EPSILON_NS;
      while time_left > TIMEOUT_EPSILON_NS {
        match sender.try_send(mt) {
          Ok(()) => return Ok(()),
          Err(TrySendError::Full(tt)) => {
            thread::sleep(std::time::Duration::from_nanos(delay as u64)); // and try again
            mt = tt;
            time_left -= delay;
            delay *= 2;
          }
          Err(other) => return Err(other),
        }
      }
      Err(TrySendError::Full(mt))        
    }
    Err(other) => Err(other),
  }
}
