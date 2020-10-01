use log::error;
use atosdds::structure::guid::GUID;
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;
use termion::{event::Key, raw::RawTerminal, input::TermRead, AsyncReader};

use std::{collections::HashMap, io::StdoutLock, io::Write, time::Duration as StdDuration};

use crate::{
  ros2::turtle_control::TurtleControl,
  ros2::turtle_data::Twist,
  ros_data::{Gid, NodeInfo, ROSParticipantInfo},
};

pub enum RosCommand {
  StopEventLoop,
  UpdateNode,
  AddNodeListSender {
    sender: mio_channel::SyncSender<DataUpdate>,
  },
  TurtleCmdVel {
    twist: Twist,
  },
}

pub enum DataUpdate {
  UpdateNode { info: ROSParticipantInfo },
  DeleteNode { guid: GUID },
  TurtleCmdVel { twist: Twist },
}

pub struct MainController<'a> {
  poll: Poll,
  stdout: RawTerminal<StdoutLock<'a>>,
  input_timer: Timer<()>,
  node_timer: Timer<()>,
  async_reader: termion::input::Events<AsyncReader>,
  command_sender: mio_channel::SyncSender<RosCommand>,
  nodelist_receiver: mio_channel::Receiver<DataUpdate>,
}

impl<'a> MainController<'a> {
  const KEYBOARD_CHECK_TIMEOUT: u64 = 50;
  const NODE_UPDATE_TIMEOUT: u64 = 1;

  const KEYBOARD_CHECK_TOKEN: Token = Token(0);
  const NODE_TIMER_TOKEN: Token = Token(1);
  const UPDATE_NODE_LIST_TOKEN: Token = Token(2);

  pub fn new(
    stdout: RawTerminal<StdoutLock<'a>>,
    command_sender: mio_channel::SyncSender<RosCommand>,
  ) -> MainController<'a> {
    let poll = Poll::new().unwrap();
    let input_timer = Timer::default();
    let node_timer = Timer::default();
    let async_reader = termion::async_stdin().events();

    let (nodelist_sender, nodelist_receiver) = mio_channel::sync_channel(100);
    command_sender
      .send(RosCommand::AddNodeListSender {
        sender: nodelist_sender,
      })
      .unwrap();

