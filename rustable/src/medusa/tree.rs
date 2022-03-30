#![allow(dead_code)]

use crate::medusa::constants::{AccessType, NODE_HIGHEST_PRIORITY};
use crate::medusa::space::{Space, SpaceDef, VirtualSpace};
use crate::medusa::ConfigError;
use regex::Regex;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug)]
pub struct Node {
    path_regex: Regex,

    vs: VirtualSpace,

    children: Box<[Arc<Node>]>,
}

impl Node {
    pub fn builder() -> NodeBuilder {
        NodeBuilder::new()
    }

    pub(crate) fn path(&self) -> &str {
        self.path_regex.as_str()
    }

    pub(crate) fn has_children(&self) -> bool {
        self.children.len() > 0
    }

    pub(crate) fn child_by_path(&self, path: &str) -> Option<&Arc<Node>> {
        self.children.iter().find(|x| x.path_regex.is_match(path))
    }

    pub(crate) fn virtual_space(&self) -> &VirtualSpace {
        &self.vs
    }
}

#[derive(Debug)]
pub struct Tree {
    name: &'static str,
    root: Arc<Node>,
}

impl Tree {
    pub fn builder() -> TreeBuilder {
        TreeBuilder::new()
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub(crate) fn root(&self) -> &Arc<Node> {
        &self.root
    }
}

#[derive(Debug, Default)]
pub struct NodeBuilder {
    path: &'static str,
    regex_path: &'static str,

    at_names: [HashSet<&'static str>; AccessType::Length as usize],

    children: BTreeMap<u16, HashMap<String, NodeBuilder>>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn path(&self) -> &'static str {
        self.path
    }

    pub fn with_path(mut self, path: &'static str) -> Self {
        self.path = path;
        self
    }

    pub fn add_access_type(mut self, at: AccessType, name: &'static str) -> Self {
        self.at_names[at as usize].insert(name);
        self
    }

    pub fn add_node(mut self, node: NodeBuilder) -> Self {
        let path = node.path().to_owned();
        self.children
            .entry(NODE_HIGHEST_PRIORITY)
            .or_default()
            .insert(path, node);
        self
    }

    pub fn add_node_with_priority(mut self, priority: u16, node: NodeBuilder) -> Self {
        let path = node.path().to_owned();
        self.children
            .entry(priority)
            .or_default()
            .insert(path, node);
        self
    }

    pub(crate) fn get_or_create_child(
        &mut self,
        priority: u16,
        path: &'static str,
    ) -> &mut NodeBuilder {
        self.children
            .entry(priority)
            .or_default()
            .entry(path.to_owned())
            .or_insert_with(|| NodeBuilder::new().with_path(path))
    }

    pub(crate) fn set_access_without_member(
        &mut self,
        at_names: &[Vec<&'static str>; AccessType::Length as usize],
    ) {
        for (r#type, set) in self.at_names.iter_mut().enumerate() {
            if r#type != AccessType::Member as usize {
                set.extend(&at_names[r#type as usize]);
            }
        }
    }

    pub(crate) fn member_of_include_or_exclude(&mut self, name: &'static str, include: bool) {
        if include {
            self.at_names[AccessType::Member as usize].insert(name);
        } else {
            self.at_names[AccessType::Member as usize].remove(name);
        }
    }

    fn build(
        self,
        def: &mut SpaceDef,
        cinfo: &mut HashMap<usize, Arc<Node>>,
    ) -> Result<Arc<Node>, ConfigError> {
        let children = self
            .children
            .into_iter()
            .map(|(_, hmap)| hmap)
            .flatten()
            .map(|(_, x)| x.build(def, cinfo))
            .collect::<Result<_, _>>()?;

        let path_regex = Regex::new(self.path)?;

        // define new spaces which may not exist yet (assign an id for every new name)
        self.at_names
            .iter()
            .for_each(|names| names.iter().for_each(|space| def.define_space(space)));

        let spaces = self
            .at_names
            .into_iter()
            .map(|names| names.into_iter().map(Space::ByName).collect::<Vec<Space>>())
            .collect::<Vec<Vec<Space>>>();

        let mut vs = VirtualSpace::new();
        vs.set_access_types(def, &spaces.try_into().unwrap());

        let node = Arc::new(Node {
            path_regex,
            vs,
            children,
        });

        cinfo.insert(Arc::as_ptr(&node) as usize, Arc::clone(&node));

        Ok(node)
    }
}

#[derive(Default)]
pub struct TreeBuilder {
    name: &'static str,
    root: Option<NodeBuilder>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = name;
        self
    }

    pub fn set_root(mut self, root: NodeBuilder) -> Self {
        self.root = Some(root);
        self
    }

    pub(crate) fn get_or_create_root(&mut self, path: &'static str) -> &mut NodeBuilder {
        self.root
            .get_or_insert_with(|| NodeBuilder::new().with_path(path))
    }

    pub(crate) fn build(
        self,
        def: &mut SpaceDef,
        cinfo: &mut HashMap<usize, Arc<Node>>,
    ) -> Result<Tree, ConfigError> {
        Ok(Tree {
            name: self.name,
            root: self.root.expect("Root is missing.").build(def, cinfo)?,
        })
    }
}
