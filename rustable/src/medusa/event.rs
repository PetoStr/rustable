use crate::medusa::constants::*;
use crate::medusa::error::AttributeError;
use crate::medusa::MedusaAttributes;
use std::mem;
use std::num::NonZeroU64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Monitoring {
    Subject,
    Object,
}

impl Default for Monitoring {
    fn default() -> Self {
        Self::Subject
    }
}

#[derive(Debug, Default, Clone)]
pub struct MedusaEvtypeHeader {
    pub(crate) evid: u64,
    pub(crate) size: u16,

    //actbit: u16,
    pub(crate) monitoring: Monitoring,
    pub(crate) monitoring_bit: u16,

    //ev_kclass: [u64; 2],
    pub(crate) ev_sub: u64,
    pub(crate) ev_obj: Option<NonZeroU64>,

    pub(crate) name: String,
    pub(crate) ev_name: [String; 2],
}

impl MedusaEvtypeHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn size() -> usize {
        mem::size_of::<u64>()
            + mem::size_of::<u16>()
            + mem::size_of::<u16>()
            + mem::size_of::<u64>()
            + mem::size_of::<u64>()
            + MEDUSA_COMM_EVNAME_MAX
            + 2 * MEDUSA_COMM_ATTRNAME_MAX
    }
}

/// Event, such as `getfile` or `getprocess`.
#[derive(Default, Clone, Debug)]
pub struct MedusaEvtype {
    pub(crate) header: MedusaEvtypeHeader,
    pub(crate) attributes: MedusaAttributes,
}

impl MedusaEvtype {
    /// Returns slice of bytes for attribute `attr_name`.
    pub fn get_attribute(&self, attr_name: &str) -> Result<&[u8], AttributeError> {
        self.attributes.get(attr_name)
    }

    /// Returns name of this event.
    pub fn name(&self) -> &str {
        self.header.name()
    }
}
