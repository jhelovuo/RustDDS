use mio::Token;
use mio_extras::channel as mio_channel;

use crate::structure::guid::GUID;

pub const STOP_POLL_TOKEN: Token = Token(0);

pub const DISCOVERY_SENDER_TOKEN: Token = Token(1);
pub const USER_TRAFFIC_SENDER_TOKEN: Token = Token(2);

pub const DATA_SEND_TOKEN: Token = Token(5);

pub const DISCOVERY_LISTENER_TOKEN: Token = Token(6);
pub const DISCOVERY_MUL_LISTENER_TOKEN: Token = Token(7);
pub const USER_TRAFFIC_LISTENER_TOKEN: Token = Token(8);
pub const USER_TRAFFIC_MUL_LISTENER_TOKEN: Token = Token(9);

pub const ADD_READER_TOKEN: Token = Token(10);
pub const REMOVE_READER_TOKEN: Token = Token(11);

pub const READER_CHANGE_TOKEN: Token = Token(12);
pub const DATAREADER_CHANGE_TOKEN: Token = Token(13);

pub const ADD_DATAREADER_TOKEN: Token = Token(14);
pub const REMOVE_DATAREADER_TOKEN: Token = Token(15);

pub const ADD_WRITER_TOKEN: Token = Token(16);
pub const REMOVE_WRITER_TOKEN: Token = Token(17);

pub const ADD_DATAWRITER_TOKEN: Token = Token(18);
pub const REMOVE_DATAWRITER_TOKEN: Token = Token(19);

pub const ACKNACK_MESSGAGE_TO_LOCAL_WRITER_TOKEN: Token = Token(20);

pub const DISCOVERY_UPDATE_NOTIFICATION_TOKEN: Token = Token(21);
pub const DISCOVERY_COMMAND_TOKEN: Token = Token(22);

pub const DISCOVERY_PARTICIPANT_DATA_TOKEN: Token = Token(30);
pub const DISCOVERY_PARTICIPANT_CLEANUP_TOKEN: Token = Token(31);
pub const DISCOVERY_SEND_PARTICIPANT_INFO_TOKEN: Token = Token(32);
pub const DISCOVERY_READER_DATA_TOKEN: Token = Token(33);
pub const DISCOVERY_SEND_READERS_INFO_TOKEN: Token = Token(34);
pub const DISCOVERY_WRITER_DATA_TOKEN: Token = Token(35);
pub const DISCOVERY_SEND_WRITERS_INFO_TOKEN: Token = Token(36);
pub const DISCOVERY_TOPIC_DATA_TOKEN: Token = Token(37);
pub const DISCOVERY_TOPIC_CLEANUP_TOKEN: Token = Token(38);
pub const DISCOVERY_SEND_TOPIC_INFO_TOKEN: Token = Token(39);
pub const DISCOVERY_PARTICIPANT_MESSAGE_TOKEN: Token = Token(40);
pub const DISCOVERY_PARTICIPANT_MESSAGE_TIMER_TOKEN: Token = Token(41);

pub const DPEV_ACKNACK_TIMER_TOKEN: Token = Token(50);

pub struct TokenReceiverPair<T> {
  pub token: Token,
  pub receiver: mio_channel::Receiver<T>,
}

const PB: u16 = 7400;
const DG: u16 = 250;
const PG: u16 = 2;

const D0: u16 = 0;
const D1: u16 = 10;
const D2: u16 = 1;
const D3: u16 = 11;

pub fn get_spdp_well_known_multicast_port(domain_id: u16) -> u16 {
  PB + DG * domain_id + D0
}

pub fn get_spdp_well_known_unicast_port(domain_id: u16, participant_id: u16) -> u16 {
  PB + DG * domain_id + D1 + PG * participant_id
}

pub fn get_user_traffic_multicast_port(domain_id: u16) -> u16 {
  PB + DG * domain_id + D2
}

pub fn get_user_traffic_unicast_port(domain_id: u16, participant_id: u16) -> u16 {
  PB + DG * domain_id + D3 + PG * participant_id
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimerMessageType {
  writer_heartbeat,
  writer_cache_cleaning,
  reader_deadline_missed_check
}

#[derive(Debug)]
pub enum DiscoveryNotificationType {
  ReadersInfoUpdated,
  WritersInfoUpdated { needs_new_cache_change: bool },
  TopicsInfoUpdated,
  AssertTopicLiveliness { writer_guid: GUID },
}
