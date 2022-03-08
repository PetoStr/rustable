#![allow(dead_code)]

use crate::medusa::space::{Space, SpaceDef, VirtualSpace};
use crate::medusa::ConfigError;
use regex::Regex;
use std::collections::HashMap;
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
    name: String,
    root: Arc<Node>,
}

impl Tree {
    pub fn builder() -> TreeBuilder {
        TreeBuilder::new()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn root(&self) -> &Arc<Node> {
        &self.root
    }
}

#[derive(Debug, Default)]
pub struct NodeBuilder {
    path: String,

    member_of: Vec<Space>,
    reads: Vec<Space>,
    writes: Vec<Space>,
    sees: Vec<Space>,

    children: Vec<NodeBuilder>,
}

impl NodeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn path(mut self, path: &str) -> Self {
        self.path = path.to_owned();
        self
    }

    pub fn member_of(mut self, name: &str) -> Self {
        self.member_of.push(Space::ByName(name.to_owned()));
        self
    }

    pub fn reads(mut self, name: &str) -> Self {
        self.reads.push(Space::ByName(name.to_owned()));
        self
    }

    pub fn writes(mut self, name: &str) -> Self {
        self.writes.push(Space::ByName(name.to_owned()));
        self
    }

    pub fn sees(mut self, name: &str) -> Self {
        self.sees.push(Space::ByName(name.to_owned()));
        self
    }

    pub fn add_node(mut self, node: NodeBuilder) -> Self {
        self.children.push(node);
        self
    }

    // these functions below create a new node in constable
    /*pub fn include_path(mut self, path: &str) -> Self {
        self
    }

    pub fn exclude_path(mut self, path: &str) -> Self {
        self
    }

    pub fn include_space(mut self, path: &str) -> Self {
        self
    }

    pub fn exclude_space(mut self, path: &str) -> Self {
        self
    }*/

    fn build(
        self,
        def: &mut SpaceDef,
        cinfo: &mut HashMap<usize, Arc<Node>>,
    ) -> Result<Arc<Node>, ConfigError> {
        let children = self
            .children
            .into_iter()
            .map(|x| x.build(def, cinfo))
            .collect::<Result<_, _>>()?;

        let path_regex = Regex::new(&self.path)?;

        // define new spaces which may not exist yet
        self.member_of
            .iter()
            .for_each(|space| def.define_space(space.clone()));
        self.reads
            .iter()
            .for_each(|space| def.define_space(space.clone()));
        self.writes
            .iter()
            .for_each(|space| def.define_space(space.clone()));
        self.sees
            .iter()
            .for_each(|space| def.define_space(space.clone()));

        let mut vs = VirtualSpace::new();
        vs.set_member(def, &self.member_of);
        vs.set_read(def, &self.reads);
        vs.set_write(def, &self.writes);
        vs.set_see(def, &self.sees);

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
    name: String,
    root: NodeBuilder,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_owned();
        self
    }

    pub fn set_root(mut self, root: NodeBuilder) -> Self {
        self.root = root;
        self
    }

    pub(crate) fn build(
        self,
        def: &mut SpaceDef,
        cinfo: &mut HashMap<usize, Arc<Node>>,
    ) -> Result<Tree, ConfigError> {
        Ok(Tree {
            name: self.name,
            root: self.root.build(def, cinfo)?,
        })
    }
}
