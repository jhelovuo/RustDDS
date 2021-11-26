use std::{
  io::{ Write},
  os::unix::io::AsRawFd,
};

#[allow(unused_imports)] 
use log::{debug, info, warn, error};

use mio::{Events, Poll, PollOpt, Ready, Token, unix::EventedFd};
use mio_extras::{channel as mio_channel, };
use termion::{event::Key, input::TermRead, raw::RawTerminal, AsyncReader};

use crate::{Twist,Vector3};

#[derive(Debug)]
pub enum RosCommand {
  StopEventLoop,
  TurtleCmdVel {
    twist: Twist,
  },
}


// Define turtle movement commands as Twist values
const MOVE_FORWARD : Twist = Twist 
  { linear: Vector3{ x: 2.0, .. Vector3::ZERO }
  , angular: Vector3::ZERO };

const MOVE_BACKWARD : Twist = Twist 
  { linear: Vector3{ x: -2.0, .. Vector3::ZERO }
  , angular: Vector3::ZERO };

const ROTATE_LEFT : Twist = Twist 
  { linear: Vector3::ZERO 
  , angular: Vector3{ z: 2.0, .. Vector3::ZERO } };

const ROTATE_RIGHT : Twist = Twist 
  { linear: Vector3::ZERO
  , angular: Vector3{ z: -2.0, .. Vector3::ZERO } };



pub struct MainController {
  poll: Poll,
  stdout: RawTerminal<std::io::Stdout>,
  async_reader: termion::input::Events<AsyncReader>,
  command_sender: mio_channel::SyncSender<RosCommand>,
  nodelist_receiver: mio_channel::Receiver<Twist>,
}

impl MainController {
  const KEYBOARD_CHECK_TOKEN: Token = Token(0);
  const UPDATE_NODE_LIST_TOKEN: Token = Token(1);

  pub fn new(
    stdout: RawTerminal<std::io::Stdout>,
    command_sender: mio_channel::SyncSender<RosCommand>,
    nodelist_receiver: mio_channel::Receiver<Twist>,
  ) -> MainController {
    let poll = Poll::new().unwrap();
    let async_reader = termion::async_stdin().events();


    MainController {
      poll,
      stdout,
      async_reader,
      command_sender,
      nodelist_receiver,
    }
  }

  pub fn start(&mut self) {

    self
      .poll
      .register(
        &EventedFd(&self.stdout.as_raw_fd()),
        // stdout seems a silly place to poll for input, but I
        // think the tty device is the same as for stdin.
        MainController::KEYBOARD_CHECK_TOKEN,
        Ready::readable(),
        PollOpt::level(),
      )
      .unwrap();

    self
      .poll
      .register(
        &self.nodelist_receiver,
        MainController::UPDATE_NODE_LIST_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // clearing screen
    write!(
      self.stdout,
      "{}{}",
      termion::clear::All,
      termion::cursor::Goto(1, 1)
    )
    .unwrap();
    self.stdout.flush().unwrap();

    loop {
      write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
      self.stdout.flush().unwrap();

      let mut events = Events::with_capacity(100);
      self.poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        if event.token() == MainController::KEYBOARD_CHECK_TOKEN {
          // a small wait here to allow the termion input mechnism to react.
          // Still some keyboard presses are missed. What are we doing wrong here?
          std::thread::sleep(std::time::Duration::from_millis(10));
          while let Some(Ok(kevent)) = &self.async_reader.next() {
            info!("key: {:?}",kevent);
            match kevent {
              termion::event::Event::Key(Key::Char('q')) => {
                debug!("Quit.");
                self.send_command(RosCommand::StopEventLoop);
                return
              }
              termion::event::Event::Key(Key::Up) => {
                debug!("Move left.");
                let twist = MOVE_FORWARD;
                self.print_sent_turtle_cmd_vel(&twist);
                self.send_command(RosCommand::TurtleCmdVel { twist })
              }
              termion::event::Event::Key(Key::Right) => {
                debug!("Move right.");
                let twist = ROTATE_RIGHT;
                self.print_sent_turtle_cmd_vel(&twist);
                self.send_command(RosCommand::TurtleCmdVel { twist })
              }
              termion::event::Event::Key(Key::Down) => {
                debug!("Rotate down.");
                let twist = MOVE_BACKWARD;
                self.print_sent_turtle_cmd_vel(&twist);
                self.send_command(RosCommand::TurtleCmdVel { twist })
              }
              termion::event::Event::Key(Key::Left) => {
                debug!("Rotate left.");
                let twist = ROTATE_LEFT;
                self.print_sent_turtle_cmd_vel(&twist);
                self.send_command(RosCommand::TurtleCmdVel { twist })
              }
              termion::event::Event::Key(key) => {
                write!(
                  self.stdout,
                  "{}{}{:?} : Press q to quit, cursor keys to control turtle.",
                  termion::cursor::Goto(1, 1),
                  termion::clear::CurrentLine,
                  key,
                )
                .unwrap();
              }
              _ => (),
            }
          }

        } else if event.token() == MainController::UPDATE_NODE_LIST_TOKEN {
          while let Ok(twist) = self.nodelist_receiver.try_recv() {
              self.print_turtle_cmd_vel(&twist);
          }
        } else {
          error!("What is this? {:?}", event.token())
        }
      } 
    }
  }

  fn send_command(&self, command: RosCommand) {
    self.command_sender.try_send(command)
      .unwrap_or_else(|e| error!("UI: Failed to send command {:?}", e) )
  }


  fn print_turtle_cmd_vel(&mut self, twist: &Twist) {
    write!(
      self.stdout,
      "{}{}Read Turtle cmd_vel {:?}",
      termion::cursor::Goto(1, 4),
      termion::clear::CurrentLine,
      twist
    )
    .unwrap();
  }

  fn print_sent_turtle_cmd_vel(&mut self, twist: &Twist) {
    write!(
      self.stdout,
      "{}{}Sent Turtle cmd_vel {:?}",
      termion::cursor::Goto(1, 2),
      termion::clear::CurrentLine,
      twist
    )
    .unwrap();
  }
}
