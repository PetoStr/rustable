use bit_vec::BitVec;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Space {
    All,
    ByName(String),
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SpaceDef {
    id_cn: usize,
    name_to_id: HashMap<String, usize>,
    id_to_name: HashMap<usize, String>,
}

impl SpaceDef {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn define_space(&mut self, space: Space) {
        if let Space::ByName(name) = space {
            if self.name_to_id.contains_key(&name) {
                return;
            }

            let id = self.new_id();
            self.insert_space(name, id);
        }
    }

    #[allow(unused)]
    pub(crate) fn id(&self, name: &str) -> Option<&usize> {
        self.name_to_id.get(name)
    }

    #[allow(unused)]
    pub(crate) fn name(&self, id: &usize) -> Option<&String> {
        self.id_to_name.get(id)
    }

    fn insert_space(&mut self, name: String, id: usize) {
        self.name_to_id.insert(name.clone(), id);
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
    member: BitVec,
    read: BitVec,
    write: BitVec,
    see: BitVec,
}

impl VirtualSpace {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    pub(crate) fn set_member(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.member = spaces_to_bitvec(spaces, def);
    }

    pub(crate) fn set_read(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.read = spaces_to_bitvec(spaces, def);
    }

    pub(crate) fn set_write(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.write = spaces_to_bitvec(spaces, def);
    }

    pub(crate) fn set_see(&mut self, def: &SpaceDef, spaces: &[Space]) {
        self.see = spaces_to_bitvec(spaces, def);
    }

    pub(crate) fn to_member_bytes(&self) -> Vec<u8> {
        self.member.to_bytes()
    }

    pub(crate) fn to_read_bytes(&self) -> Vec<u8> {
        self.read.to_bytes()
    }

    pub(crate) fn to_write_bytes(&self) -> Vec<u8> {
        self.write.to_bytes()
    }

    pub(crate) fn to_see_bytes(&self) -> Vec<u8> {
        self.see.to_bytes()
    }
}

pub(crate) fn spaces_to_bitvec(spaces: &[Space], def: &SpaceDef) -> BitVec {
    let nbits = def.id_cn;
    let ids = &def.name_to_id;

    let mut bitvec = BitVec::from_elem(nbits, false);
    for space in spaces {
        match space {
            Space::All => {
                // note that medusa object bitmap will have extra bits zeroed
                // which are not used nevertheless
                bitvec.set_all();
            }
            Space::ByName(name) if !name.is_empty() => {
                let id = ids
                    .get(name)
                    .unwrap_or_else(|| panic!("no such id for space: {}", name));
                bitvec.set(*id, true);
            }
            _ => (),
        }
    }

    bitvec
}
