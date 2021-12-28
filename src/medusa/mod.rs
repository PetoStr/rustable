use crate::cstr_to_string;
use std::num::NonZeroU64;

pub mod context;
pub mod mcp;
mod parser;
mod reader;
mod writer;

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

const MEDUSA_BITMAP_BLOCK_SIZE: usize = 8;
const MEDUSA_BITMAP_BLOCK_MASK: usize = MEDUSA_BITMAP_BLOCK_SIZE - 1;

const MEDUSA_VS_ATTR_NAME: &str = "vs";
const MEDUSA_VSR_ATTR_NAME: &str = "vsr";
const MEDUSA_VSW_ATTR_NAME: &str = "vsw";
const MEDUSA_VSS_ATTR_NAME: &str = "vss";
const MEDUSA_OACT_ATTR_NAME: &str = "med_oact";
const MEDUSA_SACT_ATTR_NAME: &str = "med_sact";

#[derive(Default, Clone, Copy)]
pub struct MedusaClassHeader {
    id: u64,
    size: i16,
    name: [u8; MEDUSA_COMM_KCLASSNAME_MAX],
}

impl MedusaClassHeader {
    pub fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaClass {
    header: MedusaClassHeader,
    attributes: MedusaAttributes,
}

impl MedusaClass {
    pub fn add_vs(&mut self, n: usize) {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME);
        vs[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_vs(&mut self, n: usize) {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME);
        vs[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_vs(&mut self) {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME);
        vs.fill(0);
    }

    pub fn add_vs_read(&mut self, n: usize) {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME);
        vsr[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_vs_read(&mut self, n: usize) {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME);
        vsr[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_vs_read(&mut self) {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME);
        vsr.fill(0);
    }

    pub fn add_vs_write(&mut self, n: usize) {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME);
        vsw[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_vs_write(&mut self, n: usize) {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME);
        vsw[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_vs_write(&mut self) {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME);
        vsw.fill(0);
    }

    pub fn add_vs_see(&mut self, n: usize) {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME);
        vss[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_vs_see(&mut self, n: usize) {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME);
        vss[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_vs_see(&mut self) {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME);
        vss.fill(0);
    }

    pub fn add_object_act(&mut self, n: usize) {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME);
        oact[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_object_act(&mut self, n: usize) {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME);
        oact[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_object_act(&mut self) {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME);
        oact.fill(0);
    }

    pub fn add_subject_act(&mut self, n: usize) {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME);
        sact[n / MEDUSA_BITMAP_BLOCK_SIZE] |= 1 << (n & MEDUSA_BITMAP_BLOCK_MASK);
    }

    pub fn remove_subject_act(&mut self, n: usize) {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME);
        sact[n / MEDUSA_BITMAP_BLOCK_SIZE] &= !(1 << (n & MEDUSA_BITMAP_BLOCK_MASK));
    }

    pub fn clear_subject_act(&mut self) {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME);
        sact.fill(0);
    }

    // TODO set_attribute_{unsigned,signed,string,bitmap,bytes}
    pub fn set_attribute(&mut self, attr_name: &str, data: Vec<u8>) {
        self.attributes.set(attr_name, data);
    }

    pub fn get_attribute(&self, attr_name: &str) -> &[u8] {
        self.attributes.get(attr_name)
    }

    fn pack_attributes(&self) -> Vec<u8> {
        let mut res = vec![0; self.header.size as usize];
        self.attributes.pack(&mut res);
        res
    }
}

#[derive(Default, Clone, Copy)]
pub struct MedusaAttributeHeader {
    offset: i16,
    length: i16, // size in bytes
    r#type: u8,  // i think this should be u8 and not i8 because of bit masks
    name: [u8; MEDUSA_COMM_ATTRNAME_MAX],
}

impl MedusaAttributeHeader {
    pub fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaAttribute {
    header: MedusaAttributeHeader,
    data: Vec<u8>,
}

impl MedusaAttribute {
    fn pack_data(&self) -> Vec<u8> {
        self.data
            .iter()
            .copied()
            .chain(std::iter::once(0).cycle())
            .take(self.header.length as usize)
            .collect()
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct MedusaEvtypeHeader {
    evid: u64,
    size: u16,
    actbit: u16,
    //ev_kclass: [u64; 2],
    ev_sub: u64,
    ev_obj: Option<NonZeroU64>,
    name: [u8; MEDUSA_COMM_EVNAME_MAX],
    ev_name: [[u8; MEDUSA_COMM_ATTRNAME_MAX]; 2],
}

impl MedusaEvtypeHeader {
    pub fn name(&self) -> String {
        cstr_to_string(&self.name)
    }
}

#[derive(Default, Clone)]
pub struct MedusaEvtype {
    header: MedusaEvtypeHeader,
    attributes: MedusaAttributes,
}

impl MedusaEvtype {
    pub fn get_attribute(&self, attr_name: &str) -> &[u8] {
        self.attributes.get(attr_name)
    }

    pub fn name(&self) -> String {
        self.header.name()
    }
}

#[derive(Default, Clone)]
struct MedusaAttributes {
    inner: Vec<MedusaAttribute>,
}

impl MedusaAttributes {
    fn set(&mut self, attr_name: &str, data: Vec<u8>) {
        let mut attr = self
            .inner
            .iter_mut()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does, maybe LinkedHashMap?
            .unwrap_or_else(|| panic!("no attribute {}", attr_name));

        attr.data = data;
    }

    fn get(&self, attr_name: &str) -> &[u8] {
        let attr = self
            .inner
            .iter()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does, maybe LinkedHashMap?
            .unwrap_or_else(|| panic!("no attribute {}", attr_name));

        &attr.data
    }

    fn get_mut(&mut self, attr_name: &str) -> &mut [u8] {
        let attr = self
            .inner
            .iter_mut()
            .find(|x| x.header.name() == attr_name) // TODO HashMap, but preserve order like Vec does, maybe LinkedHashMap?
            .unwrap_or_else(|| panic!("no attribute {}", attr_name));

        &mut attr.data
    }

    fn set_from_raw(&mut self, raw_data: &[u8]) {
        for attr in self.inner.iter_mut() {
            let offset = attr.header.offset as usize;
            let length = attr.header.length as usize;
            attr.data = raw_data[offset..][..length].to_vec();
        }
    }

    fn pack(&self, res: &mut [u8]) {
        for attribute in &self.inner {
            let data = attribute.pack_data();

            // TODO make faster, `slice::copy_from_slice()` did not work
            for i in 0..attribute.header.length as usize {
                res[attribute.header.offset as usize + i] = data[i];
            }
        }
    }

    fn push(&mut self, attribute: MedusaAttribute) {
        self.inner.push(attribute);
    }
}

#[derive(Clone, Copy, Debug)]
enum RequestType {
    Fetch,
    Update,
}

#[derive(Clone, Copy, Debug)]
pub struct MedusaRequest<'a> {
    req_type: RequestType,
    class_id: u64,
    id: u64,
    data: &'a [u8],
}

impl<'a> MedusaRequest<'_> {
    // TODO big endian - check rust core to_le_bytes() implementation
    fn to_vec(self) -> Vec<u8> {
        let request = match self.req_type {
            RequestType::Fetch => MEDUSA_COMM_FETCH_REQUEST.to_le_bytes(),
            RequestType::Update => MEDUSA_COMM_UPDATE_REQUEST.to_le_bytes(),
        };
        let class_id = self.class_id.to_le_bytes();
        let id = self.id.to_le_bytes();
        request
            .iter()
            .copied()
            .chain(class_id.iter().copied())
            .chain(id.iter().copied())
            .chain(self.data.iter().copied())
            .collect::<Vec<u8>>()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct DecisionAnswer {
    request_id: u64,
    status: u16,
}

impl DecisionAnswer {
    // TODO big endian
    fn to_vec(self) -> [u8; 8 + std::mem::size_of::<Self>()] {
        let answer = MEDUSA_COMM_AUTHANSWER.to_le_bytes();
        let request = self.request_id.to_le_bytes();
        let status = self.status.to_le_bytes();
        answer
            .iter()
            .copied()
            .chain(request.iter().copied())
            .chain(status.iter().copied())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap()
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
#[repr(packed)]
pub struct UpdateAnswer {
    class_id: u64,
    msg_seq: u64,
    status: i32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FetchAnswer {
    class_id: u64,
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
    pub request_id: u64,
    pub evtype: MedusaEvtype,
    pub subject: MedusaClass,
    pub object: Option<MedusaClass>,
}
