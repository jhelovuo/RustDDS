use mio::Token;
use mio_extras::channel as mio_channel;

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


pub struct TokenReceiverPair<T> {
  pub token: Token,
  pub receiver: mio_channel::Receiver<T>,
}
