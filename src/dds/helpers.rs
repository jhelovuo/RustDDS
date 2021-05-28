use mio_extras::channel::{ SyncSender, TrySendError};
use crate::structure::duration::Duration;
use std::thread;
use std::convert::From;


const timeout_epsilon : Duration = Duration::from_nanos( 1000 );

// Always give background thread 0.1 ms to react
const timeout_fallback : Duration = Duration::from_nanos( 100_000 );

pub fn try_send_timeout<T>(sender: &SyncSender<T>, t: T, timeout_opt:Option<Duration>) 
  -> Result<(), TrySendError<T>> 
{
  // TODO: Write a more optimized fast path, where send succeeds on first try.

  let timeout = timeout_opt.unwrap_or(timeout_fallback);
  let mut delays = Vec::with_capacity(20);
  if timeout <= timeout_epsilon {
    delays.push(timeout_epsilon)
  } else {
    let mut to = timeout;
    while to > timeout_epsilon {
      to = to / 2;
      delays.push(to);
    }
  }
  let mut mt = t;
  while let Some(delay) = delays.pop() {
    match sender.try_send(mt) {
        Ok(()) => return Ok(()),
        Err(TrySendError::Full(tt)) => {
          thread::sleep(std::time::Duration::from(delay)); // and try again
          mt = tt;
        }
        Err(other) => return Err(other),
    }
  }
  Err(TrySendError::Full(mt))
}