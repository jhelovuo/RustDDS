use serde::{Serialize, Deserialize};

use crate::{
  dds::traits::key::Key,
  structure::{guid::GUID, time::Timestamp},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Gid {
  data: [u8; 24],
}

impl Gid {
  pub fn from_guid(guid: GUID) -> Gid {
    let mut data: [u8; 24] = [0; 24];
    data[..12].clone_from_slice(&guid.guidPrefix.entityKey);
    data[12..15].clone_from_slice(&guid.entityId.entityKey);
    data[15..16].clone_from_slice(&[guid.entityId.entityKind]);
    Gid { data }
  }
}

impl Key for Gid {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeInfo {
  pub node_namespace: String,
  pub node_name: String,
  pub reader_guid: Vec<Gid>,
  pub writer_guid: Vec<Gid>,
}

impl NodeInfo {
  pub fn get_full_name(&self) -> String {
    let mut name = self.node_namespace.to_owned();
    name.push_str(&self.node_name);
    name
  }

  pub fn clear_all(&mut self) {
    self.reader_guid.clear();
    self.writer_guid.clear();
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROSParticipantInfo {
  guid: Gid,
  nodes: Vec<NodeInfo>,
}

impl ROSParticipantInfo {
  pub fn new(guid: Gid, nodes: Vec<NodeInfo>) -> ROSParticipantInfo {
    ROSParticipantInfo { guid, nodes }
  }

  pub fn guid(&self) -> Gid {
    self.guid
  }

  pub fn into_nodes(self) -> Vec<NodeInfo> {
    self.nodes
  }

  pub fn nodes(&self) -> &Vec<NodeInfo> {
    &self.nodes
  }

  pub fn nodes_mut(&mut self) -> &mut Vec<NodeInfo> {
    &mut self.nodes
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterEvents {
  timestamp: Timestamp,
  // fully qualified path
  node: String,
  new_parameters: Vec<Parameter>,
  changed_parameters: Vec<Parameter>,
  deleted_parameters: Vec<Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
  name: String,
  value: ParameterValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterValue {
  ptype: u8,
  boolean_value: bool,
  int_value: i64,
  double_value: f64,
  string_value: String,
  byte_array: Vec<u8>,
  bool_array: Vec<bool>,
  int_array: Vec<i64>,
  double_array: Vec<f64>,
  string_array: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
  timestamp: Timestamp,
  level: u8,
  name: String,
  msg: String,
  file: String,
  function: String,
  line: u32,
}
