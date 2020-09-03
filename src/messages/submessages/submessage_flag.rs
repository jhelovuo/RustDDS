use speedy::{Endianness, Readable};
use enumflags2::BitFlags;


pub trait FromEndianness {
  fn from_endianness(end: speedy::Endianness) -> Self;
}


macro_rules! submessageflag_impls {
  ($t:ident) => {
    impl FromEndianness for BitFlags<$t> {
        fn from_endianness(end: speedy::Endianness) -> Self {
          if end == Endianness::LittleEndian { $t::Endianness.into() } 
          else { Self::empty() }
        }
    }

    /* This does not work, because BitFlags is not a local type. 
    impl<C> Writable<C: Context> for BitFlags<$t> {
      fn write_to<T: ?Sized + Writer<C>>(&self, writer: &mut T) -> Result<(), C::Error> {
        writer.write_u8( self.bits() )
      }
    }
    */    
  }
}

/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum ACKNACK_Flags {
  Endianness = 0b01,
  Final = 0b10,
}
submessageflag_impls!(ACKNACK_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum DATA_Flags {
  Endianness         = 0b00001,
  InlineQos          = 0b00010,
  Data               = 0b00100,
  Key                = 0b01000,
  NonStandardPayload = 0b10000,  
}
submessageflag_impls!(DATA_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum DATAFRAG_Flags {
  Endianness         = 0b00001,
  InlineQos          = 0b00010,
  Key                = 0b00100,
  NonStandardPayload = 0b01000,  
}
submessageflag_impls!(DATAFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum GAP_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(GAP_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum HEARTBEAT_Flags {
  Endianness         = 0b00001,
  Final              = 0b00010,
  Liveliness         = 0b00100,
}
submessageflag_impls!(HEARTBEAT_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum HEARTBEATFRAG_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(HEARTBEATFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum INFODESTINATION_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(INFODESTINATION_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum INFOREPLY_Flags {
  Endianness = 0b01,
  Multicast  = 0b10,
}
submessageflag_impls!(INFOREPLY_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum INFOSOURCE_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(INFOSOURCE_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum INFOTIMESTAMP_Flags {
  Endianness = 0b01,
  Invalidate = 0b10,
}
submessageflag_impls!(INFOTIMESTAMP_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum PAD_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(PAD_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum NACKFRAG_Flags {
  Endianness         = 0b00001,
}
submessageflag_impls!(NACKFRAG_Flags);

#[derive(BitFlags, Debug, PartialOrd, PartialEq, Ord, Eq, Readable,  Clone, Copy)]
#[repr(u8)]
pub enum INFOREPLYIP4_Flags {
  Endianness = 0b01,
  Multicast  = 0b10,
}
submessageflag_impls!(INFOREPLYIP4_Flags);

pub fn endianness_flag(flags: u8) -> speedy::Endianness {
    if (flags & 0x01) != 0 {
      Endianness::LittleEndian
    } else {
      Endianness::BigEndian
    }  
}



#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn endianness_flag_test() {
    assert_eq!(
      Endianness::BigEndian,
      endianness_flag(0x00)
    );
    assert_eq!(
      Endianness::LittleEndian,
      endianness_flag(0x01)
    );
  }

  /*#[test]  This is a weird test. Bytes have no endianness (unless streaming bits on serial link).
  fn correct_bits_order() {
    let submessage_flag = SubmessageFlag {
      flags: 0b10110100_u8,
    };

    assert!(!submessage_flag.is_flag_set(0b0000_0001));
    assert!(!submessage_flag.is_flag_set(0b0000_0010));
    assert!(submessage_flag.is_flag_set(0b0000_0100));
    assert!(!submessage_flag.is_flag_set(0b0000_1000));
    assert!(submessage_flag.is_flag_set(0b0001_0000));
    assert!(submessage_flag.is_flag_set(0b0010_0000));
    assert!(!submessage_flag.is_flag_set(0b0100_0000));
    assert!(submessage_flag.is_flag_set(0b1000_0000));
  } */

  /*#[test] This should be unit tested inside enumflags2 crate.
  fn helper_functions_test() {
    for x in 0..7 {
      let mut flags = SubmessageFlag { flags: 0x00 };
      let bit = u8::from(2).pow(x);

      assert!(!flags.is_flag_set(bit));
      flags.set_flag(bit);
      assert!(flags.is_flag_set(bit));
      flags.clear_flag(bit);
      assert!(!flags.is_flag_set(bit));
    }
  }*/
  
  /*
  serialization_test!(type = SubmessageFlag,
  {
      submessage_flag,
      SubmessageFlag { flags: 0b10110100_u8 },
      le = [0b10110100_u8],
      be = [0b10110100_u8]
  });
  */

  /* testing removed functionality here
  #[test]
  fn test_RTPS_submessage_flags_helper(){
    let fla : SubmessageFlag = SubmessageFlag{
      flags: 0b00000001_u8,
    };
    let mut helper = SubmessageFlagHelper::get_submessage_flags_helper_from_submessage_flag(&SubmessageKind::DATA, &fla);
    println!("{:?}",&helper);
    assert_eq!(helper.EndiannessFlag,true);
    assert_eq!(helper.InlineQosFlag,false);
    assert_eq!(helper.DataFlag,false);
    assert_eq!(helper.NonStandardPayloadFlag,false);
    assert_eq!(helper.FinalFlag,false);
    assert_eq!(helper.InvalidateFlag,false);
    assert_eq!(helper.KeyFlag,false);
    assert_eq!(helper.LivelinessFlag,false);
    assert_eq!(helper.MulticastFlag,false);

    let fla_dese = SubmessageFlagHelper::create_submessage_flags_from_flag_helper(&SubmessageKind::DATA, &helper);
    assert_eq!(fla, fla_dese);

    let fla2 : SubmessageFlag = SubmessageFlag{
      flags: 0b00011111_u8,
    };
    helper = SubmessageFlagHelper::get_submessage_flags_helper_from_submessage_flag(&SubmessageKind::DATA, &fla2);
    println!("{:?}",&helper);
    assert_eq!(helper.EndiannessFlag,true);
    assert_eq!(helper.InlineQosFlag,true);
    assert_eq!(helper.DataFlag,true);
    assert_eq!(helper.NonStandardPayloadFlag,true);
    assert_eq!(helper.FinalFlag,false);
    assert_eq!(helper.InvalidateFlag,false);
    assert_eq!(helper.KeyFlag,true);
    assert_eq!(helper.LivelinessFlag,false);
    assert_eq!(helper.MulticastFlag,false);

    let fla2_dese = SubmessageFlagHelper::create_submessage_flags_from_flag_helper(&SubmessageKind::DATA, &helper);
    assert_eq!(fla2, fla2_dese);

    let fla3 : SubmessageFlag = SubmessageFlag{
      flags: 0b00001010_u8,
    };
    helper = SubmessageFlagHelper::get_submessage_flags_helper_from_submessage_flag(&SubmessageKind::DATA, &fla3);
    println!("{:?}",&helper);
    assert_eq!(helper.EndiannessFlag,false);
    assert_eq!(helper.InlineQosFlag,true);
    assert_eq!(helper.DataFlag,false);
    assert_eq!(helper.NonStandardPayloadFlag,false);
    assert_eq!(helper.FinalFlag,false);
    assert_eq!(helper.InvalidateFlag,false);
    assert_eq!(helper.KeyFlag,true);
    assert_eq!(helper.LivelinessFlag,false);
    assert_eq!(helper.MulticastFlag,false);



    let fla3_dese = SubmessageFlagHelper::create_submessage_flags_from_flag_helper(&SubmessageKind::DATA, &helper);
    assert_eq!(fla3, fla3_dese);
  }
  */
}
