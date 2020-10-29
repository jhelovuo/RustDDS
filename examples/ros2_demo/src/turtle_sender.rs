use atosdds::{
  dds::DomainParticipant, ros2::NodeOptions, ros2::RosContext, ros2::RosNode, ros2::RosNodeBuilder,
  serialization::CDRSerializerAdapter, ros2::IRosNodeControl,
};

use log::{error, info};
use mio::{Events, Poll, PollOpt, Ready, Token};
use mio_extras::channel as mio_channel;

use crate::{
  commands::{NodeInfoCommand, ThreadControl},
  ros2::turtle_data::{TurtleCmdVelTopic, Twist},
};

pub struct TurtleSender {}

impl TurtleSender {
  const THREAD_CONTROL_TOKEN: Token = Token(0);
  const TURTLE_TWIST_TOKEN: Token = Token(1);

  pub fn run(
    domain_participant: DomainParticipant,
    thread_control: mio_channel::Receiver<ThreadControl>,
    receiver: mio_channel::Receiver<Twist>,
    ni_sender: mio_channel::Sender<NodeInfoCommand>,
  ) {
    let ros_context = RosContext::new(domain_participant.clone(), false).unwrap();
    let ros_node_options = NodeOptions::new(domain_participant.domain_id(), false);

    // make sure topic lives long enough for ros_node
    let turtle_cmd_vel_topic = RosNode::create_ros_topic(
      &domain_participant,
      &TurtleCmdVelTopic::topic_name(),
      &TurtleCmdVelTopic::type_name(),
      TurtleCmdVelTopic::get_qos(),
    )
    .unwrap();

    {
      let mut ros_node = RosNodeBuilder::new()
        .name("turtle_sender")
        .namespace("/ros2_demo")
        .node_options(ros_node_options)
        .ros_context(&ros_context)
        .build()
        .unwrap();

      let turtle_cmd_vel_writer = ros_node
        .create_ros_nokey_publisher::<Twist, CDRSerializerAdapter<Twist>>(
          &turtle_cmd_vel_topic,
          None,
        )
        .unwrap();

      ros_node.add_writer(turtle_cmd_vel_writer.get_guid());

      ni_sender
        .send(NodeInfoCommand::Add {
          node_info: ros_node.generate_node_info(),
        })
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

      loop {
        let mut events = Events::with_capacity(10);
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
          if event.token() == TurtleSender::THREAD_CONTROL_TOKEN {
            while let Ok(ctrl) = thread_control.try_recv() {
              match ctrl {
                ThreadControl::Stop => {
                  info!("Stopping TurtleSender");
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
          } else if event.token() == TurtleSender::TURTLE_TWIST_TOKEN {
            while let Ok(twist) = receiver.try_recv() {
              match turtle_cmd_vel_writer.write(twist, None) {
                Ok(_) => (),
                Err(e) => {
                  error!("Failed to write to turtle writer. {:?}", e);
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
          }
        }
      }
    }
  }
}
