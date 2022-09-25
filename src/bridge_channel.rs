use tokio::spawn;
use tokio::sync::mpsc;

/// MPSC async -> sync channel.
/// The sender uses `tokio::sync::mpsc`
/// while the receiver uses `glib::MainContext::channel`.
///
/// This makes it possible to send events asynchronously
/// and receive them on the main thread,
/// allowing UI updates to be handled on the receiving end.
pub struct BridgeChannel<T> {
    async_tx: mpsc::Sender<T>,
    sync_rx: glib::Receiver<T>,
}

impl<T: Send + 'static> BridgeChannel<T> {
    /// Creates a new channel
    pub fn new() -> Self {
        let (async_tx, mut async_rx) = mpsc::channel(32);
        let (sync_tx, sync_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        spawn(async move {
            while let Some(val) = async_rx.recv().await {
                sync_tx.send(val).expect("Failed to send message");
            }
        });

        Self { async_tx, sync_rx }
    }

    /// Gets a clone of the sender.
    pub fn create_sender(&self) -> mpsc::Sender<T> {
        self.async_tx.clone()
    }

    /// Attaches a callback to the receiver.
    pub fn recv<F>(self, f: F) -> glib::SourceId
    where
        F: FnMut(T) -> glib::Continue + 'static,
    {
        self.sync_rx.attach(None, f)
    }
}
