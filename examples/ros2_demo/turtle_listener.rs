use atosdds::{
  dds::DomainParticipant, dds::traits::Entity, ros2::NodeOptions, ros2::RosContext, ros2::RosNode,
  ros2::RosNodeBuilder, serialization::CDRDeserializerAdapter, ros2::IRosNodeControl,
};

use log::{info, warn};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;

use crate::{
  commands::{NodeInfoCommand, ThreadControl},
  ros2::turtle_data::{TurtleCmdVelTopic, Twist},
};

pub struct TurtleListener {}

impl TurtleListener {
  const THREAD_CONTROL_TOKEN: Token = Token(0);
  const TURTLE_CMD_VEL_READER_TOKEN: Token = Token(1);

  pub fn run(
    domain_participant: DomainParticipant,
    tc_receiver: mio_channel::Receiver<ThreadControl>,
    sender: mio_channel::Sender<Twist>,
    ni_sender: mio_channel::Sender<NodeInfoCommand>,
  ) {
    let ros_context = RosContext::new(domain_participant.clone(), false).unwrap();
    let ros_node_options = NodeOptions::new(domain_participant.domain_id(), true);

    // make sure topic lives long enough for ros_node
    let turtle_cmd_vel_topic = RosNode::create_ros_topic(
      &domain_participant,
      &TurtleCmdVelTopic::topic_name(),
      &TurtleCmdVelTopic::type_name(),
      TurtleCmdVelTopic::get_qos(),
      TurtleCmdVelTopic::topic_kind(),
    )
    .unwrap();

    {
      let mut ros_node = RosNodeBuilder::new()
        .name("turtle_listener")
        .namespace("/ros2_demo")
        .node_options(ros_node_options)
        .ros_context(&ros_context)
        .build()
        .unwrap();

      let mut turtle_cmd_vel_reader = ros_node
        .create_ros_nokey_subscriber::<Twist, CDRDeserializerAdapter<_>>(
          &turtle_cmd_vel_topic,
          None,
        )
        .unwrap();

      ros_node.add_reader(turtle_cmd_vel_reader.get_guid());

      ni_sender
        .send(NodeInfoCommand::Add {
          node_info: ros_node.generate_node_info(),
        })
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
            while let Ok(ctrl) = tc_receiver.try_recv() {
              match ctrl {
                ThreadControl::Stop => {
                  info!("Stopping TurtleListener");
                  ros_node.clear_node();
                  ni_sender
                    .send(NodeInfoCommand::Remove {
                      node_info: ros_node.generate_node_info(),
                    })
                    .unwrap_or(());
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
}
