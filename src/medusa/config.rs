#![allow(dead_code)]

use crate::medusa::error::ConfigError;
use crate::medusa::handler::{EventHandler, EventHandlerBuilder};
use crate::medusa::space::SpaceDef;
use crate::medusa::tree::{Node, Tree, TreeBuilder};
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

#[derive(Default)]
pub struct ConfigBuilder {
    trees: Vec<TreeBuilder>,
    event_handlers: HashMap<String, Vec<EventHandlerBuilder>>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_tree(mut self, tree: TreeBuilder) -> Self {
        self.trees.push(tree);
        self
    }

    pub fn add_event_handler(mut self, event_handler: EventHandlerBuilder) -> Self {
        let event = event_handler.event.clone();
        self.event_handlers
            .entry(event)
            .or_default()
            .push(event_handler);
        self
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let mut def = SpaceDef::new();
        let mut cinfo = HashMap::new();

        let trees = self
            .trees
            .into_iter()
            .map(|x| x.build(&mut def, &mut cinfo))
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
}
