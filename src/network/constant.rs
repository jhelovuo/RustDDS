use mio::Token;

pub const STOP_POLL_TOKEN: Token = Token(0);
pub const ADD_UDP_LISTENER_TOKEN: Token = Token(1);
pub const REMOVE_UDP_LISTENER_TOKEN: Token = Token(2);
pub const ADD_UDP_SENDER_TOKEN: Token = Token(3);
pub const REMOVE_UDP_SENDER_TOKEN: Token = Token(4);
pub const START_FREE_TOKENS: Token = Token(5);
