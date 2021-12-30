#![allow(dead_code)]

use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
struct Node {
    name: String,
    path: Regex,
    children: Box<[Node]>,
}

#[derive(Debug)]
pub struct Tree {
    root: Node,
}

impl Tree {
    pub fn builder() -> TreeBuilder {
        TreeBuilder::default()
    }
}

#[derive(Default)]
struct NodeBuilder {
    name: String,
    path: String,

    parent: Option<Rc<RefCell<NodeBuilder>>>,
    children: Vec<Rc<RefCell<NodeBuilder>>>,
}

impl NodeBuilder {
    fn build(&self) -> Node {
        let name = self.name.clone();
        // TODO Result
        let path = Regex::new(&self.path).expect("invalid expression");
        let children = self.children.iter().map(|x| x.borrow().build()).collect();

        Node {
            name,
            path,
            children,
        }
    }
}

#[derive(Default)]
pub struct TreeBuilder {
    cur: Rc<RefCell<NodeBuilder>>,
}

impl TreeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn begin_node(mut self, name: &str, path: &str) -> Self {
        let child = Rc::new(RefCell::new(NodeBuilder {
            name: name.to_owned(),
            path: path.to_owned(),
            parent: Some(Rc::clone(&self.cur)),
            children: Vec::new(),
        }));

        let res = Rc::clone(&child);
        self.cur.borrow_mut().children.push(child);

        self.cur = res;
        self
    }

    pub fn with_handler(self) -> Self {
        unimplemented!()
    }

    pub fn end_node(mut self) -> Self {
        let parent = Rc::clone(
            self.cur
                .borrow()
                .parent
                .as_ref()
                .expect("end_node() called on root"),
        );
        self.cur = parent;
        self
    }

    pub fn build(&mut self) -> Tree {
        if self.cur.borrow().parent.is_some() {
            // TODO return Error
            panic!("can only build from root level, use end_node() for every begin_node()");
        }

        let root = self.cur.borrow().build();
        Tree { root }
    }
}
