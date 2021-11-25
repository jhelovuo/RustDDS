#![deny(clippy::all)]

use std::{io::Write, time::Duration};

use termion::raw::*;

#[allow(unused_imports)] 
use log::{debug, info, warn, error};

use serde::{Deserialize, Serialize};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;

use rustdds::{
  ros2::{NodeOptions, RosParticipant},
  serialization::CDRDeserializerAdapter,
  serialization::CDRSerializerAdapter,
  dds::data_types::{DDSDuration, TopicKind},
  dds::qos::{
    policy::{
      Durability, History,  Liveliness,
      Reliability,
    },
    QosPolicies, QosPolicyBuilder,
  },
};

use structures::{DataUpdate, MainController, RosCommand};

// modules
mod structures;

const TURTLE_CMD_VEL_READER_TOKEN: Token = Token(1);
const ROS2_COMMAND_TOKEN: Token = Token(2);
const ROS2_NODE_RECEIVED_TOKEN: Token = Token(3);
const TOPIC_UPDATE_TIMER_TOKEN: Token = Token(4);

// This corresponds to ROS2 message type 
// https://github.com/ros2/common_interfaces/blob/master/geometry_msgs/msg/Twist.msg
//
// The struct definition must have a layout corresponding to the
// ROS2 msg definition to get compatible serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Twist {
  pub linear: Vector3,
  pub angular: Vector3,
}

// This corresponds to ROS2 message type
// https://github.com/ros2/common_interfaces/blob/master/geometry_msgs/msg/Vector3.msg
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vector3 {
  pub x: f64,
  pub y: f64,
  pub z: f64,
}

fn main() {
  log4rs::init_file("examples/ros2_demo/log4rs.yaml", Default::default()).unwrap();

  let (command_sender, command_receiver) = mio_channel::sync_channel::<RosCommand>(10);
  let (nodelist_sender, nodelist_receiver) = mio_channel::sync_channel(100);

  let jhandle = std::thread::Builder::new()
      .name("ros2_loop".into())
      .spawn(move || ros2_loop(command_receiver, nodelist_sender))
      .unwrap();

  // raw mode stdout for termion library.
  let stdout_org = std::io::stdout();
  let mut stdout = stdout_org.lock().into_raw_mode().unwrap();

  let mut main_control = MainController::new(stdout, command_sender, nodelist_receiver);
  main_control.start();

  jhandle.join().unwrap();

  // need to wait a bit for cleanup, beacuse drop is not waited for join
  std::thread::sleep(Duration::from_millis(10));
}

fn ros2_loop( command_receiver: mio_channel::Receiver<RosCommand>, 
              nodelist_sender: mio_channel::SyncSender<DataUpdate>, ) 
{
  info!("ros2_loop");

  let qos: QosPolicies = {
    QosPolicyBuilder::new()
      .durability(Durability::Volatile)
      .liveliness(Liveliness::Automatic {
        lease_duration: DDSDuration::DURATION_INFINITE,
      })
      .reliability(Reliability::Reliable {
        max_blocking_time: DDSDuration::from_millis(100),
      })
      .history(History::KeepLast { depth: 10 })
      .build()
  };

  let mut ros_participant = RosParticipant::new().unwrap();

  // topic update timer (or any update)
  let mut update_timer = mio_extras::timer::Timer::default();
  update_timer.set_timeout(Duration::from_secs(1), ());

  let mut ros_node = ros_participant
    .new_ros_node(
      "turtle_listener",       // name
      "/ros2_demo",            // namespace
      NodeOptions::new(false), // enable rosout
    )
    .unwrap();

  let turtle_cmd_vel_topic = ros_node
    .create_ros_topic(
      "/turtle1/cmd_vel",
      String::from("geometry_msgs::msg::dds_::Twist_"),
      qos.clone(),
      TopicKind::NoKey,
    )
    .unwrap();

  let turtle_cmd_vel_writer = ros_node
    .create_ros_nokey_publisher::<Twist, CDRSerializerAdapter<Twist>>(turtle_cmd_vel_topic.clone(), None)
    .unwrap();

  let mut turtle_cmd_vel_reader = ros_node
    .create_ros_nokey_subscriber::<Twist, CDRDeserializerAdapter<_>>(turtle_cmd_vel_topic, None)
    .unwrap();

  info!("ros2_loop: ros2 objects created");

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
      &turtle_cmd_vel_reader,
      TURTLE_CMD_VEL_READER_TOKEN,
      Ready::readable(),
      PollOpt::edge(),
    )
    .unwrap();


  // senders
  //let mut nodes_updated_sender: Option<mio_channel::SyncSender<DataUpdate>> = None;
  info!("Entering event_loop");
  'event_loop: loop {
    let mut events = Events::with_capacity(100);
    poll.poll(&mut events, None).unwrap();

    for event in events.iter() {
      if event.token() == ROS2_COMMAND_TOKEN {
        while let Ok(command) = command_receiver.try_recv() {
          //info!("ROS2 command {:?}",command);
          match command {
            RosCommand::StopEventLoop => {
              // tc_ts_sender.send(ThreadControl::Stop).unwrap_or(());
              // tc_tl_sender.send(ThreadControl::Stop).unwrap_or(());
              info!("Stopping main event loop");
              ros_participant.clear();
              break 'event_loop
            }
            //RosCommand::AddNodeListSender { sender } => nodes_updated_sender = Some(sender),
            RosCommand::TurtleCmdVel { twist } => 
              match turtle_cmd_vel_writer.write(twist.clone(), None) {
                Ok(_) => { info!("Wrote to ROS2 {:?}",twist); }
                Err(e) => {
                  error!("Failed to write to turtle writer. {:?}", e);
                  ros_node.clear_node();
                  return;
                }
              },
          };
        }
      } else if event.token() == ROS2_NODE_RECEIVED_TOKEN {
        debug!("Started reading nodes.");
        let pts = ros_participant.handle_node_read();
        debug!("Nodes read");
        for pis in pts.iter() {
          match nodelist_sender.send(DataUpdate::UpdateNode { info: pis.clone() }) {
            Ok(_) => (),
            Err(e) => error!("Failed to update node. {:?}", e),
          }
        }
        debug!("Finished reading nodes.");
      } else if event.token() == TOPIC_UPDATE_TIMER_TOKEN {
        let list = ros_participant.discovered_topics();
        nodelist_sender.send(DataUpdate::TopicList{ list }).unwrap();
        update_timer.set_timeout(Duration::from_secs(1), ());
      } else if event.token() == TURTLE_CMD_VEL_READER_TOKEN {
        while let Ok(Some(twist)) = turtle_cmd_vel_reader.take_next_sample() {
          debug!("Sending twist {:?}",twist.value() );
          nodelist_sender
            .send(DataUpdate::TurtleCmdVel { twist: twist.value().clone() })
            .unwrap();
        }
      }  else {
        error!("Unknown poll token {:?}", event.token())
      }
    }
  }
  
}
