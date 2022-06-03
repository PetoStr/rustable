use crate::medusa::config::Config;
use crate::medusa::{
    FetchAnswer, MedusaClass, MedusaEvtype, MedusaRequest, RequestType, UpdateAnswer, Writer,
};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};

/// Shared context between various asynchronous tasks.
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

    /// Returns configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns identification of a class having the given name.
    pub fn class_id_from_name(&self, class_name: &str) -> Option<u64> {
        self.class_id.get(class_name).map(|x| *x)
    }

    /// Returns identification of an event having the given name.
    pub fn evtype_id_from_name(&self, evtype_name: &str) -> Option<u64> {
        self.evtype_id.get(evtype_name).map(|x| *x)
    }

    /// Returns an empty class having the given id with no attribute data.
    pub fn empty_class_from_id(&self, class_id: &u64) -> Option<MedusaClass> {
        self.classes.get(class_id).map(|x| x.value().clone())
    }

    /// Returns an empty event having the given id with no attribute data.
    pub fn empty_evtype_from_id(&self, evtype_id: &u64) -> Option<MedusaEvtype> {
        self.evtypes.get(evtype_id).map(|x| x.value().clone())
    }

    /// Returns an empty class having the given name with no attribute data.
    pub fn empty_class(&self, class_name: &str) -> Option<MedusaClass> {
        let class_id = self.class_id_from_name(class_name)?;
        self.empty_class_from_id(&class_id)
    }

    /// Returns an empty event having the given name with no attribute data.
    pub fn empty_evtype(&self, evtype_name: &str) -> Option<MedusaEvtype> {
        let evtype_id = self.evtype_id_from_name(evtype_name)?;
        self.empty_evtype_from_id(&evtype_id)
    }

    /// Performs `update` request.
    pub async fn update_request(&self, class_id: u64, data: &[u8]) -> UpdateAnswer {
        let req = MedusaRequest {
            req_type: RequestType::Update,
            class_id,
            id: self.get_new_request_id(),
            data,
        };

        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.update_requests.insert(req.id, sender);

        self.writer.write(Arc::from(req.to_vec()));

        receiver.recv().await.expect("channel is disconnected")
    }

    /// Performs `fetch` request.
    pub async fn fetch_request(&self, class_id: u64, data: &[u8]) -> FetchAnswer {
        let req = MedusaRequest {
            req_type: RequestType::Fetch,
            class_id,
            id: self.get_new_request_id(),
            data,
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
