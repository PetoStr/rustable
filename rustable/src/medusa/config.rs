#![allow(dead_code)]

use crate::medusa::constants::{HandlerFlags, NODE_HIGHEST_PRIORITY};
use crate::medusa::error::ConfigError;
use crate::medusa::handler::{CustomHandler, EventHandler, EventHandlerBuilder};
use crate::medusa::space::{SpaceBuilder, SpaceDef};
use crate::medusa::tree::{Node, NodeBuilder, Tree, TreeBuilder};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct Config {
    trees: Box<[Tree]>,
    cinfo_nodes: HashMap<usize, Arc<Node>>,

    event_handlers: HashMap<String, Box<[EventHandler]>>,
    name_to_space_bit: HashMap<String, usize>,
    space_bit_to_name: HashMap<usize, String>,
    // TODO medusa connections, default answer
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    pub fn tree_by_name(&self, name: &str) -> Option<&Tree> {
        self.trees.iter().find(|x| x.name() == name)
    }

    pub fn name_to_space_bit(&self, name: &str) -> Option<&usize> {
        self.name_to_space_bit.get(name)
    }

    pub fn space_bit_to_name(&self, bit: &usize) -> Option<&String> {
        self.space_bit_to_name.get(bit)
    }

    pub(crate) fn node_by_cinfo(&self, cinfo: &usize) -> Option<&Arc<Node>> {
        self.cinfo_nodes.get(cinfo)
    }

    pub(crate) fn handlers_by_event(&self, event: &str) -> Option<&[EventHandler]> {
        self.event_handlers.get(event).map(|x| x.as_ref())
    }
}

struct ParsedPath {
    tree_name: &'static str,
    items: Vec<&'static str>,
}

impl ParsedPath {
    fn new(path: &'static str) -> Self {
        let mut split = path.split_terminator('/');

        let tree_name = split
            .next()
            .expect("Path is missing a tree name at the start.");

        // `/` (root) should always be the first and only item
        let mut items = vec!["/"];
        items.extend(split);

        Self { tree_name, items }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    trees: HashMap<String, TreeBuilder>,

    include_space: HashMap<&'static str, Vec<&'static str>>,
    exclude_space: HashMap<&'static str, Vec<&'static str>>,
    space_to_path: HashMap<&'static str, (&'static str, bool)>,

    event_handlers: HashMap<String, Vec<EventHandlerBuilder>>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_space(mut self, space: SpaceBuilder) -> Self {
        let name = space.name();
        let path = space.path();
        let recursive = space.recursive();

        if self.space_to_path.insert(name, (path, recursive)).is_some() {
            panic!("duplicate space name \"{name}\"");
        }

        let parsed_path = ParsedPath::new(path);
        let last_node = self.update_or_create_tree_by_path(parsed_path, recursive, name, true);
        last_node.set_access_without_member(&space.at_names);

        for (include_path, recursive) in space.include_path {
            let parsed_path = ParsedPath::new(include_path);
            self.update_or_create_tree_by_path(parsed_path, recursive, name, true);
        }

        for (exclude_path, recursive) in space.exclude_path {
            let parsed_path = ParsedPath::new(exclude_path);
            self.update_or_create_tree_by_path(parsed_path, recursive, name, false);
        }

        self.include_space
            .entry(name)
            .or_default()
            .extend(space.include_space);
        self.exclude_space
            .entry(name)
            .or_default()
            .extend(space.exclude_space);

        self
    }

    pub fn add_spaces<I>(mut self, spaces: I) -> Self
    where
        I: IntoIterator<Item = SpaceBuilder>,
    {
        for space in spaces {
            self = self.add_space(space);
        }
        self
    }

    pub fn add_tree(mut self, tree: TreeBuilder) -> Self {
        let name = tree.name().to_owned();
        self.trees.insert(name, tree);
        self
    }

    pub fn add_event_handler(mut self, event_handler: EventHandlerBuilder) -> Self {
        let event = event_handler.event.to_string();
        self.event_handlers
            .entry(event)
            .or_default()
            .push(event_handler);
        self
    }

    pub fn add_hierarchy_event_handler(
        mut self,
        event: &'static str,
        primary_tree: &str,
        attribute: Option<&str>,
        flags: HandlerFlags,
    ) -> Self {
        let event_handler = EventHandlerBuilder::new()
            .event(event)
            .with_hierarchy_handler(primary_tree, attribute, flags);

        let event = event_handler.event.to_string();
        self.event_handlers
            .entry(event)
            .or_default()
            .push(event_handler);
        self
    }

    pub fn add_custom_event_handler(mut self, custom_handler: impl CustomHandler) -> Self {
        let event_handler = EventHandlerBuilder::new().with_custom_handler(custom_handler);

        let event = event_handler.event.to_string();
        self.event_handlers
            .entry(event)
            .or_default()
            .push(event_handler);
        self
    }

    pub fn build(mut self) -> Result<Config, ConfigError> {
        let mut def = SpaceDef::new();
        let mut cinfo = HashMap::new();

        for (space, includes) in self.include_space.clone() {
            for include in includes {
                let &(path, recursive) = self
                    .space_to_path
                    .get(include)
                    .unwrap_or_else(|| panic!("Space {include} does not exist"));
                let parsed_path = ParsedPath::new(path);
                self.update_or_create_tree_by_path(parsed_path, recursive, space, true);
            }
        }

        for (space, excludes) in self.exclude_space.clone() {
            for exclude in excludes {
                let &(path, recursive) = self
                    .space_to_path
                    .get(exclude)
                    .unwrap_or_else(|| panic!("Space {exclude} does not exist"));
                let parsed_path = ParsedPath::new(path);
                self.update_or_create_tree_by_path(parsed_path, recursive, space, false);
            }
        }

        let trees = self
            .trees
            .into_iter()
            .map(|(_, x)| x.build(&mut def, &mut cinfo))
            .collect::<Result<_, _>>()?;

        let event_handlers = self
            .event_handlers
            .into_iter()
            .map(|(k, v)| (k, v.into_iter().map(|x| x.build(&def)).collect()))
            .collect::<HashMap<String, Box<[EventHandler]>>>();

        let name_to_space_bit = def.name_to_id_owned();
        let space_bit_to_name = def.id_to_name_owned();

        Ok(Config {
            trees,
            cinfo_nodes: cinfo,
            event_handlers,
            name_to_space_bit,
            space_bit_to_name,
        })
    }

    fn update_or_create_tree_by_path(
        &mut self,
        path: ParsedPath,
        recursive: bool,
        space: &'static str,
        include: bool,
    ) -> &mut NodeBuilder {
        let tree = self.get_or_create_tree(path.tree_name);
        let mut iter = path.items.into_iter();

        let root_path = iter.next().expect("Root is missing.");

        let mut node = tree.get_or_create_root(root_path);
        for item in iter {
            node = node.get_or_create_child(NODE_HIGHEST_PRIORITY, item);
        }

        node.member_of_include_or_exclude(space, include);

        node.set_recursive(recursive);

        node
    }

    fn get_or_create_tree(&mut self, name: &'static str) -> &mut TreeBuilder {
        self.trees
            .entry(name.to_owned())
            .or_insert_with(|| TreeBuilder::new().with_name(name))
    }
}
