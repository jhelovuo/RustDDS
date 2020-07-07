use mio::Token;

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
