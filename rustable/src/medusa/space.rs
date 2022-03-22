use crate::bitmap;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Space {
    All,
    ByName(&'static str),
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SpaceDef {
    id_cn: usize,
    name_to_id: HashMap<&'static str, usize>,
    id_to_name: HashMap<usize, &'static str>,
}

impl SpaceDef {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn define_space(&mut self, space: Space) {
        if let Space::ByName(name) = space {
            if self.name_to_id.contains_key(name) {
                return;
            }

            let id = self.new_id();
            self.insert_space(name, id);
        }
    }

    pub(crate) fn name_to_id_owned(&self) -> HashMap<String, usize> {
        self.name_to_id
            .iter()
            .map(|(k, v)| (k.to_string(), *v))
            .collect()
    }

    pub(crate) fn id_to_name_owned(&self) -> HashMap<usize, String> {
        self.id_to_name
            .iter()
            .map(|(k, v)| (*k, v.to_string()))
            .collect()
    }

    pub(crate) fn bitmap_nbytes(&self) -> usize {
        (self.id_cn + 7) / 8
    }

    fn insert_space(&mut self, name: &'static str, id: usize) {
        self.name_to_id.insert(name, id);
        self.id_to_name.insert(id, name);
    }

    fn new_id(&mut self) -> usize {
        let id = self.id_cn;
        self.id_cn += 1;

        id
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct VirtualSpace {
    member: Vec<u8>,
    read: Vec<u8>,
    write: Vec<u8>,
    see: Vec<u8>,
}

impl VirtualSpace {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn set_member(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.member = spaces_to_bitmap(spaces, def);
    }

    pub(crate) fn set_read(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.read = spaces_to_bitmap(spaces, def);
    }

    pub(crate) fn set_write(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.write = spaces_to_bitmap(spaces, def);
    }

    pub(crate) fn set_see(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.see = spaces_to_bitmap(spaces, def);
    }

    pub(crate) fn to_member_bytes(&self) -> Vec<u8> {
        self.member.clone()
    }

    pub(crate) fn to_read_bytes(&self) -> Vec<u8> {
        self.read.clone()
    }

    pub(crate) fn to_write_bytes(&self) -> Vec<u8> {
        self.write.clone()
    }

    pub(crate) fn to_see_bytes(&self) -> Vec<u8> {
        self.see.clone()
    }
}

pub(crate) fn spaces_to_bitmap(spaces: &[Space], def: &SpaceDef) -> Vec<u8> {
    let nbytes = def.bitmap_nbytes();
    let ids = &def.name_to_id;

    let mut vec = vec![0; nbytes];
    for space in spaces {
        match space {
            Space::All => {
                // note that medusa object bitmap will have extra bits zeroed
                // which are not used nevertheless
                bitmap::set_all(&mut vec);
            }
            Space::ByName(name) if !name.is_empty() => {
                let id = ids
                    .get(name)
                    .unwrap_or_else(|| panic!("no such id for space: {}", name));
                bitmap::set_bit(&mut vec, *id);
            }
            _ => (),
        }
    }

    vec
}
