use crate::bitmap;
use crate::cstr_to_string;
use crate::medusa::space::{spaces_to_bitmap, Space, SpaceDef};
use crate::medusa::{
    AuthRequestData, Context, HandlerFlags, MedusaAnswer, MedusaClass, MedusaEvtype,
};
use derivative::Derivative;
use std::future::Future;
use std::pin::Pin;

pub struct HandlerArgs<'a> {
    pub evtype: MedusaEvtype,
    pub subject: MedusaClass,
    pub object: Option<MedusaClass>,

    pub handler_data: &'a HandlerData,
}

pub type Handler =
    for<'a> fn(
        ctx: &'a Context,
        args: HandlerArgs<'a>,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<MedusaAnswer>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct HandlerData {
    pub event: String,
    pub attribute: Option<String>,
    pub flags: HandlerFlags,

    pub primary_tree: String,

    pub subject_vs: Vec<u8>,
    pub object_vs: Vec<u8>,

    bitmap_nbytes: usize,
}

#[macro_export]
macro_rules! force_boxed {
    ($inc:expr) => {{
        fn boxed<'a>(
            ctx: &'a $crate::medusa::Context,
            args: $crate::medusa::HandlerArgs<'a>,
        ) -> ::std::pin::Pin<
            ::std::boxed::Box<
                dyn ::std::future::Future<Output = ::anyhow::Result<$crate::medusa::MedusaAnswer>>
                    + ::std::marker::Send
                    + 'a,
            >,
        > {
            ::std::boxed::Box::pin($inc(ctx, args))
        }
        boxed
    }};
}

pub struct CustomHandlerDef {
    pub event: &'static str,
    pub handler: Handler,
    pub subject: Space,
    pub object: Option<Space>,
}

pub trait CustomHandler {
    fn define(self) -> CustomHandlerDef;
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct EventHandlerBuilder {
    pub(crate) event: &'static str,
    attribute: Option<String>,
    flags: HandlerFlags,
    primary_tree: String,

    subject: Option<Space>,
    object: Option<Space>,

    #[derivative(Debug = "ignore")]
    handler: Option<Handler>,
}

impl EventHandlerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn event(mut self, event: &'static str) -> Self {
        self.event = event;
        self
    }

    pub fn with_hierarchy_handler(
        mut self,
        primary_tree: &str,
        attribute: Option<&str>,
        flags: HandlerFlags,
    ) -> Self {
        if self.handler.is_some() {
            panic!("handler already set");
        }

        self.attribute = attribute.map(|x| x.to_owned());
        self.flags = flags;
        self.subject = Some(Space::All);
        self.object = Some(Space::All);
        self.primary_tree = primary_tree.to_owned();
        self.handler = Some(force_boxed!(hierarchy_handler));
        self
    }

    pub fn with_custom_handler(mut self, custom_handler: impl CustomHandler) -> Self {
        if self.handler.is_some() {
            panic!("handler already set");
        }

        let CustomHandlerDef {
            event,
            handler,
            subject,
            object,
        } = custom_handler.define();

        self.event = event;
        self.subject = Some(subject);
        self.object = object;
        self.handler = Some(handler);
        self
    }

    pub(crate) fn build(self, def: &SpaceDef) -> EventHandler {
        let handler = self
            .handler
            .unwrap_or_else(|| panic!("no handler specified for event: {}", self.event));

        let bitmap_nbytes = def.bitmap_nbytes();
        let subject_vs = spaces_to_bitmap(&[self.subject.unwrap()], def);
        let object_vs = match self.object {
            Some(object) => spaces_to_bitmap(&[object], def),
            None => vec![0xff; bitmap_nbytes],
        };

        EventHandler {
            data: HandlerData {
                event: self.event.to_string(),
                attribute: self.attribute,
                flags: self.flags,
                primary_tree: self.primary_tree,
                subject_vs,
                object_vs,
                bitmap_nbytes,
            },
            handler,
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct EventHandler {
    data: HandlerData,

    #[derivative(Debug = "ignore")]
    handler: Handler,
}

impl EventHandler {
    pub fn builder() -> EventHandlerBuilder {
        EventHandlerBuilder::new()
    }

    pub(crate) async fn handle(&self, ctx: &Context, auth_data: AuthRequestData) -> MedusaAnswer {
        let args = HandlerArgs {
            evtype: auth_data.evtype,
            subject: auth_data.subject,
            object: auth_data.object,
            handler_data: &self.data,
        };
        (self.handler)(ctx, args)
            .await
            .expect("Handler returned error")
    }

    pub(crate) fn is_applicable(
        &self,
        subject: &MedusaClass,
        object: Option<&MedusaClass>,
    ) -> bool {
        if !bitmap::all(&self.data.subject_vs) {
            let svs = &subject.get_vs().expect("subject has no vs")[..self.data.bitmap_nbytes];
            if bitmap::and(&mut self.data.subject_vs.clone(), svs) != self.data.subject_vs {
                return false;
            }
        }

        if !bitmap::all(&self.data.object_vs) {
            if let Some(object) = object {
                let ovs = &object.get_vs().expect("object has no vs")[..self.data.bitmap_nbytes];
                if bitmap::and(&mut self.data.object_vs.clone(), ovs) != self.data.object_vs {
                    return false;
                }
            }
        }

        true
    }
}

async fn hierarchy_handler(ctx: &Context, args: HandlerArgs<'_>) -> anyhow::Result<MedusaAnswer> {
    let config = ctx.config();
    let HandlerArgs {
        mut subject,
        object,
        evtype,
        handler_data,
    } = args;

    let tree = config
        .tree_by_name(&handler_data.primary_tree)
        .unwrap_or_else(|| panic!("primary tree `{}` not found", handler_data.primary_tree));

    let mut cinfo = subject.get_object_cinfo()?;
    let mut node;

    let path_attr = handler_data.attribute.as_deref().unwrap_or("");
    let path = cstr_to_string(evtype.get_attribute(path_attr).unwrap_or(b"\0"));

    if cinfo == 0 {
        if handler_data.flags.contains(HandlerFlags::FROM_OBJECT)
            && subject.header.id == object.as_ref().expect("No object.").header.id
            && path != "/"
        // ignore root's possible parent
        {
            let parent_cinfo = object.as_ref().expect("No object.").get_object_cinfo()?;
            cinfo = parent_cinfo;
        }

        if cinfo == 0 {
            node = tree.root();
        } else {
            node = config.node_by_cinfo(&cinfo).expect("node not found");
        }

        let _ = subject.clear_object_act();
        let _ = subject.clear_subject_act();
    } else {
        node = config.node_by_cinfo(&cinfo).expect("node not found");
    }

    // is not root?
    if cinfo != 0 {
        if let Some(child) = node.child_by_path(&path) {
            node = child;
        } else {
            println!("{} not covered by tree", path);
            return Ok(MedusaAnswer::Deny);
        }
    }

    println!(
        "{}: \"{}\" -> \"{}\"",
        evtype.header.name,
        path,
        node.path()
    );

    subject.enter_tree_with_node(ctx, &evtype, node).await;

    Ok(MedusaAnswer::Allow)
}
