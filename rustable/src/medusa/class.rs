use crate::bitmap;
use crate::medusa::constants::*;
use crate::medusa::space::VirtualSpace;
use crate::medusa::{
    AttributeBytes, AttributeError, Context, MedusaAttributes, MedusaEvtype, Monitoring, Node,
};
use std::sync::atomic::Ordering;
use std::sync::Arc;
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

/// Entity which may represent either subject or object.
#[derive(Default, Clone, Debug)]
pub struct MedusaClass {
    pub(crate) header: MedusaClassHeader,
    pub(crate) attributes: MedusaAttributes,
}

impl MedusaClass {
    /// Manually enters this entity into tree.
    pub async fn enter_tree(
        &mut self,
        ctx: &Context,
        evtype: &MedusaEvtype,
        primary_tree: &str,
        path: &str,
    ) {
        assert!(path.starts_with('/'));

        let tree = ctx
            .config()
            .tree_by_name(primary_tree)
            .unwrap_or_else(|| panic!("primary tree `{}` not found", primary_tree));

        let mut node = tree.root();
        let mut recursive_parent = if node.is_recursive() {
            Some(node)
        } else {
            None
        };
        let mut recursed = false;
        if path != "/" {
            // skip empty string caused by leading '/'
            for part in path.split_terminator('/').skip(1) {
                let child = node.child_by_path(part);
                match child {
                    Some(ch) => {
                        if ch.is_recursive() {
                            recursive_parent = Some(ch);
                        }
                        node = ch;
                    }
                    None => {
                        node = recursive_parent.expect("{part} not covered by tree");
                        recursed = true;
                    }
                }
            }
        }

        println!(
            "{}: \"{}\" -> \"{}\"{}",
            evtype.header.name,
            path,
            node.path(),
            if recursed { " (recursion)" } else { "" }
        );

        self.enter_tree_with_node(ctx, evtype, node, recursed).await;
    }

    /// Manually enters this entity into specific node.
    pub async fn enter_tree_with_node(
        &mut self,
        ctx: &Context,
        evtype: &MedusaEvtype,
        node: &Arc<Node>,
        recursed: bool,
    ) {
        let cinfo = Arc::as_ptr(node) as usize;

        self.set_access_types(node.virtual_space());

        let covered_events = ctx.config().covered_events_mask.load(Ordering::SeqCst);
        let _ = self.set_attribute::<u64>(MEDUSA_OACT_ATTR_NAME, covered_events);
        let _ = self.set_attribute::<u64>(MEDUSA_SACT_ATTR_NAME, covered_events);

        // remove the monitoring bit if this is not a parent
        if recursed || !(node.has_children() && evtype.header.monitoring == Monitoring::Object) {
            let _ = self.remove_object_act(evtype.header.monitoring_bit as usize);
            let _ = self.remove_subject_act(evtype.header.monitoring_bit as usize);
        }

        self.set_object_cinfo(cinfo).unwrap();

        self.update(ctx).await;
    }

    /// Copies access types from `vs`.
    pub fn set_access_types(&mut self, vs: &VirtualSpace) {
        let _ = self.set_vs(vs.to_at_bytes(AccessType::Member));
        let _ = self.set_vs_read(vs.to_at_bytes(AccessType::Read));
        let _ = self.set_vs_write(vs.to_at_bytes(AccessType::Write));
        let _ = self.set_vs_see(vs.to_at_bytes(AccessType::See));
    }

    /// Performs `update` request on this entity.
    pub async fn update(&self, ctx: &Context) -> i32 {
        let data = self.pack_attributes();
        let id = self.header.id;

        let answer = ctx.update_request(id, &data).await;

        answer.status
    }

    /// Performs `fetch` request. In case that the returned object has not yet been registered,
    /// `None` is returned.
    pub async fn fetch(&self, ctx: &Context) -> Option<MedusaClass> {
        let data = self.pack_attributes();
        let id = self.header.id;

        let answer = ctx.fetch_request(id, &data).await;

        let mut object = ctx.empty_class_from_id(&answer.class_id)?;
        object.attributes.set_from_raw(&answer.data);

        Some(object)
    }

    /// Adds virtual space.
    pub fn add_vs(&mut self, n: usize) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::set_bit(vs, n);

