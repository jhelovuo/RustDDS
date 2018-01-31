extern crate cdr;
extern crate bit_set;

mod common;
mod history_cache;
mod participant;
mod message;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
