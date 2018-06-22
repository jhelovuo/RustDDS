extern crate rtps;
use self::rtps::common::sequence_number::{SequenceNumber_t};

#[test]
fn sequence_number_starts_by_default_from_one() {
    let default_sequence_number = SequenceNumber_t::default();
    assert_eq!(SequenceNumber_t { high: 0, low: 1}, default_sequence_number);
    assert_eq!(SequenceNumber_t { high: 0, low: 1}, default_sequence_number);
    assert_eq!(1, default_sequence_number.value());
}

#[test]
fn sequence_number_addition_with_other_sequence_number() {
    {
        let left = SequenceNumber_t { high: 0, low: 0 };
        let right = SequenceNumber_t { high: 0, low: 0 };
        assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left+right);
    }
    {
        let left = SequenceNumber_t { high: 0, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 0, low: 30 }, left+right);
    }
    {
        let left = SequenceNumber_t { high: 1, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 1, low: 30 }, left+right);
    }
}

#[test]
#[should_panic]
fn sequeance_number_addition_with_other_sequence_number_with_wrap() {
    let left = SequenceNumber_t { high: <i32>::max_value(), low: <u32>::max_value() };
    let right = SequenceNumber_t { high: 0, low: 1 };
    left + right;
}

#[test]
fn sequence_number_subtraction_with_other_sequence_number() {
    {
        let left = SequenceNumber_t { high: 0, low: 0 };
        let right = SequenceNumber_t { high: 0, low: 0 };
        assert_eq!(SequenceNumber_t { high: 0, low: 0 }, left-right);
    }
    {
        let left = SequenceNumber_t { high: 0, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 0, low: 10 }, left-right);
    }
    {
        let left = SequenceNumber_t { high: 1, low: 20 };
        let right = SequenceNumber_t { high: 0, low: 10 };
        assert_eq!(SequenceNumber_t { high: 1, low: 10 }, left-right);
    }
}

#[test]
#[should_panic]
fn sequeance_number_subtraction_with_other_sequence_number_with_wrap() {
    let left = SequenceNumber_t { high: 0, low: 0 };
    let right = SequenceNumber_t { high: 0, low: 10 };
    left - right;
}

#[test]
fn sequeance_number_compare_with_other_sequence_number() {
    assert!(SequenceNumber_t { high: 0, low: 0 } == SequenceNumber_t { high: 0, low: 0 });
    assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 0, low: 1 });
    assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 1, low: 0 });
    assert!(SequenceNumber_t { high: 0, low: 0 } != SequenceNumber_t { high: 1, low: 1 });

    assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 0, low: 1 });
    assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 1, low: 0 });
    assert!(SequenceNumber_t { high: 0, low: 0 } < SequenceNumber_t { high: 1, low: 1 });
    assert!(SequenceNumber_t { high: 0, low: 1 } > SequenceNumber_t { high: 0, low: 0 });
    assert!(SequenceNumber_t { high: 0, low: 1 } == SequenceNumber_t { high: 0, low: 1 });
    assert!(SequenceNumber_t { high: 0, low: 1 } < SequenceNumber_t { high: 1, low: 0 });
    assert!(SequenceNumber_t { high: 0, low: 1 } < SequenceNumber_t { high: 1, low: 1 });

    assert!(SequenceNumber_t { high: 1, low: 0 } > SequenceNumber_t { high: 0, low: 0 });
    assert!(SequenceNumber_t { high: 1, low: 0 } > SequenceNumber_t { high: 0, low: 1 });
    assert!(SequenceNumber_t { high: 1, low: 0 } == SequenceNumber_t { high: 1, low: 0 });
    assert!(SequenceNumber_t { high: 1, low: 0 } < SequenceNumber_t { high: 1, low: 1 });
    assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 0, low: 0 });
    assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 0, low: 1 });
    assert!(SequenceNumber_t { high: 1, low: 1 } > SequenceNumber_t { high: 1, low: 0 });
}
