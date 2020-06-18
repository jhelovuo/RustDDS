use speedy::{Readable, Writable};

#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Writable)]
pub struct ProtocolVersion_t {
  pub major: u8,
  pub minor: u8,
}

impl ProtocolVersion_t {
  pub const PROTOCOLVERSION: ProtocolVersion_t = ProtocolVersion_t::PROTOCOLVERSION_2_4;

  pub const PROTOCOLVERSION_1_0: ProtocolVersion_t = ProtocolVersion_t { major: 1, minor: 0 };
  pub const PROTOCOLVERSION_1_1: ProtocolVersion_t = ProtocolVersion_t { major: 1, minor: 1 };
  pub const PROTOCOLVERSION_2_0: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 0 };
  pub const PROTOCOLVERSION_2_1: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 1 };
  pub const PROTOCOLVERSION_2_2: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 2 };
  pub const PROTOCOLVERSION_2_4: ProtocolVersion_t = ProtocolVersion_t { major: 2, minor: 4 };
}

impl Default for ProtocolVersion_t {
  fn default() -> Self {
    ProtocolVersion_t::PROTOCOLVERSION
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  serialization_test!( type = ProtocolVersion_t,
  {
      protocol_version,
      ProtocolVersion_t::PROTOCOLVERSION,
      le = [0x02, 0x04],
      be = [0x02, 0x04]
  },
  {
      protocol_version_default,
      ProtocolVersion_t::default(),
      le = [0x02, 0x04],
      be = [0x02, 0x04]
  },
  {
      protocol_version_1_0,
      ProtocolVersion_t::PROTOCOLVERSION_1_0,
      le = [0x01, 0x00],
      be = [0x01, 0x00]
  },
  {
      protocol_version_1_1,
      ProtocolVersion_t::PROTOCOLVERSION_1_1,
      le = [0x01, 0x01],
      be = [0x01, 0x01]
  },
  {
      protocol_version_2_0,
      ProtocolVersion_t::PROTOCOLVERSION_2_0,
      le = [0x02, 0x00],
      be = [0x02, 0x00]
  },
  {
      protocol_version_2_1,
      ProtocolVersion_t::PROTOCOLVERSION_2_1,
      le = [0x02, 0x01],
      be = [0x02, 0x01]
  },
  {
      protocol_version_2_2,
      ProtocolVersion_t::PROTOCOLVERSION_2_2,
      le = [0x02, 0x02],
      be = [0x02, 0x02]
  },
  {
      protocol_version_2_4,
      ProtocolVersion_t::PROTOCOLVERSION_2_4,
      le = [0x02, 0x04],
      be = [0x02, 0x04]
  });
}
