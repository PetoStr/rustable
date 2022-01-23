use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub(crate) struct Writer {
    sender: UnboundedSender<Arc<[u8]>>,
}

impl Writer {
    pub(crate) fn new<W>(mut write_handle: W, mut shutdown_notify: broadcast::Receiver<()>) -> Self
    where
        W: AsyncWriteExt + Unpin + Send + 'static,
    {
        let (sender, mut receiver): (_, UnboundedReceiver<Arc<[u8]>>) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv() => {
                        match data {
                            Some(ref ref_data) => write_handle.write_all(ref_data).await.unwrap(),
                            None => break
                        }
                    }
                    _ = shutdown_notify.recv() => break,
                }
            }
        });

        Self { sender }
    }

    pub(crate) fn write(&self, data: Arc<[u8]>) {
        self.sender.send(data).expect("writer is disconnected");
    }
}