        Ok(())
    }

    /// Removes virtual space.
    pub fn remove_vs(&mut self, n: usize) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::clear_bit(vs, n);

        Ok(())
    }

    /// Sets virtual spaces.
    pub fn set_vs(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VS_ATTR_NAME, vs)
    }

    /// Clears virtual spaces.
    pub fn clear_vs(&mut self) -> Result<(), AttributeError> {
        let vs = self.attributes.get_mut(MEDUSA_VS_ATTR_NAME)?;
        bitmap::clear_all(vs);

        Ok(())
    }

    /// Adds virtual space for `read` access type.
    pub fn add_vs_read(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::set_bit(vsr, n);

        Ok(())
    }

    /// Removes virtual space for `read` access type.
    pub fn remove_vs_read(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::clear_bit(vsr, n);

        Ok(())
    }

    /// Sets virtual spaces for `read` access type.
    pub fn set_vs_read(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSR_ATTR_NAME, vs)
    }

    /// Clears virtual spaces for `read` access type.
    pub fn clear_vs_read(&mut self) -> Result<(), AttributeError> {
        let vsr = self.attributes.get_mut(MEDUSA_VSR_ATTR_NAME)?;
        bitmap::clear_all(vsr);

        Ok(())
    }

    /// Adds virtual space for `write` access type.
    pub fn add_vs_write(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::set_bit(vsw, n);

        Ok(())
    }

    /// Removes virtual space for `write` access type.
    pub fn remove_vs_write(&mut self, n: usize) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::clear_bit(vsw, n);

        Ok(())
    }

    /// Sets virtual spaces for `write` access type.
    pub fn set_vs_write(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSW_ATTR_NAME, vs)
    }

    /// Clears virtual spaces for `write` access type.
    pub fn clear_vs_write(&mut self) -> Result<(), AttributeError> {
        let vsw = self.attributes.get_mut(MEDUSA_VSW_ATTR_NAME)?;
        bitmap::clear_all(vsw);

        Ok(())
    }

    /// Adds virtual space for `see` access type.
    pub fn add_vs_see(&mut self, n: usize) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::set_bit(vss, n);

        Ok(())
    }

    /// Removes virtual space for `see` access type.
    pub fn remove_vs_see(&mut self, n: usize) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::clear_bit(vss, n);

        Ok(())
    }

    /// Sets virtual spaces for `see` access type.
    pub fn set_vs_see(&mut self, vs: Vec<u8>) -> Result<(), AttributeError> {
        self.attributes.set(MEDUSA_VSS_ATTR_NAME, vs)
    }

    /// Clears virtual spaces for `see` access type.
    pub fn clear_vs_see(&mut self) -> Result<(), AttributeError> {
        let vss = self.attributes.get_mut(MEDUSA_VSS_ATTR_NAME)?;
        bitmap::clear_all(vss);

        Ok(())
    }

    /// Adds object monitoring bit.
    pub fn add_object_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::set_bit(oact, n);

        Ok(())
    }

    /// Removes object monitoring bit.
    pub fn remove_object_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::clear_bit(oact, n);

        Ok(())
    }

    /// Clears object monitoring bits.
    pub fn clear_object_act(&mut self) -> Result<(), AttributeError> {
        let oact = self.attributes.get_mut(MEDUSA_OACT_ATTR_NAME)?;
        bitmap::clear_all(oact);

        Ok(())
    }

    /// Adds subject monitoring bit.
    pub fn add_subject_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::set_bit(sact, n);

        Ok(())
    }

    /// Removes subject monitoring bit.
    pub fn remove_subject_act(&mut self, n: usize) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::clear_bit(sact, n);

        Ok(())
    }

    /// Clears subject monitoring bits.
    pub fn clear_subject_act(&mut self) -> Result<(), AttributeError> {
        let sact = self.attributes.get_mut(MEDUSA_SACT_ATTR_NAME)?;
        bitmap::clear_all(sact);

        Ok(())
    }

    /// Sets `cinfo` attribute.
    pub fn set_object_cinfo(&mut self, cinfo: usize) -> Result<(), AttributeError> {
        self.set_attribute(MEDUSA_OCINFO_ATTR_NAME, cinfo)
    }

    /// Returns content of `cinfo` attribute.
    pub fn get_object_cinfo(&self) -> Result<usize, AttributeError> {
        self.get_attribute::<usize>(MEDUSA_OCINFO_ATTR_NAME)
    }

    /// Returns content of `vs` attribute.
    pub fn get_vs(&self) -> Result<&[u8], AttributeError> {
        self.attributes.get(MEDUSA_VS_ATTR_NAME)
    }

    /// Sets attribute `attr_name` to value `data` of type `T`.
    pub fn set_attribute<T: AttributeBytes>(
        &mut self,
        attr_name: &str,
        data: T,
    ) -> Result<(), AttributeError> {
        self.attributes.set(attr_name, data.to_bytes())
    }

    /// Returns value of attribute `attr_name` with type `T`.
    pub fn get_attribute<T: AttributeBytes>(&self, attr_name: &str) -> Result<T, AttributeError> {
        Ok(T::from_bytes(self.attributes.get(attr_name)?.to_vec()))
    }

    /// Packs attributes into vector of bytes.
    pub fn pack_attributes(&self) -> Vec<u8> {
        let mut res = vec![0; self.header.size as usize];
        self.attributes.pack(&mut res);
        res
    }
}
