extern crate rtps;
extern crate time;

use self::rtps::message::submessage_header::{SubmessageKind, SubmessageHeader};

assert_ser_de!({
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
