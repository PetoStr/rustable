use crate::cstr_to_string;
use crate::medusa::space::{spaces_to_bitvec, Space, SpaceDef};
use crate::medusa::{AuthRequestData, MedusaAnswer, MedusaClass, Monitoring, SharedContext};
use async_trait::async_trait;
use bit_vec::BitVec;
use derivative::Derivative;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct HandlerData {
    pub event: String,
    pub attribute: Option<String>,
    pub from_object: bool,

    pub primary_tree: String,

    pub subject_vs: BitVec,
    pub object_vs: BitVec,
}

#[async_trait]
pub trait Handler {
    async fn handle(
        &self,
        data: &HandlerData,
        ctx: &SharedContext,
        auth_data: AuthRequestData,
    ) -> MedusaAnswer;
}

#[derive(Derivative)]
#[derivative(Debug, Default)]
pub struct EventHandlerBuilder {
    pub(crate) event: String,
    attribute: Option<String>,
    from_object: bool,
    primary_tree: String,

    subject: Option<Space>,
    object: Option<Space>,

    #[derivative(Debug = "ignore")]
    handler: Option<Box<dyn Handler + Send + Sync + 'static>>,
}

impl EventHandlerBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn event(mut self, event: &str) -> Self {
        self.event = event.to_owned();
        self
    }

    pub fn with_hierarchy_handler(
        mut self,
        attribute: Option<&str>,
        from_object: bool,
        primary_tree: &str,
    ) -> Self {
        if self.handler.is_some() {
            panic!("handler already set");
        }

        self.attribute = attribute.map(|x| x.to_owned());
        self.from_object = from_object;
        self.subject = Some(Space::All);
        self.object = Some(Space::All);
        self.primary_tree = primary_tree.to_owned();
        self.handler = Some(Box::new(HierarchyHandler));
        self
    }

    pub fn with_custom_handler<H>(
        mut self,
        handler: H,
        subject: Space,
        object: Option<Space>,
    ) -> Self
    where
        H: Handler + Send + Sync + 'static,
    {
        if self.handler.is_some() {
            panic!("handler already set");
        }

        self.subject = Some(subject);
        self.object = object;
        self.handler = Some(Box::from(handler));
        self
    }

    pub(crate) fn build(self, def: &SpaceDef) -> EventHandler {
        let handler = self
            .handler
            .unwrap_or_else(|| panic!("no handler specified for event: {}", self.event));

        let subject_vs = spaces_to_bitvec(&[self.subject.unwrap()], def);
        let object_vs = match self.object {
            Some(object) => spaces_to_bitvec(&[object], def),
            None => BitVec::new(),
        };

        EventHandler {
            data: HandlerData {
                event: self.event,
                attribute: self.attribute,
                from_object: self.from_object,
                primary_tree: self.primary_tree,
                subject_vs,
                object_vs,
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
    handler: Box<dyn Handler + Send + Sync + 'static>,
}

impl EventHandler {
    pub fn builder() -> EventHandlerBuilder {
        EventHandlerBuilder::new()
    }

    pub(crate) async fn handle(
        &self,
        ctx: &SharedContext,
        auth_data: AuthRequestData,
    ) -> MedusaAnswer {
        self.handler.handle(&self.data, ctx, auth_data).await
    }

    pub(crate) fn is_applicable(
        &self,
        subject: &MedusaClass,
        object: Option<&MedusaClass>,
    ) -> bool {
        if !self.data.subject_vs.all() {
            let svs = bitvec_from_vs_exact(subject, self.data.subject_vs.len());
            // clone to prevent self.subject_vs from being modified by calling and()
            if self.data.subject_vs.clone().and(&svs) {
                return false;
            }
        }

        if !self.data.object_vs.all() {
            if let Some(object) = object {
                let ovs = bitvec_from_vs_exact(object, self.data.object_vs.len());
                // clone to prevent self.object_vs from being modified by calling and()
                if self.data.object_vs.clone().and(&ovs) {
                    return false;
                }
            }
        }

        true
    }
}

fn bitvec_from_vs_exact(class: &MedusaClass, len: usize) -> BitVec {
    let mut bitvec = BitVec::from_bytes(class.get_vs().unwrap());
    bitvec.truncate(len);
    bitvec.grow(len, false);

    bitvec
}

struct HierarchyHandler;

#[async_trait]
impl Handler for HierarchyHandler {
    // TODO replace unwraps
    async fn handle(
        &self,
        data: &HandlerData,
        ctx: &SharedContext,
        mut auth_data: AuthRequestData,
    ) -> MedusaAnswer {
        let config = &ctx.config;

        let tree = config
            .tree_by_name(&data.primary_tree)
            .unwrap_or_else(|| panic!("primary tree `{}` not found", data.primary_tree));

        let mut cinfo = auth_data.subject.get_object_cinfo().unwrap();
        let mut node;

        let path_attr = data.attribute.as_deref().unwrap_or("");
        let path = cstr_to_string(auth_data.evtype.get_attribute(path_attr).unwrap_or(b"\0"));

        if cinfo == 0 {
            if data.from_object
                && auth_data.subject.header.id == auth_data.object.as_ref().unwrap().header.id
                && path != "/" // ignore root's possible parent
            {
                let parent_cinfo = auth_data
                    .object
                    .as_ref()
                    .unwrap()
                    .get_object_cinfo()
                    .unwrap();
                cinfo = parent_cinfo;
            }

            if cinfo == 0 {
                node = tree.root();
            } else {
                node = config.node_by_cinfo(&cinfo).expect("node not found");
            }

            let _ = auth_data.subject.clear_object_act();
            let _ = auth_data.subject.clear_subject_act();
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
            auth_data.evtype.header.name,
            path,
            node.path()
        );

        let _ = auth_data
            .subject
            .set_vs(node.virtual_space().to_member_bytes());
        let _ = auth_data
            .subject
            .set_vs_read(node.virtual_space().to_read_bytes());
        let _ = auth_data
            .subject
            .set_vs_write(node.virtual_space().to_write_bytes());
        let _ = auth_data
            .subject
            .set_vs_see(node.virtual_space().to_see_bytes());
        if node.has_children() && auth_data.evtype.header.monitoring == Monitoring::Object {
            let _ = auth_data
                .subject
                .add_object_act(auth_data.evtype.header.monitoring_bit as usize);
            let _ = auth_data
                .subject
                .add_subject_act(auth_data.evtype.header.monitoring_bit as usize);
        }

        auth_data.subject.set_object_cinfo(cinfo).unwrap();

        ctx.update_object_no_wait(&auth_data.subject);
        //ctx.update_object(&auth_data.subject).await;

        MedusaAnswer::Ok
    }
}