    MainController {
      poll,
      stdout,
      input_timer,
      node_timer,
      async_reader,
      command_sender,
      nodelist_receiver,
    }
  }

  pub fn start(&mut self) {
    self.init_main_registers();

    write!(
      self.stdout,
      "{}{}Nodelist",
      termion::cursor::Goto(1, 20),
      termion::clear::AfterCursor
    )
    .unwrap();
    self.stdout.flush().unwrap();

    let mut node_list: HashMap<Gid, Vec<NodeInfo>> = HashMap::new();

    loop {
      write!(self.stdout, "{}", termion::cursor::Goto(1, 1)).unwrap();
      self.stdout.flush().unwrap();

      let mut events = Events::with_capacity(100);
      self.poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        if event.token() == MainController::KEYBOARD_CHECK_TOKEN {
          while let Some(kevent) = &self.async_reader.next() {
            match kevent.as_ref().unwrap() {
              termion::event::Event::Key(Key::Char('q')) => {
                match self.command_sender.send(RosCommand::StopEventLoop) {
                  Ok(_) => return,
                  _ => (),
                }
              }
              termion::event::Event::Key(Key::Up) => {
                let twist = TurtleControl::move_forward();
                self.print_sent_turtle_cmd_vel(&twist);
                self
                  .command_sender
                  .send(RosCommand::TurtleCmdVel { twist })
                  .unwrap_or(());
              }
              termion::event::Event::Key(Key::Right) => {
                let twist = TurtleControl::rotate_right();
                self.print_sent_turtle_cmd_vel(&twist);
                self
                  .command_sender
                  .send(RosCommand::TurtleCmdVel { twist })
                  .unwrap_or(());
              }
              termion::event::Event::Key(Key::Down) => {
                let twist = TurtleControl::move_backward();
                self.print_sent_turtle_cmd_vel(&twist);
                self
                  .command_sender
                  .send(RosCommand::TurtleCmdVel { twist })
                  .unwrap_or(());
              }
              termion::event::Event::Key(Key::Left) => {
                let twist = TurtleControl::rotate_left();
                self.print_sent_turtle_cmd_vel(&twist);
                self
                  .command_sender
                  .send(RosCommand::TurtleCmdVel { twist })
                  .unwrap_or(());
              }
              termion::event::Event::Key(key) => {
                write!(
                  self.stdout,
                  "{}{}{:?}",
                  termion::cursor::Goto(1, 1),
                  termion::clear::CurrentLine,
                  key
                )
                .unwrap();
              }
              _ => (),
            }
          }

          self.input_timer.set_timeout(
            StdDuration::from_millis(MainController::KEYBOARD_CHECK_TIMEOUT),
            (),
          );
        } else if event.token() == MainController::NODE_TIMER_TOKEN {
          match self.command_sender.send(RosCommand::UpdateNode) {
            Ok(_) => (),
            Err(e) => {
              error!("Failed to send UPDATE_NODE command {:?}", e);
              return;
            }
          }
        // self.node_timer.set_timeout(
        //   StdDuration::from_secs(MainController::NODE_UPDATE_TIMEOUT),
        //   (),
        // );
        } else if event.token() == MainController::UPDATE_NODE_LIST_TOKEN {
          while let Ok(rec_nodes) = self.nodelist_receiver.try_recv() {
            match rec_nodes {
              DataUpdate::UpdateNode { info } => {
                let nodes: Vec<NodeInfo> = info.nodes().iter().map(|p| p.clone()).collect();

                write!(
                  self.stdout,
                  "{}{}{}Nodes: {:?}",
                  termion::cursor::Goto(1, 2),
                  [' '; 39].to_vec().into_iter().collect::<String>(),
                  termion::cursor::Goto(1, 2),
                  nodes.len()
                )
                .unwrap();

                node_list.insert(info.guid(), nodes);
              }
              DataUpdate::DeleteNode { guid } => {
                node_list.remove(&Gid::from_guid(guid));
              }
              DataUpdate::TurtleCmdVel { twist } => {
                self.print_turtle_cmd_vel(&twist);
              }
            }
          }

          write!(
            self.stdout,
            "{}{}Nodelist: {}",
            termion::cursor::Goto(1, 20),
            termion::clear::AfterCursor,
            node_list.iter().flat_map(|(_, nd)| nd.iter()).count(),
          )
          .unwrap();
          for (i, node_info) in node_list.iter().flat_map(|(_, nd)| nd.iter()).enumerate() {
            write!(
              self.stdout,
              "{}{}{}",
              termion::cursor::Goto(1, 21 + i as u16),
              node_info.node_namespace,
              node_info.node_name
            )
            .unwrap();
          }

          self.stdout.flush().unwrap();
        }
      }
    }
  }

  fn init_main_registers(&mut self) {
    self.input_timer.set_timeout(
      StdDuration::from_millis(MainController::KEYBOARD_CHECK_TIMEOUT),
      (),
    );
    self.node_timer.set_timeout(
      StdDuration::from_secs(MainController::NODE_UPDATE_TIMEOUT),
      (),
    );

    self
      .poll
      .register(
        &self.input_timer,
        MainController::KEYBOARD_CHECK_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    self
      .poll
      .register(
        &self.node_timer,
        MainController::NODE_TIMER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
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
  }

  fn print_turtle_cmd_vel(&mut self, twist: &Twist) {
    write!(
      self.stdout,
      "{}{}{}",
      termion::cursor::Goto(40, 1),
      termion::clear::CurrentLine,
      "Turtle cmd_vel"
    )
    .unwrap();
    write!(
      self.stdout,
      "{}{}{:?}",
      termion::cursor::Goto(40, 2),
      termion::clear::CurrentLine,
      twist
    )
    .unwrap();
  }

  fn print_sent_turtle_cmd_vel(&mut self, twist: &Twist) {
    write!(
      self.stdout,
      "{}{}{}",
      termion::cursor::Goto(40, 3),
      termion::clear::CurrentLine,
      "Sent Turtle cmd_vel"
    )
    .unwrap();
    write!(
      self.stdout,
      "{}{}{:?}",
      termion::cursor::Goto(40, 4),
      termion::clear::CurrentLine,
      twist
    )
    .unwrap();
  }
}
