use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub(crate) struct Writer {
    sender: UnboundedSender<Arc<[u8]>>,
}

impl Writer {
    pub(crate) fn new<W>(mut write_handle: W) -> Self
    where
        W: AsyncWriteExt + Unpin + Send + 'static,
    {
        let (sender, mut receiver): (_, UnboundedReceiver<Arc<[u8]>>) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                write_handle.write_all(data.as_ref()).await.unwrap();
            }
        });

        Self { sender }
    }

    pub(crate) fn write(&self, data: Arc<[u8]>) {
        self.sender.send(data).expect("writer is disconnected");
    }
}
