use atosdds::ros2::builtin_datatypes::NodeInfo;

pub enum NodeInfoCommand {
  Add { node_info: NodeInfo },
  Remove { node_info: NodeInfo },
}

pub enum ThreadControl {
  Stop,
}
