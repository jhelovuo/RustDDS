use speedy::{Endianness, Readable, Writable};
use super::submessage_kind::SubmessageKind;

/// Identifies the endianness used to encapsulate the Submessage, the
/// presence of optional elements with in the Submessage, and possibly
/// modifies the interpretation of the Submessage. There are
/// 8 possible flags. The first flag (index 0) identifies the
/// endianness used to encapsulate the Submessage. The remaining
/// flags are interpreted differently depending on the kind
/// of Submessage and are described separately for each Submessage.
#[derive(Debug, PartialOrd, PartialEq, Ord, Eq, Readable, Writable, Clone)]
pub struct SubmessageFlag {
  pub flags: u8,
}

impl SubmessageFlag {
  /// Indicates endianness
  pub fn endianness_flag(&self) -> speedy::Endianness {
    if self.is_flag_set(0x01) {
      Endianness::LittleEndian
    } else {
      Endianness::BigEndian
    }
  }

  pub fn set_flag(&mut self, bit: u8) {
    self.flags |= bit;
  }
  pub fn clear_flag(&mut self, bit: u8) {
    self.flags &= !bit;
  }
  pub fn is_flag_set(&self, bit: u8) -> bool {
    self.flags & bit != 0
  }
}



///Purpose of this object is to help RTPS Message SubmessageHeader SubmessageFlag value setting and value reading.
#[derive(Debug, Clone)]
pub struct SubmessageFlagHelper {
  pub KeyFlag : bool,         
  pub DataFlag: bool,         
  pub InlineQosFlag : bool,   
  pub NonStandardPayloadFlag: bool, 
  pub EndiannessFlag : bool,
  pub FinalFlag : bool,        
  pub LivelinessFlag : bool,   
  pub MulticastFlag : bool,    
  pub InvalidateFlag : bool,   
}
impl SubmessageFlagHelper {
  pub fn new(endianness : Endianness) -> SubmessageFlagHelper{
    let mut f = SubmessageFlagHelper {
      KeyFlag : false,        
      DataFlag: false,         
      InlineQosFlag : false,   
      NonStandardPayloadFlag : false,
      EndiannessFlag : false,
      FinalFlag : false,        
      LivelinessFlag : false,   
      MulticastFlag : false,    
      InvalidateFlag : false,   
    };
    if endianness == Endianness::LittleEndian{
      f.EndiannessFlag = true;
    }
    return f;
  }

  pub fn get_endianness(&self) -> Endianness {
    if self.EndiannessFlag == true{
      return Endianness::LittleEndian;
    }else{
      return Endianness::BigEndian;
    }
  }

