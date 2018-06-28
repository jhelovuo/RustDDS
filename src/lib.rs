extern crate cdr;
extern crate bit_set;
extern crate serde;
#[macro_use]
extern crate serde_derive;

pub mod common;
mod history_cache;
mod participant;
pub mod message_receiver;
pub mod message;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
