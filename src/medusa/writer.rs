use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) struct WriteWorker;

impl WriteWorker {
    pub(crate) async fn spawn<W: AsyncWriteExt + Unpin + Send + 'static>(
        mut write_handle: W,
        mut receiver: UnboundedReceiver<Arc<[u8]>>,
    ) {
        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                write_handle.write_all(data.as_ref()).await.unwrap();
            }
        });
    }
}
