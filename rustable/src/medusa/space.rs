use crate::bitmap;
use crate::medusa::constants::AccessType;
use std::collections::HashMap;

/// Builder for virtual space.
#[derive(Debug, Default, Clone)]
pub struct SpaceBuilder {
    pub(crate) name: Option<&'static str>,
    pub(crate) path: Option<(&'static str, bool)>,

    pub(crate) at_names: [Vec<&'static str>; AccessType::Length as usize],

    pub(crate) include_space: Vec<&'static str>,
    pub(crate) exclude_space: Vec<&'static str>,

    pub(crate) include_path: Vec<(&'static str, bool)>,
    pub(crate) exclude_path: Vec<(&'static str, bool)>,
}

impl SpaceBuilder {
    /// Creates new `SpaceBuilder`.
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns virtual space name.
    pub fn name(&self) -> &'static str {
        self.name.as_ref().expect("Space does not have a name.")
    }

    /// Returns virtual space path.
    pub fn path(&self) -> &'static str {
        self.path.as_ref().expect("Space does not have a path.").0
    }

    /// Returns whether virtual space path is recursive.
    pub fn recursive(&self) -> bool {
        self.path.as_ref().expect("Space does not have a path.").1
    }

    /// Sets virtual space name.
    ///
    /// Returns `Self`.
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets virtual space path.
    ///
    /// Returns `Self`.
    pub fn with_path(mut self, path: &'static str) -> Self {
        self.path = Some((path, false));
        self
    }

    /// Sets virtual space path recursively.
    ///
    /// Returns `Self`.
    pub fn with_path_recursive(mut self, path: &'static str) -> Self {
        self.path = Some((path, true));
        self
    }

    /// Extends access rights for type `read`.
    ///
    /// Returns `Self`.
    pub fn reads<I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = &'static str>,
    {
        self.at_names[AccessType::Read as usize].extend(names);
        self
    }

    /// Extends access rights for type `write`.
    ///
    /// Returns `Self`.
    pub fn writes<I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = &'static str>,
    {
        self.at_names[AccessType::Write as usize].extend(names);
        self
    }

    /// Extends access rights for type `see`.
    ///
    /// Returns `Self`.
    pub fn sees<I>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = &'static str>,
    {
        self.at_names[AccessType::See as usize].extend(names);
        self
    }

    /// Includes the provided virtual space by name.
    ///
    /// Returns `Self`.
    pub fn include_space(mut self, space: &'static str) -> Self {
        self.include_space.push(space);
        self
    }

    /// Excludes the provided virtual space by name.
    ///
    /// Returns `Self`.
    pub fn exclude_space(mut self, space: &'static str) -> Self {
        self.exclude_space.push(space);
        self
    }

    /// Includes the provided path.
    ///
    /// Returns `Self`.
    pub fn include_path(mut self, path: &'static str) -> Self {
        self.include_path.push((path, false));
        self
    }

    /// Includes the provided path recursively.
    ///
    /// Returns `Self`.
    pub fn include_path_recursive(mut self, path: &'static str) -> Self {
        self.include_path.push((path, true));
        self
    }

    /// Excludes the provided path.
    ///
    /// Returns `Self`.
    pub fn exclude_path(mut self, path: &'static str) -> Self {
        self.exclude_path.push((path, false));
        self
    }

    /// Excludes the provided path recursively.
    ///
    /// Returns `Self`.
    pub fn exclude_path_recursive(mut self, path: &'static str) -> Self {
        self.exclude_path.push((path, true));
        self
    }
}

/// Virtual space reference without the need of using special symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    /// Covers all possible virtual space names.
    All,

    /// Covers only one specific virtual space name.
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

    pub(crate) fn define_space(&mut self, name: &'static str) {
        if self.name_to_id.contains_key(name) {
            return;
        }

        let id = self.new_id();
        self.insert_space(name, id);
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

/// A set of virtual spaces for various access types. The name can be misleading.
#[derive(Debug, Default, Clone)]
pub struct VirtualSpace {
    access_types: [Vec<u8>; AccessType::Length as usize],
}

impl VirtualSpace {
    /// Creates new `VirtualSpace`.
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn set_access_types(
        &mut self,
        def: &SpaceDef,
        spaces: &[Vec<Space>; AccessType::Length as usize],
    ) {
        for (at, space) in self.access_types.iter_mut().zip(spaces.iter()) {
            *at = spaces_to_bitmap(space, def);
        }
    }

    /// Returns a vector of defined `at` access types.
    pub fn to_at_bytes(&self, at: AccessType) -> Vec<u8> {
        self.access_types[at as usize].clone()
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
