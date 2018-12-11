use crate::common::submessage_flag;
use speedy::{Context, Readable, Reader, Writable, Writer};
use std::io::{Error, ErrorKind, Result};
use std::mem::size_of;

#[derive(Debug, Eq, PartialEq)]
pub enum SubmessageKind {
    PAD = 0x01,
    ACKNACK = 0x06,
    HEARTBEAT = 0x07,
    GAP = 0x08,
    INFO_TS = 0x09,
    INFO_SRC = 0x0c,
    INFO_REPLAY_IP4 = 0x0d,
    INFO_DST = 0x0e,
    INFO_REPLAY = 0x0f,
    NACK_FRAG = 0x12,
    HEARTBEAT_FRAG = 0x13,
    DATA = 0x15,
    DATA_FRAG = 0x16,
}

#[derive(Debug, PartialEq, Readable, Writable)]
pub struct SubmessageHeader {
    pub submessage_id: SubmessageKind,
    pub flags: submessage_flag::SubmessageFlag,
    pub submessage_length: u16,
}

impl<'a, C: Context> Readable<'a, C> for SubmessageKind {
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self> {
        let id = reader.read_u8()?;
        match id {
            0x01 => Ok(SubmessageKind::PAD),
            0x06 => Ok(SubmessageKind::ACKNACK),
            0x07 => Ok(SubmessageKind::HEARTBEAT),
            0x08 => Ok(SubmessageKind::GAP),
            0x09 => Ok(SubmessageKind::INFO_TS),
            0x0c => Ok(SubmessageKind::INFO_SRC),
            0x0d => Ok(SubmessageKind::INFO_REPLAY_IP4),
            0x0e => Ok(SubmessageKind::INFO_DST),
            0x0f => Ok(SubmessageKind::INFO_REPLAY),
            0x12 => Ok(SubmessageKind::NACK_FRAG),
            0x13 => Ok(SubmessageKind::HEARTBEAT_FRAG),
            0x15 => Ok(SubmessageKind::DATA),
            0x16 => Ok(SubmessageKind::DATA_FRAG),
            _ => Err(Error::new(ErrorKind::InvalidData, "unknown SubmessageKind")),
        }
    }
}

impl<C: Context> Writable<C> for SubmessageKind {
    #[inline]
    fn write_to<'a, T: ?Sized + Writer<'a, C>>(&'a self, writer: &mut T) -> Result<()> {
        use self::SubmessageKind::*;
        let id = match self {
            PAD => 0x01,
            ACKNACK => 0x06,
            HEARTBEAT => 0x07,
            GAP => 0x08,
            INFO_TS => 0x09,
            INFO_SRC => 0x0c,
            INFO_REPLAY_IP4 => 0x0d,
            INFO_DST => 0x0e,
            INFO_REPLAY => 0x0f,
            NACK_FRAG => 0x12,
            HEARTBEAT_FRAG => 0x13,
            DATA => 0x15,
            DATA_FRAG => 0x16,
        };
        writer.write_u8(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    serialization_test!( type = SubmessageKind,
        {
            submessage_kind_pad,
            SubmessageKind::PAD,
            le = [0x01],
            be = [0x01]
        },
        {
            submessage_kind_acknack,
            SubmessageKind::ACKNACK,
            le = [0x06],
            be = [0x06]
        },
        {
            submessage_kind_heartbeat,
            SubmessageKind::HEARTBEAT,
            le = [0x07],
            be = [0x07]
        },
        {
            submessage_kind_gap,
            SubmessageKind::GAP,
            le = [0x08],
            be = [0x08]
        },
        {
            submessage_kind_info_ts,
            SubmessageKind::INFO_TS,
            le = [0x09],
            be = [0x09]
        },
        {
            submessage_kind_info_src,
            SubmessageKind::INFO_SRC,
            le = [0x0c],
            be = [0x0c]
        },
        {
            submessage_kind_info_replay_ip4,
            SubmessageKind::INFO_REPLAY_IP4,
            le = [0x0d],
            be = [0x0d]
        },
        {
            submessage_kind_info_dst,
            SubmessageKind::INFO_DST,
            le = [0x0e],
            be = [0x0e]
        },
        {
            submessage_kind_info_replay,
            SubmessageKind::INFO_REPLAY,
            le = [0x0f],
            be = [0x0f]
        },
        {
            submessage_kind_nack_frag,
            SubmessageKind::NACK_FRAG,
            le = [0x12],
            be = [0x12]
        },
        {
            submessage_kind_heartbeat_frag,
            SubmessageKind::HEARTBEAT_FRAG,
            le = [0x13],
            be = [0x13]
        },
        {
            submessage_kind_data,
            SubmessageKind::DATA,
            le = [0x15],
            be = [0x15]
        },
        {
            submessage_kind_data_frag,
            SubmessageKind::DATA_FRAG,
            le = [0x16],
            be = [0x16]
        }
    );
}
