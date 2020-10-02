use log::error;
use atosdds::structure::guid::GUID;
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::timer::Timer;
use mio_extras::channel as mio_channel;
use termion::{event::Key, raw::RawTerminal, input::TermRead, AsyncReader};
use atosdds::DiscoveredTopicData;

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
  TopicList { list: Vec<DiscoveredTopicData> },
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
    let mut topic_list: Vec<DiscoveredTopicData> = Vec::new();

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
              DataUpdate::UpdateNode { mut info } => {
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
              DataUpdate::TopicList { list } => {
                write!(
                  self.stdout,
                  "{}{}{}Topics: {:?}",
                  termion::cursor::Goto(1, 3),
                  [' '; 39].to_vec().into_iter().collect::<String>(),
                  termion::cursor::Goto(1, 3),
                  list.len()
                )
                .unwrap();

                topic_list = list;
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

          let node_amount = node_list.iter().flat_map(|(_, nd)| nd.iter()).count();
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

          let topic_start = 21 + node_amount + 2;

          let (topics, services): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) =
            topic_list
              .iter()
              .partition(|p| p.get_topic_name().starts_with("rt"));
          let (services_request, services): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) =
            services
              .iter()
              .partition(|p| p.get_topic_name().starts_with("rq"));
          let (services_reply, dds_topics): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) = services.iter().partition(|p| p.get_topic_name().starts_with("rr"));

          write!(
            self.stdout,
            "{}Topics: {}",
            termion::cursor::Goto(1, topic_start as u16),
            topics.len(),
          )
          .unwrap();

          let mut max_width = 9;
          for (i, topic_info) in topics.iter().enumerate() {
            let ft = format!(
              "{} - {}",
              topic_info.get_topic_name(),
              topic_info.get_type_name()
            );
            max_width = if ft.len() > max_width {
              ft.len()
            } else {
              max_width
            };
            write!(
              self.stdout,
              "{}{}",
              termion::cursor::Goto(1, (topic_start + i + 1) as u16),
              ft
            )
            .unwrap();
          }

          write!(
            self.stdout,
            "{}Services Request: {}",
            termion::cursor::Goto(max_width as u16 + 2, topic_start as u16),
            services_request.len(),
          )
          .unwrap();
          for (i, service_info) in services_request.iter().enumerate() {
            let ft = format!(
              "{} - {}",
              service_info.get_topic_name(),
              service_info.get_type_name()
            );
            write!(
              self.stdout,
              "{}{}",
              termion::cursor::Goto(max_width as u16 + 2, (topic_start + i + 1) as u16),
              ft
            )
            .unwrap();
          }

          let dds_topic_start = topic_start
          + (if services_request.len() > topics.len() {
            services_request.len()
          } else {
            topics.len()
          }) + 2;
          write!(
            self.stdout,
            "{}Services Reply: {}",
            termion::cursor::Goto(
              1,
              dds_topic_start as u16
            ),
            services_reply.len(),
          )
          .unwrap();

          max_width = 20;
          for (i, reply_info) in services_reply.iter().enumerate() {
            let ft = format!(
              "{} - {}",
              reply_info.get_topic_name(),
              reply_info.get_type_name()
            );
            max_width = if ft.len() > max_width {
              ft.len()
            } else {
              max_width
            };
            write!(
              self.stdout,
              "{}{}",
              termion::cursor::Goto(1, (dds_topic_start + i + 1) as u16),
              ft
            )
            .unwrap();
          }

          write!(
            self.stdout,
            "{}DDS Topics: {}",
            termion::cursor::Goto(
              max_width as u16 + 2,
              dds_topic_start as u16
            ),
            dds_topics.len(),
          )
          .unwrap();

          for (i, dds_info) in dds_topics.iter().enumerate() {
            let ft = format!(
              "{} - {}",
              dds_info.get_topic_name(),
              dds_info.get_type_name()
            );
            write!(
              self.stdout,
              "{}{}",
              termion::cursor::Goto(max_width as u16 + 2, (dds_topic_start + i + 1) as u16),
              ft
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
