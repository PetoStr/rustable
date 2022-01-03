use crate::medusa::Writer;
use crate::medusa::*;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};

#[derive(Clone)]
pub struct SharedContext {
    pub(crate) classes: Arc<DashMap<u64, MedusaClass>>,
    pub(crate) evtypes: Arc<DashMap<u64, MedusaEvtype>>,

    pub(crate) fetch_requests: Arc<DashMap<u64, UnboundedSender<FetchAnswer>>>,
    pub(crate) update_requests: Arc<DashMap<u64, UnboundedSender<UpdateAnswer>>>,

    pub(crate) class_id: Arc<DashMap<String, u64>>,
    pub(crate) evtype_id: Arc<DashMap<String, u64>>,

    pub(crate) writer: Arc<Writer>,

    request_id_cn: Arc<AtomicU64>,
}

impl SharedContext {
    pub(crate) fn new(writer: Writer) -> Self {
        Self {
            classes: Arc::new(DashMap::new()),
            evtypes: Arc::new(DashMap::new()),
            fetch_requests: Arc::new(DashMap::new()),
            update_requests: Arc::new(DashMap::new()),
            class_id: Arc::new(DashMap::new()),
            evtype_id: Arc::new(DashMap::new()),
            writer: Arc::new(writer),
            request_id_cn: Arc::new(AtomicU64::new(111)),
        }
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
        self.request_id_cn.fetch_add(1, Ordering::Relaxed)
    }
}
