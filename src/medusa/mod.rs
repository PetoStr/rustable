use crate::cstr_to_string;

pub mod mcp;
pub mod parser;
pub mod writer;

type Command = u32;

const MEDUSA_COMM_KCLASSNAME_MAX: usize = 32 - 2;
const MEDUSA_COMM_ATTRNAME_MAX: usize = 32 - 5;
const MEDUSA_COMM_EVNAME_MAX: usize = 32 - 2;

const GREETING_NATIVE_BYTE_ORDER: u64 = 0x0000000066007e5a;
const GREETING_REVERSED_BYTE_ORDER: u64 = 0x5a7e006600000000;

const MEDUSA_COMM_AUTHREQUEST: u32 = 0x01;
const MEDUSA_COMM_KCLASSDEF: u32 = 0x02;
const MEDUSA_COMM_KCLASSUNDEF: u32 = 0x03;
const MEDUSA_COMM_EVTYPEDEF: u32 = 0x04;
const MEDUSA_COMM_EVTYPEUNDEF: u32 = 0x05;
const MEDUSA_COMM_FETCH_ANSWER: u32 = 0x08;
const MEDUSA_COMM_FETCH_ERROR: u32 = 0x09;
const MEDUSA_COMM_UPDATE_ANSWER: u32 = 0x0a;

const MEDUSA_COMM_FETCH_REQUEST: u64 = 0x88;
const MEDUSA_COMM_UPDATE_REQUEST: u64 = 0x8a;
const MEDUSA_COMM_AUTHANSWER: u64 = 0x81;

const MED_COMM_TYPE_END: u8 = 0x00;
const _MED_COMM_TYPE_UNSIGNED: u8 = 0x01;
const _MED_COMM_TYPE_SIGNED: u8 = 0x02;
const _MED_COMM_TYPE_STRING: u8 = 0x03;
const _MED_COMM_TYPE_BITMAP: u8 = 0x04;
const _MED_COMM_TYPE_BYTES: u8 = 0x05;

// TODO refactor these weird names
#[derive(Default, Clone)]
pub struct MedusaCommKClassHeader {
    kclassid: u64,
    size: i16,
    name: [u8; MEDUSA_COMM_KCLASSNAME_MAX],
}

impl MedusaCommKClassHeader {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommKClass {
    header: MedusaCommKClassHeader,
    attributes: Vec<MedusaCommAttribute>,
}

impl MedusaCommKClass {
    pub fn set_attribute(&mut self, attr_name: &str, data: Vec<u8>) {
        let name = self.header.name();
        let mut attr = self
            .attributes
            .iter_mut()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does
            .unwrap_or_else(|| panic!("{} has no attribute {}", name, attr_name));

        attr.data = data;
    }

    pub fn get_attribute(&mut self, attr_name: &str) -> &[u8] {
        let name = self.header.name();
        let attr = self
            .attributes
            .iter()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does
            .unwrap_or_else(|| panic!("{} has no attribute {}", name, attr_name));

        &attr.data
    }

    pub fn pack_attributes(&self) -> Vec<u8> {
        let mut res = vec![0; self.header.size as usize];

        for attribute in &self.attributes {
            let data = attribute.pack_data();

            // TODO make faster, `slice::copy_from_slice()` did not work
            for i in 0..attribute.header.length as usize {
                res[attribute.header.offset as usize + i] = data[i];
            }
        }

        res
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommAttributeHeader {
    offset: i16,
    length: i16, // size in bytes
    r#type: u8,  // i think this should be u8 and not i8 because of bit masks
    name: [u8; MEDUSA_COMM_ATTRNAME_MAX],
}

impl MedusaCommAttributeHeader {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaCommAttribute {
    header: MedusaCommAttributeHeader,
    data: Vec<u8>,
}

impl MedusaCommAttribute {
    fn pack_data(&self) -> Vec<u8> {
        self.data
            .iter()
            .copied()
            .chain(std::iter::once(0).cycle())
            .take(self.header.length as usize)
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct MedusaCommEvtype {
    evid: u64,
    size: u16,
    actbit: u16,
    //ev_kclass: [u64; 2],
    ev_sub: u64,
    ev_obj: u64,
    name: [u8; MEDUSA_COMM_EVNAME_MAX],
    ev_name: [[u8; MEDUSA_COMM_ATTRNAME_MAX]; 2],
    // TODO attributes
}

impl MedusaCommEvtype {
    fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RequestType {
    Fetch,
    Update,
}

#[derive(Clone, Copy, Debug)]
pub struct MedusaCommRequest<'a> {
    req_type: RequestType,
    kclassid: u64,
    id: u64,
    data: &'a [u8],
}

impl<'a> MedusaCommRequest<'_> {
    // TODO big endian - check rust core to_le_bytes() implementation
    // consider chaning the function name
    fn as_bytes(&self) -> Vec<u8> {
        let update_b = match self.req_type {
            RequestType::Fetch => MEDUSA_COMM_FETCH_REQUEST.to_le_bytes(),
            RequestType::Update => MEDUSA_COMM_UPDATE_REQUEST.to_le_bytes(),
        };
        let kclassid_b = self.kclassid.to_le_bytes();
        let id_b = self.id.to_le_bytes();
        update_b
            .iter()
            .copied()
            .chain(kclassid_b.iter().copied())
            .chain(id_b.iter().copied())
            .chain(self.data.iter().copied())
            .collect::<Vec<u8>>()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct DecisionAnswer {
    request_id: u64,
    result: u16,
}

impl DecisionAnswer {
    // TODO big endian
    // TODO as_bytes adds additional data -> change name?
    fn as_bytes(&self) -> [u8; 8 + std::mem::size_of::<Self>()] {
        let answer_b = MEDUSA_COMM_AUTHANSWER.to_le_bytes();
        let request_b = self.request_id.to_le_bytes();
        let result_b = self.result.to_le_bytes();
        answer_b
            .iter()
            .copied()
            .chain(request_b.iter().copied())
            .chain(result_b.iter().copied())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct UpdateAnswer {
    kclassid: u64,
    msg_seq: u64,
    ans_res: i32,
}

#[derive(Clone, Debug)]
pub struct FetchAnswer {
    kclassid: u64,
    msg_seq: u64,
    data: Vec<u8>,
}

#[repr(u16)]
pub enum MedusaAnswer {
    Err = u16::MAX,
    Yes = 0,
    No,
    Skip,
    Ok,
}

#[derive(Clone)]
pub struct AuthRequestData {
    // TODO
    pub request_id: u64,
    pub event: String,
    pub subject: u64,
    //pub object: MedusaCommKClass,
}
