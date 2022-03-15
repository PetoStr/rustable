use crate::bitmap;
use crate::medusa::constants::*;
use crate::medusa::{AttributeBytes, AttributeError, MedusaAttributes};
use std::{fmt, mem};

#[derive(Default, Clone)]
pub struct MedusaClassHeader {
    pub(crate) id: u64,
    pub(crate) size: i16,
    pub(crate) name: String,
}

impl MedusaClassHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn size() -> usize {
        mem::size_of::<u64>() + mem::size_of::<i16>() + MEDUSA_COMM_KCLASSNAME_MAX
    }
}

impl fmt::Debug for MedusaClassHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MedusaClassHeader")
            .field("id", &format_args!("0x{:x}", self.id))
            .field("size", &self.size)
            .field("name", &format_args!("\"{}\"", self.name()))
            .finish()
    }
}

#[derive(Default, Clone, Debug)]
pub struct MedusaClass {
    pub(crate) header: MedusaClassHeader,
    pub(crate) attributes: MedusaAttributes,
}

impl MedusaClass {
    pub fn add_vs(&mut self, n: usize) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::set_bit(vs, n);

        Ok(())
    }

    pub fn remove_vs(&mut self, n: usize) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::clear_bit(vs, n);

        Ok(())
    }

    pub fn set_vs(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VS_ATTR_NAME, vs)
    }

    pub fn clear_vs(&mut self) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::clear_all(vs);

        Ok(())
    }

    pub fn add_vs_read(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::set_bit(vsr, n);

        Ok(())
    }

    pub fn remove_vs_read(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::clear_bit(vsr, n);

        Ok(())
    }

    pub fn set_vs_read(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSR_ATTR_NAME, vs)
    }

    pub fn clear_vs_read(&mut self) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::clear_all(vsr);

        Ok(())
    }

    pub fn add_vs_write(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::set_bit(vsw, n);

        Ok(())
    }

    pub fn remove_vs_write(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::clear_bit(vsw, n);

        Ok(())
    }

    pub fn set_vs_write(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSW_ATTR_NAME, vs)
    }

    pub fn clear_vs_write(&mut self) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::clear_all(vsw);

        Ok(())
    }

    pub fn add_vs_see(&mut self, n: usize) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::set_bit(vss, n);

        Ok(())
    }

    pub fn remove_vs_see(&mut self, n: usize) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::clear_bit(vss, n);

        Ok(())
    }

    pub fn set_vs_see(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSS_ATTR_NAME, vs)
    }

    pub fn clear_vs_see(&mut self) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::clear_all(vss);

        Ok(())
    }

    pub fn add_object_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::set_bit(oact, n);

        Ok(())
    }

    pub fn remove_object_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::clear_bit(oact, n);

        Ok(())
    }

    pub fn clear_object_act(&mut self) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::clear_all(oact);

        Ok(())
    }

    pub fn add_subject_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::set_bit(sact, n);

        Ok(())
    }

    pub fn remove_subject_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::clear_bit(sact, n);

        Ok(())
    }

    pub fn clear_subject_act(&mut self) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::clear_all(sact);

        Ok(())
    }

    pub fn set_object_cinfo(&mut self, cinfo: usize) -> Result<(), AttributeError> {
        self.set_attribute(MEDUSA_OCINFO_ATTR_NAME, cinfo)
    }

    pub fn get_object_cinfo(&self) -> Option<usize> {
        self.get_attribute::<usize>(MEDUSA_OCINFO_ATTR_NAME)
    }

    pub fn get_vs(&self) -> Option<&[u8]> {
        self.attributes.get(MEDUSA_VS_ATTR_NAME)
    }

    pub fn set_attribute<T: AttributeBytes>(
        &mut self,
        attr_name: &str,
        data: T,
    ) -> Result<(), AttributeError> {
        self.attributes.set(attr_name, data.to_bytes())
    }

    pub fn get_attribute<T: AttributeBytes>(&self, attr_name: &str) -> Option<T> {
        Some(T::from_bytes(self.attributes.get(attr_name)?.to_vec()))
    }

    pub fn pack_attributes(&self) -> Vec<u8> {
        let mut res = vec![0; self.header.size as usize];
        self.attributes.pack(&mut res);
        res
    }
}
