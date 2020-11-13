extern crate atosdds;
extern crate mio;
extern crate mio_extras;

use std::{io::Write, time::Duration};

use atosdds::{
  dds::DomainParticipant,
  ros2::{RosContext, RosParticipant},
};
use commands::ThreadControl;
use log::{debug, error};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::{channel as mio_channel};
use structures::{MainController, DataUpdate, RosCommand};
use termion::raw::IntoRawMode;
use log4rs;
use turtle_listener::TurtleListener;
use turtle_sender::TurtleSender;

// modules
mod commands;
mod ros2;
mod structures;
mod turtle_listener;
mod turtle_sender;

const ROS2_COMMAND_TOKEN: Token = Token(1000);
const ROS2_NODE_RECEIVED_TOKEN: Token = Token(1001);
const TURTLE_CMD_VEL_RECEIVER_TOKEN: Token = Token(1002);
const TOPIC_UPDATE_TIMER_TOKEN: Token = Token(1003);
const NODE_INFO_TOKEN: Token = Token(1004);

fn main() {
  log4rs::init_file("examples/ros2_demo/log4rs.yaml", Default::default()).unwrap();

  let (command_sender, command_receiver) = mio_channel::sync_channel::<RosCommand>(10);

  let stdout_org = std::io::stdout();
  let mut stdout = stdout_org.lock().into_raw_mode().unwrap();
  // clearing screen
  write!(
    stdout,
    "{}{}",
    termion::clear::All,
    termion::cursor::Goto(1, 1)
  )
  .unwrap();
  stdout.flush().unwrap();

  let jhandle = std::thread::spawn(move || ros2_loop(command_receiver));

  let mut main_control = MainController::new(stdout, command_sender.clone());
  main_control.start();

  jhandle.join().unwrap();

  // need to wait a bit for cleanup, beacuse drop is not waited for join
  std::thread::sleep(Duration::from_millis(10));
}

fn ros2_loop(command_receiver: mio_channel::Receiver<RosCommand>) {
  let domain_participant = DomainParticipant::new(0);
  let ros_context = RosContext::new(domain_participant.clone(), true).unwrap();
  let mut ros_participant = RosParticipant::new(&ros_context).unwrap();

  // node info channel
  let (nodeinfo_sender, nodeinfo_receiver) = mio_channel::channel();

  let dpc = domain_participant.clone();
  let ni_sender = nodeinfo_sender.clone();
  // turtle listener
  let (tlisterner_sender, tlistener_receiver) = mio_channel::channel();
  let (tc_tl_sender, tc_ts_receiver) = mio_channel::channel();
  std::thread::spawn(move || {
    TurtleListener::run(dpc, tc_ts_receiver, tlisterner_sender, ni_sender)
  });

  let dpc = domain_participant.clone();
  // turtle writer
  let (tsender_sender, tsender_receiver) = mio_channel::channel();
  let (tc_ts_sender, tc_ts_receiver) = mio_channel::channel();
  std::thread::spawn(move || {
    TurtleSender::run(dpc, tc_ts_receiver, tsender_receiver, nodeinfo_sender)
  });

  {
    // topic update timer (or any update)
    let mut update_timer = mio_extras::timer::Timer::default();
    update_timer.set_timeout(Duration::from_secs(1), ());

    let poll = Poll::new().unwrap();

    poll
      .register(
        &command_receiver,
        ROS2_COMMAND_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &ros_participant,
        ROS2_NODE_RECEIVED_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &update_timer,
        TOPIC_UPDATE_TIMER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &nodeinfo_receiver,
        NODE_INFO_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &tlistener_receiver,
        TURTLE_CMD_VEL_RECEIVER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    // senders
    let mut nodes_updated_sender: Option<mio_channel::SyncSender<DataUpdate>> = None;

    'asdf: loop {
      let mut events = Events::with_capacity(100);
      poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        if event.token() == ROS2_COMMAND_TOKEN {
          while let Ok(command) = command_receiver.try_recv() {
            match command {
              RosCommand::StopEventLoop => {
                tc_ts_sender.send(ThreadControl::Stop).unwrap_or(());
                tc_tl_sender.send(ThreadControl::Stop).unwrap_or(());

                ros_participant.clear();
                break 'asdf;
              }
              RosCommand::AddNodeListSender { sender } => nodes_updated_sender = Some(sender),
              RosCommand::TurtleCmdVel { twist } => match tsender_sender.send(twist) {
                Ok(_) => (),
                Err(e) => error!("Failed to send to turtle sender. {:?}", e),
              },
            };
          }
        } else if event.token() == ROS2_NODE_RECEIVED_TOKEN {
          debug!("Started reading nodes.");
          let pts = ros_participant.handle_node_read();
          debug!("Nodes read");
          for pis in pts.iter() {
            match &nodes_updated_sender {
              Some(s) => {
                match s.send(DataUpdate::UpdateNode { info: pis.clone() }) {
                  Ok(_) => (),
                  Err(e) => error!("Failed to update node. {:?}", e),
                };
              }
              None => (),
            }
          }
          debug!("Finished reading nodes.");
        } else if event.token() == TURTLE_CMD_VEL_RECEIVER_TOKEN {
          match &nodes_updated_sender {
            Some(s) => {
              while let Ok(twist) = tlistener_receiver.try_recv() {
                match s.send(DataUpdate::TurtleCmdVel { twist }) {
                  Ok(_) => (),
                  Err(e) => error!("Failed to send TurtleCmdVel command. {:?}", e),
                }
              }
            }
            None => (),
          }
        } else if event.token() == TOPIC_UPDATE_TIMER_TOKEN {
          let list = domain_participant.get_discovered_topics();
          match &nodes_updated_sender {
            Some(s) => s.send(DataUpdate::TopicList { list }).unwrap(),
            None => (),
          };
          update_timer.set_timeout(Duration::from_secs(1), ());
        } else if event.token() == NODE_INFO_TOKEN {
          while let Ok(node_info_command) = nodeinfo_receiver.try_recv() {
            match node_info_command {
              commands::NodeInfoCommand::Add { node_info } => {
                ros_participant.add_node_info(node_info);
              }
              commands::NodeInfoCommand::Remove { node_info } => {
                ros_participant.remove_node_info(&node_info);
              }
            }
          }
        }
      }
    }
  }
}
