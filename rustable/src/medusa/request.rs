use crate::medusa::constants::*;
use crate::medusa::{MedusaClass, MedusaEvtype};
use std::mem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestType {
    Fetch,
    Update,
}

#[derive(Clone, Copy, Debug)]
pub struct MedusaRequest<'a> {
    pub req_type: RequestType,
    pub class_id: u64,
    pub id: u64,
    pub data: &'a [u8],
}

impl<'a> MedusaRequest<'_> {
    // TODO big endian - check rust core to_le_bytes() implementation
    pub fn to_vec(self) -> Vec<u8> {
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
    pub request_id: u64,
    pub status: u16,
}

impl DecisionAnswer {
    // TODO big endian
    pub fn to_vec(self) -> [u8; 8 + std::mem::size_of::<Self>()] {
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
pub struct UpdateAnswer {
    pub class_id: u64,
    pub msg_seq: u64,
    pub status: i32,
}

impl UpdateAnswer {
    pub const fn size() -> usize {
        mem::size_of::<u64>() + mem::size_of::<u64>() + mem::size_of::<i32>()
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FetchAnswer {
    pub class_id: u64,
    pub msg_seq: u64,
    pub data: Vec<u8>,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedusaAnswer {
    Err = u16::MAX,
    Yes = 0,
    Deny,
    Skip,
    Ok,
}

#[derive(Clone, Debug)]
pub struct AuthRequestData {
    pub request_id: u64,
    pub evtype: MedusaEvtype,
    pub subject: MedusaClass,
    pub object: Option<MedusaClass>,
}
