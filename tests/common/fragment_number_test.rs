extern crate rtps;
use self::rtps::common::fragment_number::{FragmentNumber_t};

#[test]
fn fragment_number_starts_by_default_from_one() {
    assert_eq!(FragmentNumber_t { vendor_id: 1 }, FragmentNumber_t::default());
}
