use crate::medusa::config::Config;
use crate::medusa::{
    AuthRequestData, FetchAnswer, MedusaClass, MedusaEvtype, MedusaRequest, Monitoring,
    RequestType, UpdateAnswer, Writer,
};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};

pub struct Context {
    pub(crate) classes: DashMap<u64, MedusaClass>,
    pub(crate) evtypes: DashMap<u64, MedusaEvtype>,

    pub(crate) fetch_requests: DashMap<u64, UnboundedSender<FetchAnswer>>,
    pub(crate) update_requests: DashMap<u64, UnboundedSender<UpdateAnswer>>,

    pub(crate) class_id: DashMap<String, u64>,
    pub(crate) evtype_id: DashMap<String, u64>,

    pub(crate) writer: Writer,

    pub(crate) config: Config,

    request_id_cn: AtomicU64,
}

impl Context {
    pub(crate) fn new(writer: Writer, config: Config) -> Self {
        Self {
            classes: DashMap::new(),
            evtypes: DashMap::new(),
            fetch_requests: DashMap::new(),
            update_requests: DashMap::new(),
            class_id: DashMap::new(),
            evtype_id: DashMap::new(),
            writer,
            config,
            request_id_cn: AtomicU64::new(111),
        }
    }

    pub async fn enter_tree(
        &self,
        auth_data: &mut AuthRequestData,
        primary_tree: &str,
        path: &str,
    ) {
        assert!(path.starts_with('/'));

        let tree = self
            .config
            .tree_by_name(primary_tree)
            .unwrap_or_else(|| panic!("primary tree `{}` not found", primary_tree));

        let mut node = tree.root();
        for part in path.split_terminator('/') {
            node = node.child_by_path(part).unwrap();
        }

        let _ = auth_data.subject.clear_object_act();
        let _ = auth_data.subject.clear_subject_act();

        let cinfo = Arc::as_ptr(node) as usize;

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

        self.update_object(&auth_data.subject).await;
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn class_id_from_name(&self, class_name: &str) -> Option<u64> {
        self.class_id.get(class_name).map(|x| *x)
    }

    pub fn evtype_id_from_name(&self, evtype_name: &str) -> Option<u64> {
        self.evtype_id.get(evtype_name).map(|x| *x)
    }

    // class with empty attribute data
    pub fn empty_class_from_id(&self, class_id: &u64) -> Option<MedusaClass> {
        self.classes.get(class_id).map(|x| x.value().clone())
    }

    // evtype with empty attribute data
    pub fn empty_evtype_from_id(&self, evtype_id: &u64) -> Option<MedusaEvtype> {
        self.evtypes.get(evtype_id).map(|x| x.value().clone())
    }

    pub fn empty_class(&self, class_name: &str) -> Option<MedusaClass> {
        let class_id = self.class_id_from_name(class_name)?;
        self.empty_class_from_id(&class_id)
    }

    pub fn empty_evtype(&self, evtype_name: &str) -> Option<MedusaEvtype> {
        let evtype_id = self.evtype_id_from_name(evtype_name)?;
        self.empty_evtype_from_id(&evtype_id)
    }

    pub fn update_object_no_wait(&self, object: &MedusaClass) {
        let req = MedusaRequest {
            req_type: RequestType::Update,
            class_id: object.header.id,
            id: self.get_new_request_id(),
            data: &object.pack_attributes(),
        };

        self.writer.write(Arc::from(req.to_vec()));
    }

    pub async fn update_object(&self, object: &MedusaClass) -> UpdateAnswer {
        let req = MedusaRequest {
            req_type: RequestType::Update,
            class_id: object.header.id,
            id: self.get_new_request_id(),
            data: &object.pack_attributes(),
        };

        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.update_requests.insert(req.id, sender);

        self.writer.write(Arc::from(req.to_vec()));

        receiver.recv().await.expect("channel is disconnected")
    }

    pub async fn fetch_object(&self, object: &MedusaClass) -> FetchAnswer {
        let req = MedusaRequest {
            req_type: RequestType::Fetch,
            class_id: object.header.id,
            id: self.get_new_request_id(),
            data: &object.pack_attributes(),
        };

        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.fetch_requests.insert(req.id, sender);

        self.writer.write(Arc::from(req.to_vec()));

        receiver.recv().await.expect("channel is disconnected")
    }

    fn get_new_request_id(&self) -> u64 {
        self.request_id_cn.fetch_add(1, Ordering::SeqCst)
    }
}
