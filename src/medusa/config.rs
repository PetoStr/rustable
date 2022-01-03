#![allow(dead_code)]

use crate::medusa::Tree;

#[derive(Debug)]
pub struct Config {
    trees: Box<[Tree]>,
    // TODO medusa connections, default answer
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    pub fn tree_by_event(&self, event_name: &str) -> Option<&Tree> {
        self.trees.iter().find(|x| x.event() == event_name)
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    trees: Vec<Tree>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_tree(mut self, tree: Tree) -> Self {
        self.trees.push(tree);
        self
    }

    pub fn build(self) -> Config {
        Config {
            trees: self.trees.into(),
        }
    }
}
