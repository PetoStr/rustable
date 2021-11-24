use crossbeam_channel::Receiver;
use std::io::prelude::*;
use std::sync::Arc;
use threadfin::{Task, ThreadPool};

pub(crate) struct WriteWorker {
    pub(crate) task: Option<Task<()>>,
}

impl WriteWorker {
    pub(crate) fn new<W: Write + 'static + Send>(
        pool: &ThreadPool,
        mut write_handle: W,
        receiver: Receiver<Arc<[u8]>>,
    ) -> Self {
        let task = pool.execute(move || {
            while let Ok(data) = receiver.recv() {
                write_handle.write_all(data.as_ref()).unwrap();
            }
        });

        Self { task: Some(task) }
    }
}
