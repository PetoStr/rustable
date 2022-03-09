#![allow(unused)]

use crate::medusa::MedusaAnswer;
use bitflags::bitflags;

pub const PROTOCOL_VERSION: u64 = 2;

#[cfg(debug_assertions)]
pub const DEFAULT_ANSWER: MedusaAnswer = MedusaAnswer::Ok;

#[cfg(not(debug_assertions))]
pub const DEFAULT_ANSWER: MedusaAnswer = MedusaAnswer::Deny;

pub const MEDUSA_COMM_KCLASSNAME_MAX: usize = 32 - 2;
pub const MEDUSA_COMM_ATTRNAME_MAX: usize = 32 - 5;
pub const MEDUSA_COMM_EVNAME_MAX: usize = 32 - 2;

pub const GREETING_NATIVE_BYTE_ORDER: u64 = 0x0000000066007e5a;
pub const GREETING_REVERSED_BYTE_ORDER: u64 = 0x5a7e006600000000;

pub const MEDUSA_COMM_AUTHREQUEST: u32 = 0x01;
pub const MEDUSA_COMM_KCLASSDEF: u32 = 0x02;
pub const MEDUSA_COMM_KCLASSUNDEF: u32 = 0x03;
pub const MEDUSA_COMM_EVTYPEDEF: u32 = 0x04;
pub const MEDUSA_COMM_EVTYPEUNDEF: u32 = 0x05;
pub const MEDUSA_COMM_FETCH_ANSWER: u32 = 0x08;
pub const MEDUSA_COMM_FETCH_ERROR: u32 = 0x09;
pub const MEDUSA_COMM_UPDATE_ANSWER: u32 = 0x0a;

pub const MEDUSA_COMM_FETCH_REQUEST: u64 = 0x88;
pub const MEDUSA_COMM_UPDATE_REQUEST: u64 = 0x8a;
pub const MEDUSA_COMM_AUTHANSWER: u64 = 0x81;

pub const ACTBIT_FLAGS_MASK: u16 = 0xc000;

pub const MEDUSA_EVTYPE_TRIGGEREDATSUBJECT: u16 = 0x0000;
pub const MEDUSA_EVTYPE_TRIGGEREDATOBJECT: u16 = 0x8000;

pub const MEDUSA_EVTYPE_TRIGGEREDBYSUBJECTBIT: u16 = 0x0000;
pub const MEDUSA_EVTYPE_TRIGGEREDBYOBJECTBIT: u16 = 0x4000;

pub const MEDUSA_ACCTYPE_TRIGGEREDATOBJECT: u16 =
    MEDUSA_EVTYPE_TRIGGEREDATOBJECT | MEDUSA_EVTYPE_TRIGGEREDBYOBJECTBIT;

bitflags! {
    #[derive(Default)]
    pub struct AttributeMods: u8 {
        const READ_ONLY = 0x80;
        const PRIMARY_KEY = 0x40;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeEndianness {
    Native = 0,
    Unused,
    Big,
    Little,
}

impl TryFrom<u8> for AttributeEndianness {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == AttributeEndianness::Native as u8 => Ok(AttributeEndianness::Native),
            x if x == AttributeEndianness::Unused as u8 => Ok(AttributeEndianness::Unused),
            x if x == AttributeEndianness::Big as u8 => Ok(AttributeEndianness::Big),
            x if x == AttributeEndianness::Little as u8 => Ok(AttributeEndianness::Little),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeDataType {
    End = 0,
    Unsigned,
    Signed,
    String,
    Bitmap,
    Bytes,
}

impl TryFrom<u8> for AttributeDataType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == AttributeDataType::End as u8 => Ok(AttributeDataType::End),
            x if x == AttributeDataType::Unsigned as u8 => Ok(AttributeDataType::Unsigned),
            x if x == AttributeDataType::Signed as u8 => Ok(AttributeDataType::Signed),
            x if x == AttributeDataType::String as u8 => Ok(AttributeDataType::String),
            x if x == AttributeDataType::Bitmap as u8 => Ok(AttributeDataType::Bitmap),
            x if x == AttributeDataType::Bytes as u8 => Ok(AttributeDataType::Bytes),
            _ => Err(()),
        }
    }
}

pub const MEDUSA_BITMAP_BLOCK_SIZE: usize = 1 << 3;
pub const MEDUSA_BITMAP_BLOCK_MASK: usize = MEDUSA_BITMAP_BLOCK_SIZE - 1;

pub const MEDUSA_VS_ATTR_NAME: &str = "vs";
pub const MEDUSA_VSR_ATTR_NAME: &str = "vsr";
pub const MEDUSA_VSW_ATTR_NAME: &str = "vsw";
pub const MEDUSA_VSS_ATTR_NAME: &str = "vss";
pub const MEDUSA_OACT_ATTR_NAME: &str = "med_oact";
pub const MEDUSA_SACT_ATTR_NAME: &str = "med_sact";
pub const MEDUSA_OCINFO_ATTR_NAME: &str = "o_cinfo";
