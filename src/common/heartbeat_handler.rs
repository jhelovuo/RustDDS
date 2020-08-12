use std::{thread, marker::PhantomData};
extern crate timer;
extern crate chrono;

use timer::Timer;
use timer::Guard;
use chrono::Duration;
use mio::{Poll, Token, Ready, PollOpt};
use mio_extras::channel as mio_channel;


pub struct HeartbeatHandler   {
  timer : Timer,
  channel_send : mio_channel::Sender<Token>,
  token : Token,
  guard :Option<Guard>,
}

impl <'a>  HeartbeatHandler {
  pub fn new(channel_send : mio_channel::Sender<Token>, token: Token) ->  HeartbeatHandler{
    let hbh = HeartbeatHandler{
      timer : Timer::new(),
      channel_send,
      token,
      guard : None
    };
    
    return hbh;
  }
 
  pub fn set_timeout (&mut self, duration : &'a chrono::Duration ) {
    println!("Set timeout running token: {:?}", &self.token);
    let new_chanenel = self.channel_send.clone();
    let new_token = self.token.clone();
    self.guard = Some(self.timer.schedule_with_delay(duration.clone() , move || {
      println!("timeout ended, token: {:?}", new_token);
      new_chanenel.send(new_token);
    }));
  }

}


#[cfg(test)]
mod tests {
  use super::*;
  use mio_extras::channel as mio_channel;
  extern crate chrono;
  use chrono::Duration;
  use mio::Token;

  #[test]
  fn test_heartbeat_timer() {
    
    let poll = Poll::new().expect("Unable to create new poll.");

    let (hearbeatSender, hearbeatReciever) = mio_channel::channel::<Token>();
    let (hearbeatSender2, hearbeatReciever2) = mio_channel::channel::<Token>();
    let mut hbh = HeartbeatHandler::new(hearbeatSender,  Token(123));
    let mut hbh2 = HeartbeatHandler::new(hearbeatSender2,  Token(124));
    poll.register(&hearbeatReciever, Token(123), Ready::readable(), PollOpt::edge()).expect("timer channel registeration failed!!");
    poll.register(&hearbeatReciever2, Token(124), Ready::readable(), PollOpt::edge()).expect("timer channel registeration failed!!");
    hbh.set_timeout(&Duration::seconds(1));
    hbh2.set_timeout(&Duration::milliseconds(10));

    thread::sleep(time::Duration::milliseconds(1000 * 2).to_std().unwrap());
    

  } 
}

  


