macro_rules! assert_ser_de {
    ($({ $name:ident, $structure:expr, le = $le:expr, be = $be:expr }),+) => {
        $(mod $name {
            extern crate cdr;

            use super::*;
            use self::cdr::{CdrLe, CdrBe, Infinite, serialize, deserialize};

            fn remove_cdr_header(data: &[u8]) -> &[u8] {
                &data[4..]
            }

            #[test]
            fn serialize_deserialize_little_endian()
            {
                let structure = $structure;

                let encoded = serialize::<_, _, CdrLe>(&structure, Infinite).unwrap();
                assert_eq!($le, remove_cdr_header(&encoded));

                let decoded = deserialize(&encoded[..]).unwrap();
                assert_eq!($structure, decoded);
            }

            #[test]
            fn serialize_deserialize_big_endian() {
                let structure = $structure;

                let encoded = serialize::<_, _, CdrBe>(&structure, Infinite).unwrap();
                assert_eq!($be, remove_cdr_header(&encoded));

                let decoded = deserialize(&encoded[..]).unwrap();
                assert_eq!($structure, decoded);
            }
        })+
    }
}
