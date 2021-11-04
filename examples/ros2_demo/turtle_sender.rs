use rustdds::{
  ros2::{NodeOptions, RosParticipant},
  serialization::CDRSerializerAdapter,
};
use log::{error, info};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;

use crate::{
  commands::ThreadControl,
  ros2::turtle_data::{TurtleCmdVelTopic, Twist},
};

pub struct TurtleSender {}

impl TurtleSender {
  const THREAD_CONTROL_TOKEN: Token = Token(0);
  const TURTLE_TWIST_TOKEN: Token = Token(1);

  pub fn run(
    ros_participant: RosParticipant,
    thread_control: mio_channel::Receiver<ThreadControl>,
    receiver: mio_channel::Receiver<Twist>,
  ) {
    let mut ros_node = ros_participant
      .new_ros_node(
        "turtle_sender",         // name
        "/ros2_demo",            // namespace
        NodeOptions::new(false), // enable rosout
      )
      .unwrap();

    let turtle_cmd_vel_topic = ros_node
      .create_ros_topic(
        &TurtleCmdVelTopic::topic_name(),
        TurtleCmdVelTopic::type_name(),
        TurtleCmdVelTopic::get_qos(),
        TurtleCmdVelTopic::topic_kind(),
      )
      .unwrap();

    let turtle_cmd_vel_writer = ros_node
      .create_ros_nokey_publisher::<Twist, CDRSerializerAdapter<Twist>>(turtle_cmd_vel_topic, None)
      .unwrap();

    let poll = Poll::new().unwrap();

    poll
      .register(
        &thread_control,
        TurtleSender::THREAD_CONTROL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &receiver,
        TurtleSender::TURTLE_TWIST_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();
    info!("TurtleSender initialized");
    loop {
      let mut events = Events::with_capacity(10);
      poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        if event.token() == TurtleSender::THREAD_CONTROL_TOKEN {
          if let Ok(ctrl) = thread_control.try_recv() {
            match ctrl {
              ThreadControl::Stop => {
                info!("Stopping TurtleSender");
                ros_node.clear_node();
                return;
              }
            }
          }
        } else if event.token() == TurtleSender::TURTLE_TWIST_TOKEN {
          while let Ok(twist) = receiver.try_recv() {
            match turtle_cmd_vel_writer.write(twist, None) {
              Ok(_) => { /*info!("Wrote twist!");*/ }
              Err(e) => {
                error!("Failed to write to turtle writer. {:?}", e);
                ros_node.clear_node();
                return;
              }
            }
          }
        }
      }
    }
  }
}
