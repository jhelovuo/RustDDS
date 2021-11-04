use rustdds::{
  ros2::{NodeOptions, RosParticipant},
  serialization::CDRDeserializerAdapter,
};
use log::{info, warn};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;

use crate::{
  commands::ThreadControl,
  ros2::turtle_data::{TurtleCmdVelTopic, Twist},
};

pub struct TurtleListener {}

impl TurtleListener {
  const THREAD_CONTROL_TOKEN: Token = Token(0);
  const TURTLE_CMD_VEL_READER_TOKEN: Token = Token(1);

  pub fn run(
    ros_participant: RosParticipant,
    tc_receiver: mio_channel::Receiver<ThreadControl>,
    sender: mio_channel::Sender<Twist>,
  ) {
    println!("Turtle listener");
    let mut ros_node = ros_participant
      .new_ros_node(
        "turtle_listener",       // name
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

    let mut turtle_cmd_vel_reader = ros_node
      .create_ros_nokey_subscriber::<Twist, CDRDeserializerAdapter<_>>(turtle_cmd_vel_topic, None)
      .unwrap();

    let poll = Poll::new().unwrap();

    poll
      .register(
        &tc_receiver,
        TurtleListener::THREAD_CONTROL_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    poll
      .register(
        &turtle_cmd_vel_reader,
        TurtleListener::TURTLE_CMD_VEL_READER_TOKEN,
        Ready::readable(),
        PollOpt::edge(),
      )
      .unwrap();

    loop {
      let mut events = Events::with_capacity(10);
      poll.poll(&mut events, None).unwrap();

      for event in events.iter() {
        if event.token() == TurtleListener::THREAD_CONTROL_TOKEN {
          if let Ok(ctrl) = tc_receiver.try_recv() {
            match ctrl {
              ThreadControl::Stop => {
                info!("Stopping TurtleListener");
                ros_node.clear_node();
                return;
              }
            }
          }
        } else if event.token() == TurtleListener::TURTLE_CMD_VEL_READER_TOKEN {
          while let Ok(Some(data_sample)) = turtle_cmd_vel_reader.take_next_sample() {
            sender
              .send(data_sample.into_value())
              .unwrap_or_else(|e| warn!("Failed to send received Twist. {:?}", e))
          }
        }
      }
    }
  }
}
