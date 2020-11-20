extern crate timer;
extern crate chrono;

use timer::Timer;
use timer::Guard;
use mio_extras::channel as mio_channel;
use crate::{
  network::constant::{TimerMessageType},
};
use std::{collections::HashMap /*time::Instant*/};

/// Timed event handler can set countdown of chrono::Duration length. After timeout sends event via miochannel.
/// One TimedEventHandler can handle one timer of each TimerMessageType. Multiple timeuts of same TimerMessageType are not possible.
pub struct TimedEventHandler {
  timers: HashMap<TimerMessageType, Timer>,
  channel_send: mio_channel::SyncSender<TimerMessageType>,
  guards: HashMap<TimerMessageType, Option<Guard>>,
}

impl<'a> TimedEventHandler {
  pub fn new(channel_send: mio_channel::SyncSender<TimerMessageType>) -> TimedEventHandler {
    let hbh = TimedEventHandler {
      timers: HashMap::new(),
      channel_send,
      guards: HashMap::new(),
    };
    return hbh;
  }

  pub fn set_timeout(&mut self, duration: &'a chrono::Duration, timer_type: TimerMessageType) {
    if !self.timers.contains_key(&timer_type) {
      self.timers.insert(timer_type, Timer::new());
    }

    let new_chanenel = self.channel_send.clone();
    self.guards.insert(
      timer_type,
      Some(
        self
          .timers
          .get(&timer_type)
          .unwrap()
          .clone()
          .schedule_with_delay(duration.clone(), move || {
            new_chanenel
              .try_send(timer_type)
              .expect("Unable to send timeout message of type ");
          }),
      ),
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use mio_extras::channel as mio_channel;
  extern crate chrono;
  use chrono::Duration;
  use mio::{Token, Poll, Ready, PollOpt};
  use std::thread;

  #[test]
  fn test_heartbeat_timer() {
    let poll = Poll::new().expect("Unable to create new poll.");

    let (hearbeatSender, hearbeatReciever) = mio_channel::sync_channel::<TimerMessageType>(10);
    let (hearbeatSender2, hearbeatReciever2) = mio_channel::sync_channel::<TimerMessageType>(10);
    let mut hbh = TimedEventHandler::new(hearbeatSender);
    let mut hbh2 = TimedEventHandler::new(hearbeatSender2);
    poll
      .register(
        &hearbeatReciever,
        Token(123),
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("timer channel registeration failed!!");
    poll
      .register(
        &hearbeatReciever2,
        Token(124),
        Ready::readable(),
        PollOpt::edge(),
      )
      .expect("timer channel registeration failed!!");
    hbh.set_timeout(&Duration::seconds(1), TimerMessageType::writer_heartbeat);
    hbh2.set_timeout(
      &Duration::milliseconds(10),
      TimerMessageType::writer_heartbeat,
    );

    thread::sleep(std::time::Duration::from_secs(2));
  }
}
