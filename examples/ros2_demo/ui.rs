use std::{
  collections::HashMap,
  io::{StdoutLock, Write},
  time::Duration as StdDuration,
  os::unix::io::AsRawFd,
};

#[allow(unused_imports)] 
use log::{debug, info, warn, error};

use mio::{Events, Poll, PollOpt, Ready, Token, unix::EventedFd};
use mio_extras::{channel as mio_channel, timer::Timer};
use termion::{event::Key, input::TermRead, raw::RawTerminal, AsyncReader};
use rustdds::{
  dds::data_types::DiscoveredTopicData,
  ros2::builtin_datatypes::{Gid, NodeInfo, ROSParticipantInfo},
};

use crate::{Twist,Vector3};

#[derive(Debug)]
pub enum RosCommand {
  StopEventLoop,
  TurtleCmdVel {
    twist: Twist,
  },
}

pub enum DataUpdate {
  UpdateNode { info: ROSParticipantInfo },
  TurtleCmdVel { twist: Twist },
  TopicList { list: Vec<DiscoveredTopicData> },
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
  nodelist_receiver: mio_channel::Receiver<DataUpdate>,
}

impl MainController {
  const NODE_UPDATE_TIMEOUT: u64 = 1;

  const KEYBOARD_CHECK_TOKEN: Token = Token(0);
  const UPDATE_NODE_LIST_TOKEN: Token = Token(1);

  pub fn new(
    stdout: RawTerminal<std::io::Stdout>,
    command_sender: mio_channel::SyncSender<RosCommand>,
    nodelist_receiver: mio_channel::Receiver<DataUpdate>,
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
                debug!("Move down.");
                let twist = MOVE_BACKWARD;
                self.print_sent_turtle_cmd_vel(&twist);
                self.send_command(RosCommand::TurtleCmdVel { twist })
              }
              termion::event::Event::Key(Key::Left) => {
                debug!("Move left.");
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

          // self.input_timer.set_timeout(
          //   StdDuration::from_millis(MainController::KEYBOARD_CHECK_TIMEOUT),
          //   (),
          // );
        } else if event.token() == MainController::UPDATE_NODE_LIST_TOKEN {
          while let Ok(rec_nodes) = self.nodelist_receiver.try_recv() {
           
            match rec_nodes {
              DataUpdate::UpdateNode { info } => {
                /*let nodes: Vec<NodeInfo> = info.nodes().to_vec();

                write!(
                  self.stdout,
                  "{}{}{}Nodes: {:?}",
                  termion::cursor::Goto(1, 2),
                  [' '; 39].to_vec().into_iter().collect::<String>(),
                  termion::cursor::Goto(1, 2),
                  nodes.len()
                )
                .unwrap();

                node_list.insert(info.guid(), nodes);*/
              }
              DataUpdate::TopicList { list } => {
               /* write!(
                  self.stdout,
                  "{}{}{}Topics: {:?}",
                  termion::cursor::Goto(1, 3),
                  [' '; 39].to_vec().into_iter().collect::<String>(),
                  termion::cursor::Goto(1, 3),
                  list.len()
                )
                .unwrap();

                topic_list = list;*/
              } 
              DataUpdate::TurtleCmdVel { twist } => {
                self.print_turtle_cmd_vel(&twist);
              }
            }
          }

          node_list.retain(|_, p| !p.is_empty());

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
              node_info.namespace(),
              node_info.name()
            )
            .unwrap();
          }

          let topic_start = 21 + node_amount + 2;

          let (topics, services): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) =
            topic_list
              .iter()
              .partition(|p| p.topic_name().starts_with("rt"));
          let (services_request, services): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) =
            services
              .iter()
              .partition(|p| p.topic_name().starts_with("rq"));
          let (services_reply, dds_topics): (Vec<&DiscoveredTopicData>, Vec<&DiscoveredTopicData>) =
            services
              .iter()
              .partition(|p| p.topic_name().starts_with("rr"));

          write!(
            self.stdout,
            "{}Topics: {}",
            termion::cursor::Goto(1, topic_start as u16),
            topics.len(),
          )
          .unwrap();

          let mut max_width = 9;
          for (i, topic_info) in topics.iter().enumerate() {
            let ft = format!("{} - {}", topic_info.topic_name(), topic_info.type_name());
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
              service_info.topic_name(),
              service_info.type_name()
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
            })
            + 2;
          write!(
            self.stdout,
            "{}Services Reply: {}",
            termion::cursor::Goto(1, dds_topic_start as u16),
            services_reply.len(),
          )
          .unwrap();

          max_width = 20;
          for (i, reply_info) in services_reply.iter().enumerate() {
            let ft = format!("{} - {}", reply_info.topic_name(), reply_info.type_name());
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
            termion::cursor::Goto(max_width as u16 + 2, dds_topic_start as u16),
            dds_topics.len(),
          )
          .unwrap();

          for (i, dds_info) in dds_topics.iter().enumerate() {
            let ft = format!("{} - {}", dds_info.topic_name(), dds_info.type_name());
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
