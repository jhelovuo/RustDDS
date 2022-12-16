#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

use std::io;
use std::sync::Arc;

use mio_08::{Poll, Token, Events, Waker,};
use mio_misc::scheduler::
  {NotificationScheduler, ScheduleEntry, ScheduleEntryId, SchedulerStatus, Scheduler};
use mio_misc::{NotificationId, queue::NotificationQueue, channel, channel::SyncSender, queue::Notifier};
use std::sync::mpsc::Receiver;


// Provide polling mechanism for RustDDS internal use
//
// This functionality used to be supported by mio-0.6.x, but it
// is outdated. However, newer mio-0.8 does not support polling for
// channels or timers, as everythnig must be backed by a file handle.
//
// One alternative would be to make whole RustDDS async, but that would
// bind us to a specific runtime. Or if it would not, I do not know how to
// do that.
//
// The solution is to use mio-0.8 and mio-misc crates to provide
// channels and timers in addition to sockets to poll.
// This module clean up the interface slightly, so that it is simpler to use.

pub struct MyPoll {
  poll: Poll,
  events: Events,
  notify_queue: Arc<NotificationQueue>, // waker calls cause Notifications to be queued here
  scheduler: Arc<NotificationScheduler>, // This is "reactor" for timers.
  waker_token: Token,
}

impl MyPoll {
  pub fn with_capacity(capacity: usize, waker_token:Token) -> io::Result<MyPoll> {
    let poll = Poll::new()?;
    let waker = Arc::new(Waker::new(poll.registry(), waker_token)?);
    let notify_queue = Arc::new(NotificationQueue::new(waker)); 
    Ok(MyPoll {
        poll,
        events: Events::with_capacity(capacity),
        notify_queue: Arc::clone(&notify_queue),
        scheduler: Arc::new(NotificationScheduler::new(
          notify_queue,
          Arc::new(Scheduler::default()),
        )),
        waker_token,
      }
    )
  }

  pub fn new_sync_channel<T>(&self, capacity:usize, notification_token: Token) -> (SyncSender<T>, Receiver<T>) {
    if notification_token == self.waker_token {
      panic!("new_sync_channel: {:?} overlaps with poll-wide waker Token.", notification_token);
    }

    let notify_id =  token_to_notification_id(notification_token);
    let q = Arc::clone(&self.notify_queue); // Why cannot we just do this directly as function argument below?? (rustc 1.65.0)
    // channel::sync_channel(Arc::clone(&self.notify_queue), notify_id, capacity, )
    // but this does not compile?
    channel::sync_channel(q, notify_id, capacity, )
  }

  pub fn poll<I>(&mut self, timeout: I) -> io::Result<()>
  where
    I: Into<Option<std::time::Duration>>,
  {
    self.poll.poll(&mut self.events, timeout.into())
  }

  pub fn tokens_iter(&self) -> impl Iterator + '_ {
    core::iter::from_fn( || self.notify_queue.pop() )
      .map( notification_id_to_token )
      .chain( self.events.iter().map( move |e| e.token() ) )
  }

  pub fn new_timer(&self, token: Token) -> Timer {
    if token == self.waker_token {
      panic!("new_timer: {:?} overlaps with poll-wide waker Token.", token);
    }

    Timer { 
      scheduler: Arc::clone(&self.scheduler),
      token, 
    }
  }

}

pub fn notification_id_to_token(n:NotificationId) -> Token {
  Token( n.id() as usize )
}

pub fn token_to_notification_id(t:Token) -> NotificationId {
  NotificationId::from( t.0 as u32 )
}


pub struct Timer {
  scheduler: Arc<NotificationScheduler>,
  token: Token,
}

impl Timer {
  pub fn notify_with_fixed_interval(&self, interval: std::time::Duration) -> ScheduleEntryId {
    let i = token_to_notification_id(self.token);
    self.scheduler.notify_with_fixed_interval(i, interval, Some(interval), None )
  }

  pub fn notify_once_after_delay(&self, interval: std::time::Duration) -> ScheduleEntryId {
    let i = token_to_notification_id(self.token);
    self.scheduler.notify_once_after_delay(i, interval, None )
  }
}



