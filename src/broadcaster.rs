use tokio::sync::mpsc::{self, error::{TrySendError, SendError}};
use tokio::spawn;

/// Crossbeam channel wrapper
/// which sends messages to all receivers.
/// TODO: Replace with tokio::sync::broadcast
pub struct Broadcaster<T> {
    channels: Vec<crossbeam_channel::Sender<T>>,
}

impl<T: 'static + Clone + Send + Sync> Broadcaster<T> {
    /// Creates a new broadcaster.
    pub const fn new() -> Self {
        Self { channels: vec![] }
    }

    /// Creates a new sender/receiver pair.
    /// The sender is stored locally and the receiver is returned.
    pub fn subscribe(&mut self) -> crossbeam_channel::Receiver<T> {
        let (tx, rx) = crossbeam_channel::unbounded();

        self.channels.push(tx);

        rx
    }

    /// Attempts to send a messsge to all receivers.
    pub fn send(&self, message: T) -> Result<(), crossbeam_channel::SendError<T>> {
        for c in &self.channels {
            c.send(message.clone())?;
        }

        Ok(())
    }
}

pub struct GLibBroadcaster<T> {
    channels: Vec<glib::Sender<T>>,
}

impl<T: Clone> GLibBroadcaster<T> {
    pub const fn new() -> Self {
        Self { channels: vec![] }
    }

    pub fn subscribe(&mut self) -> glib::Receiver<T> {
        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        self.channels.push(tx);
        rx
    }

    pub fn send(&self, message: T) -> Result<(), std::sync::mpsc::SendError<T>> {
        for c in &self.channels {
            c.send(message.clone())?;
        }

        Ok(())
    }
}

pub struct BridgeChannel<T> {
    async_tx: mpsc::Sender<T>,
    async_rx: mpsc::Receiver<T>,

    sync_tx: glib::Sender<T>,
    sync_rx: glib::Receiver<T>
}

impl<T: Send> BridgeChannel<T> {
    pub fn new() -> Self {
        let (async_tx, async_rx) = mpsc::channel(32);
        let (sync_tx, sync_rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);

        spawn(async move {
            while let Some(val) = async_rx.recv().await {
                sync_tx.send(val);
            }
        });

        Self { async_tx, async_rx, sync_tx, sync_rx }
    }

    pub fn create_sender(&self) -> mpsc::Sender<T> {
        self.async_tx.clone()
    }

    pub async fn send(&self, msg: T) -> Result<(), SendError<T>> {
        self.async_tx.send(msg).await
    }

    pub fn try_send(&self, msg: T) -> Result<(), TrySendError<T>> {
        self.async_tx.try_send(msg)
    } 

    fn recv<F>(&self, f: F) -> glib::SourceId
    where F: FnMut(T) -> glib::Continue + 'static,
     {
        self.sync_rx.attach(None, f)
    }
}
