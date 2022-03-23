use crate::bitmap;
use crate::cstr_to_string;
use crate::medusa::space::{spaces_to_bitmap, Space, SpaceDef};
use crate::medusa::{
    AuthRequestData, Context, MedusaAnswer, MedusaClass, MedusaEvtype, Monitoring,
};
use derivative::Derivative;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct HandlerArgs<'a> {
    pub evtype: MedusaEvtype,
    pub subject: MedusaClass,
    pub object: Option<MedusaClass>,

    pub handler_data: &'a HandlerData,
}

pub type Handler = for<'a> fn(
    ctx: &'a Context,
    args: HandlerArgs<'a>,
) -> Pin<Box<dyn Future<Output = MedusaAnswer> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct HandlerData {
    pub event: String,
    pub attribute: Option<String>,
    pub from_object: bool,

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
                dyn ::std::future::Future<Output = $crate::medusa::MedusaAnswer>
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
    from_object: bool,
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
        from_object: bool,
    ) -> Self {
        if self.handler.is_some() {
            panic!("handler already set");
        }

        self.attribute = attribute.map(|x| x.to_owned());
        self.from_object = from_object;
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
                from_object: self.from_object,
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
        (self.handler)(ctx, args).await
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

// TODO replace unwraps
async fn hierarchy_handler(ctx: &Context, mut args: HandlerArgs<'_>) -> MedusaAnswer {
    let config = ctx.config();

    let tree = config
        .tree_by_name(&args.handler_data.primary_tree)
        .unwrap_or_else(|| {
            panic!(
                "primary tree `{}` not found",
                args.handler_data.primary_tree
            )
        });

    let mut cinfo = args.subject.get_object_cinfo().unwrap();
    let mut node;

    let path_attr = args.handler_data.attribute.as_deref().unwrap_or("");
    let path = cstr_to_string(args.evtype.get_attribute(path_attr).unwrap_or(b"\0"));

    if cinfo == 0 {
        if args.handler_data.from_object
            && args.subject.header.id == args.object.as_ref().unwrap().header.id
            && path != "/"
        // ignore root's possible parent
        {
            let parent_cinfo = args.object.as_ref().unwrap().get_object_cinfo().unwrap();
            cinfo = parent_cinfo;
        }

        if cinfo == 0 {
            node = tree.root();
        } else {
            node = config.node_by_cinfo(&cinfo).expect("node not found");
        }

        let _ = args.subject.clear_object_act();
        let _ = args.subject.clear_subject_act();
    } else {
        node = config.node_by_cinfo(&cinfo).expect("node not found");
    }

    // is not root?
    if cinfo != 0 {
        if let Some(child) = node.child_by_path(&path) {
            node = child;
        } else {
            println!("{} not covered by tree", path);
            return MedusaAnswer::Deny;
        }
    }
    cinfo = Arc::as_ptr(node) as usize;

    println!(
        "{}: \"{}\" -> \"{}\"",
        args.evtype.header.name,
        path,
        node.path()
    );

    let _ = args.subject.set_vs(node.virtual_space().to_member_bytes());
    let _ = args
        .subject
        .set_vs_read(node.virtual_space().to_read_bytes());
    let _ = args
        .subject
        .set_vs_write(node.virtual_space().to_write_bytes());
    let _ = args.subject.set_vs_see(node.virtual_space().to_see_bytes());
    if node.has_children() && args.evtype.header.monitoring == Monitoring::Object {
        let _ = args
            .subject
            .add_object_act(args.evtype.header.monitoring_bit as usize);
        let _ = args
            .subject
            .add_subject_act(args.evtype.header.monitoring_bit as usize);
    }

    args.subject.set_object_cinfo(cinfo).unwrap();

    ctx.update_object_no_wait(&args.subject);
    //ctx.update_object(&args.subject).await;

    MedusaAnswer::Ok
}
