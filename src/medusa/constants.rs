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

pub const MED_COMM_TYPE_END: u8 = 0x00;
pub const MED_COMM_TYPE_UNSIGNED: u8 = 0x01;
pub const MED_COMM_TYPE_SIGNED: u8 = 0x02;
pub const MED_COMM_TYPE_STRING: u8 = 0x03;
pub const MED_COMM_TYPE_BITMAP: u8 = 0x04;
pub const MED_COMM_TYPE_BYTES: u8 = 0x05;

pub const MEDUSA_BITMAP_BLOCK_SIZE: usize = 8;
pub const MEDUSA_BITMAP_BLOCK_MASK: usize = MEDUSA_BITMAP_BLOCK_SIZE - 1;

pub const MEDUSA_VS_ATTR_NAME: &str = "vs";
pub const MEDUSA_VSR_ATTR_NAME: &str = "vsr";
pub const MEDUSA_VSW_ATTR_NAME: &str = "vsw";
pub const MEDUSA_VSS_ATTR_NAME: &str = "vss";
pub const MEDUSA_OACT_ATTR_NAME: &str = "med_oact";
pub const MEDUSA_SACT_ATTR_NAME: &str = "med_sact";
