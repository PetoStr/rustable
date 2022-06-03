use crate::medusa::constants::*;
use crate::medusa::{MedusaClass, MedusaEvtype};
use std::mem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestType {
    /// Represents `fetch` request.
    Fetch,

    /// Represents `update` request.
    Update,
}

#[derive(Clone, Copy, Debug)]
pub struct MedusaRequest<'a> {
    /// Type of this request.
    pub req_type: RequestType,

    /// Which class should handle the request on security module's side.
    pub class_id: u64,

    /// Unique identification of this request.
    pub id: u64,

    /// Provided data with the request.
    pub data: &'a [u8],
}

impl<'a> MedusaRequest<'_> {
    // TODO big endian - check rust core to_le_bytes() implementation
    /// Converts `MedusaRequest` into byte vector.
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
    /// Identification of the authorization request.
    pub request_id: u64,

    /// Final verdict of the authorization request.
    pub status: u16,
}

impl DecisionAnswer {
    // TODO big endian
    /// Converts `DecisionAnswer` into byte array.
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
    /// Identification of the updated class.
    pub class_id: u64,

    /// Identification which is used to distinguish which answer belongs to which update request.
    pub msg_seq: u64,

    /// Verdict of the update request.
    pub status: i32,
}

impl UpdateAnswer {
    /// Returns expected size if this structure was packed.
    pub const fn size() -> usize {
        mem::size_of::<u64>() + mem::size_of::<u64>() + mem::size_of::<i32>()
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct FetchAnswer {
    /// Identification of class which should be fetched.
    pub class_id: u64,

    /// Identification which is used to distinguish which answer belongs to which fetch request.
    pub msg_seq: u64,

    /// Data returned from the security module.
    pub data: Vec<u8>,
}

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedusaAnswer {
    /// Indicates that an error has occurred during authorization request and security module
    /// should decide what to do next.
    Err = u16::MAX,
    Yes = 0,
    /// Indicates that the operation should be denied.
    Deny,
    Skip,
    /// Indicates that the operation should be allowed.
    Allow,
}

#[derive(Clone, Debug)]
pub struct AuthRequestData {
    /// Unique identification of this request.
    pub request_id: u64,

    /// Event.
    pub evtype: MedusaEvtype,

    /// Subject.
    pub subject: MedusaClass,

    /// Object which may not be present for certain events.
    pub object: Option<MedusaClass>,
}
