use crate::cstr_to_string;
use crate::medusa::constants::*;
use crate::medusa::AttributeError;
use hashlink::LinkedHashMap;
use std::{fmt, mem};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MedusaAttributeHeader {
    pub(crate) offset: i16,
    pub(crate) length: i16, // size in bytes
    pub(crate) mods: AttributeMods,
    pub(crate) endianness: AttributeEndianness,
    pub(crate) data_type: AttributeDataType,
    pub(crate) name: String,
}

impl MedusaAttributeHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_read_only(&self) -> bool {
        self.mods.contains(AttributeMods::READ_ONLY)
    }

    pub const fn size() -> usize {
        mem::size_of::<i16>()
            + mem::size_of::<i16>()
            + mem::size_of::<u8>()
            + MEDUSA_COMM_ATTRNAME_MAX
    }
}

#[derive(Clone)]
pub struct MedusaAttribute {
    pub(crate) header: MedusaAttributeHeader,
    pub(crate) data: Vec<u8>,
}

impl fmt::Debug for MedusaAttribute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = if self.header.data_type == AttributeDataType::Unsigned {
            let data = self.data[..self.header.length as usize].to_vec();
            if self.header.length == 1 {
                format!("(u8) {}", u8::from_le_bytes(data.try_into().unwrap()))
            } else if self.header.length == 2 {
                format!("(u16) {}", u16::from_le_bytes(data.try_into().unwrap()))
            } else if self.header.length == 4 {
                format!("(u32) {}", u32::from_le_bytes(data.try_into().unwrap()))
            } else {
                // assuming length == 8
                format!(
                    "(u64) {}",
                    u64::from_le_bytes(data.try_into().unwrap_or_default())
                )
            }
        } else if self.header.data_type == AttributeDataType::Signed {
            let data = self.data.clone();
            if self.header.length == 1 {
                format!("(i8) {}", i8::from_le_bytes(data.try_into().unwrap()))
            } else if self.header.length == 2 {
                format!("(i16) {}", i16::from_le_bytes(data.try_into().unwrap()))
            } else if self.header.length == 4 {
                format!("(i32) {}", i32::from_le_bytes(data.try_into().unwrap()))
            } else {
                // assuming length == 8
                format!(
                    "(i64) {}",
                    i64::from_le_bytes(data.try_into().unwrap_or_default())
                )
            }
        } else if self.header.data_type == AttributeDataType::String {
            cstr_to_string(&self.data)
        } else if self.header.data_type == AttributeDataType::Bitmap {
            format!("(bitmap) {:?}", &self.data)
        } else if self.header.data_type == AttributeDataType::Bytes {
            format!("(bytes) {:?}", &self.data)
        } else {
            format!("(unknown type) {:?}", &self.data)
        };

        f.debug_struct("MedusaAttribute")
            .field("header", &self.header)
            .field("data", &data)
            .finish()
    }
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

pub trait AttributeBytes {
    fn to_bytes(self) -> Vec<u8>;
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

macro_rules! attribute_bytes_impl {
    ($($t:ty)*) => ($(
        impl AttributeBytes for $t {
            fn to_bytes(self) -> Vec<u8> {
                self.to_le_bytes().to_vec()
            }

            fn from_bytes(bytes: Vec<u8>) -> $t {
                <$t>::from_le_bytes(bytes.try_into().unwrap())
            }
        }
    )*)
}

attribute_bytes_impl! { u8 u16 u32 u64 i8 i16 i32 i64 usize }

impl AttributeBytes for String {
    fn to_bytes(self) -> Vec<u8> {
        let mut vec = self.into_bytes();
        vec.push(0);

        vec
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        cstr_to_string(&bytes)
    }
}

impl AttributeBytes for Vec<u8> {
    fn to_bytes(self) -> Vec<u8> {
        self
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        bytes
    }
}

#[derive(Default, Clone, Debug)]
pub struct MedusaAttributes {
    inner: LinkedHashMap<String, MedusaAttribute>,
}

impl MedusaAttributes {
    pub fn set(&mut self, attr_name: &str, data: Vec<u8>) -> Result<(), AttributeError> {
        let attr = self
            .inner
            .get_mut(attr_name)
            .ok_or_else(|| AttributeError::UnknownAttribute(attr_name.to_owned()))?;

        if attr.header.is_read_only() {
            return Err(AttributeError::ModifyReadOnlyError(attr_name.to_owned()));
        }

        attr.data = data;

        Ok(())
    }

    pub fn get(&self, attr_name: &str) -> Option<&[u8]> {
        self.inner.get(attr_name).map(|x| &x.data[..])
    }

    pub fn get_mut(&mut self, attr_name: &str) -> Result<&mut [u8], AttributeError> {
        let attr = self
            .inner
            .get_mut(attr_name)
            .ok_or_else(|| AttributeError::UnknownAttribute(attr_name.to_owned()))?;

        Ok(&mut attr.data)
    }

    pub fn set_from_raw(&mut self, raw_data: &[u8]) {
        for attr in self.inner.values_mut() {
            let offset = attr.header.offset as usize;
            let length = attr.header.length as usize;
            attr.data = raw_data[offset..][..length].to_vec();
        }
    }

    pub fn pack(&self, res: &mut [u8]) {
        for attribute in self.inner.values() {
            let data = attribute.pack_data();

            // TODO make faster, `slice::copy_from_slice()` did not work
            for i in 0..attribute.header.length as usize {
                res[attribute.header.offset as usize + i] = data[i];
            }
        }
    }

    pub fn push(&mut self, attribute: MedusaAttribute) {
        self.inner.insert(attribute.header.name.clone(), attribute);
    }
}