  ///Meaning of each bit is different depending on the message submessage type.
  ///Flags are u8 long -> possibility of 8 diffenrent flags, but not all are used.
  pub fn get_submessage_flags_helper_from_submessage_flag(submessage_kind : &SubmessageKind, flags : &SubmessageFlag) -> SubmessageFlagHelper{
    let mut helper = SubmessageFlagHelper::new(Endianness::BigEndian);

    match submessage_kind {
      //|X|X|X|N|K|D|Q|E|
      //NonStandardPayloadFlag, Key, DataFlag, InlineQosFlag, EndiannessFlag
      &SubmessageKind::DATA =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.InlineQosFlag = flags.is_flag_set(2u8.pow(1));
        helper.DataFlag = flags.is_flag_set(2u8.pow(2));
        helper.KeyFlag = flags.is_flag_set(2u8.pow(3));
        helper.NonStandardPayloadFlag = flags.is_flag_set(2u8.pow(4));
      }
      //|X|X|X|X|N|K|Q|E|
      //NonStandardPayloadFlag, Key, InlineQosFlag, EndiannessFlag
      &SubmessageKind::DATA_FRAG =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.InlineQosFlag = flags.is_flag_set(2u8.pow(1));
        helper.KeyFlag = flags.is_flag_set(2u8.pow(2));
        helper.NonStandardPayloadFlag = flags.is_flag_set(2u8.pow(3));
      }
      //|X|X|X|X|X|X|F|E|
      //FinalFlag,EndiannessFlag
      &SubmessageKind::ACKNACK =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.FinalFlag = flags.is_flag_set(2u8.pow(1));
      }
      //|X|X|X|X|X|L|F|E|
      //LivelinessFlag,FinalFlag,EndiannessFlag
      &SubmessageKind::HEARTBEAT =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.FinalFlag = flags.is_flag_set(2u8.pow(1));
        helper.LivelinessFlag = flags.is_flag_set(2u8.pow(2));
      }
      //|X|X|X|X|X|X|X|E| 
      //EndiannessFlag
      &SubmessageKind::HEARTBEAT_FRAG =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      //|X|X|X|X|X|X|X|E|
      //EndiannessFlag
      &SubmessageKind::INFO_DST => {
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      //|X|X|X|X|X|X|M|E|
      //MulticastFlag,EndiannessFlag
      &SubmessageKind::INFO_REPLY => {
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.MulticastFlag = flags.is_flag_set(2u8.pow(1));
      }
      //|X|X|X|X|X|X|X|E| 
      //EndiannessFlag
      &SubmessageKind::INFO_SRC => {
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      //|X|X|X|X|X|X|I|E| 
      //InvalidateFlag, EndiannessFlag
      &SubmessageKind::INFO_TS => {
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.InvalidateFlag = flags.is_flag_set(2u8.pow(1));
      }
      //|X|X|X|X|X|X|X|E|
      //EndiannessFlag
      &SubmessageKind::PAD =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      //|X|X|X|X|X|X|X|E|
      //EndiannessFlag
      &SubmessageKind::NACK_FRAG =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      //|X|X|X|X|X|X|M|E| 
      //MulticastFlag,EndiannessFlag
      &SubmessageKind::INFO_REPLY_IP4 =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
        helper.MulticastFlag = flags.is_flag_set(2u8.pow(1));
      } 
      //|X|X|X|X|X|X|X|E| 
      //EndiannessFlag
      &SubmessageKind::GAP =>{
        helper.EndiannessFlag = flags.is_flag_set(2u8.pow(0));
      }
      _ =>{
        todo!();
      }
    }
    return helper;
  }

  pub fn create_submessage_flags_from_flag_helper(submessage_kind : &SubmessageKind, helper : &SubmessageFlagHelper,) -> SubmessageFlag{
    let mut flags = SubmessageFlag{flags: 0b00000000_u8};
   
    match submessage_kind {
      //|X|X|X|N|K|D|Q|E|
      //NonStandardPayloadFlag, Key, DataFlag, InlineQosFlag, EndiannessFlag
      &SubmessageKind::DATA  => { 
        if helper.EndiannessFlag{
          flags.set_flag(2u8.pow(0));
        }
        if helper.InlineQosFlag{
          flags.set_flag(2u8.pow(1));
        }
        if helper.DataFlag {
          flags.set_flag(2u8.pow(2));
        }
        if helper.KeyFlag {
          flags.set_flag(2u8.pow(3));
        }
        if helper.NonStandardPayloadFlag{
          flags.set_flag(2u8.pow(4));
        }
      }
       //|X|X|X|X|N|K|Q|E|
      //NonStandardPayloadFlag, Key, InlineQosFlag, EndiannessFlag
      &SubmessageKind::DATA_FRAG =>{
        if helper.EndiannessFlag{
          flags.set_flag(2u8.pow(0));
        }
        if helper.InlineQosFlag{
          flags.set_flag(2u8.pow(1));
        }
        if helper.KeyFlag {
          flags.set_flag(2u8.pow(2));
        }
        if helper.NonStandardPayloadFlag{
          flags.set_flag(2u8.pow(3));
        }
      }
      //|X|X|X|X|X|X|I|E| 
      //InvalidateFlag, EndiannessFlag
      &SubmessageKind::INFO_TS =>{
        if helper.EndiannessFlag{
          flags.set_flag(2u8.pow(0));
        }
        if helper.InvalidateFlag{
          flags.set_flag(2u8.pow(1));
        }
      }
      //|X|X|X|X|X|L|F|E|
      //LivelinessFlag,FinalFlag,EndiannessFlag
      &SubmessageKind::HEARTBEAT =>{
        if helper.EndiannessFlag{
          flags.set_flag(2u8.pow(0));
        }
        if helper.FinalFlag{
          flags.set_flag(2u8.pow(1));
        }
        if helper.LivelinessFlag {
          flags.set_flag(2u8.pow(2));
        }
      }
       //|X|X|X|X|X|X|X|E|
      //EndiannessFlag
      &SubmessageKind::INFO_DST => {
        if helper.EndiannessFlag{
          flags.set_flag(2u8.pow(0));
        }
      }
      _ => {
        todo!();
      }
    }
    return flags;
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn endianness_flag() {
    assert_eq!(
      Endianness::BigEndian,
      SubmessageFlag { flags: 0x00 }.endianness_flag()
    );
    assert_eq!(
      Endianness::LittleEndian,
      SubmessageFlag { flags: 0x01 }.endianness_flag()
    );
  }

  #[test]
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
  }

  #[test]
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
  }

  serialization_test!(type = SubmessageFlag,
  {
      submessage_flag,
      SubmessageFlag { flags: 0b10110100_u8 },
      le = [0b10110100_u8],
      be = [0b10110100_u8]
  });

  
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

}
