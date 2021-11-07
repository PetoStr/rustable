use crossbeam_channel::Receiver;
use std::io::prelude::*;
use std::sync::Arc;
use std::thread;

pub(crate) struct WriteWorker {
    pub(crate) thread: Option<thread::JoinHandle<()>>,
}

impl WriteWorker {
    pub(crate) fn new<W: Write + 'static + Send>(
        mut write_handle: W,
        receiver: Receiver<Arc<[u8]>>,
    ) -> Self {
        let thread = thread::spawn(move || {
            while let Ok(data) = receiver.recv() {
                write_handle.write_all(data.as_ref()).unwrap();
            }
        });

        Self {
            thread: Some(thread),
        }
    }
}
