pub struct MessageReceiver {}

impl MessageReceiver {
  pub fn new() -> MessageReceiver {
    MessageReceiver {}
  }

  pub fn handle_discovery_msg(&self, msg: Vec<u8>) {
    unimplemented!();
  }

  pub fn handle_user_msg(&self, msg: Vec<u8>) {
    unimplemented!();
  }
}
