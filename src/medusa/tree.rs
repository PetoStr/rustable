#![allow(dead_code)]

use crate::medusa::handler::EventHandler;
use derivative::Derivative;
use regex::Regex;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

#[derive(Derivative)]
#[derivative(Debug)]
struct Node {
    name: String,
    path_regex: Regex,

    #[derivative(Debug = "ignore")]
    handler: Option<Box<dyn EventHandler>>,

    children: Box<[Node]>,
}

#[derive(Debug)]
pub struct Tree {
    event: String,
    attribute: Option<String>,

    root: Node,
}

impl Tree {
    pub fn builder(event: &str) -> TreeBuilder {
        TreeBuilder::new(event, None)
    }

    pub fn builder_with_attribute(event: &str, attribute: &str) -> TreeBuilder {
        TreeBuilder::new(event, Some(attribute))
    }

    pub fn handler_by_path(&self, path: &str) -> Option<&dyn EventHandler> {
        println!("handler_by_path \"{}\"", path);
        let mut res = None;
        let mut cur_node = &self.root;
        let iter = Path::new(path).iter();

        for cur_val in iter.map(|x| x.to_str().unwrap()) {
            let mut search = false;

            for child in cur_node.children.iter() {
                if child.path_regex.is_match(cur_val) {
                    cur_node = child;
                    res = cur_node.handler.as_deref();
                    search = true;
                    break;
                }
            }

            if !search {
                break;
            }
        }
        println!("{:?}", cur_node);
        println!("handler found: {}\n", res.is_some());

        res
    }

    pub fn event(&self) -> &str {
        &self.event
    }

    pub fn attribute(&self) -> Option<&str> {
        self.attribute.as_deref()
    }
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
struct NodeBuilder {
    name: String,
    path: String,

    #[derivative(Debug = "ignore")]
    handler: Option<Box<dyn EventHandler>>,

    parent: Option<Rc<RefCell<NodeBuilder>>>,
    children: Vec<Rc<RefCell<NodeBuilder>>>,
}

impl NodeBuilder {
    fn build(&mut self) -> Node {
        let name = self.name.clone();
        // TODO Result
        let path_regex = Regex::new(&self.path).expect("invalid expression");
        let handler = self.handler.take();
        let children = self
            .children
            .iter()
            .map(|x| x.borrow_mut().build())
            .collect();

        Node {
            name,
            path_regex,
            handler,
            children,
        }
    }
}

#[derive(Default)]
pub struct TreeBuilder {
    event: String,
    attribute: Option<String>,

    cur: Rc<RefCell<NodeBuilder>>,
}

impl TreeBuilder {
    pub fn new(event: &str, attribute: Option<&str>) -> Self {
        Self {
            event: event.to_owned(),
            attribute: attribute.map(|x| x.to_owned()),
            ..Default::default()
        }
    }

    pub fn begin_node(mut self, name: &str, path: &str) -> Self {
        let child = Rc::new(RefCell::new(NodeBuilder {
            name: name.to_owned(),
            path: path.to_owned(),
            handler: None,
            parent: Some(Rc::clone(&self.cur)),
            children: Vec::new(),
        }));

        let res = Rc::clone(&child);
        self.cur.borrow_mut().children.push(child);

        self.cur = res;
        self
    }

    pub fn with_handler<H>(self, handler: H) -> Self
    where
        H: EventHandler,
    {
        self.cur.borrow_mut().handler = Some(Box::new(handler));
        self
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

    pub fn build(self) -> Tree {
        let mut cur = self.cur.borrow_mut();
        if cur.parent.is_some() {
            // TODO return Error
            panic!("can only build from root level, use end_node() for every begin_node()");
        }

        let root = cur.build();
        Tree {
            event: self.event,
            attribute: self.attribute,
            root,
        }
    }
}
