use crate::modules::ModuleUpdateEvent;
use crate::spawn;
use smithay_client_toolkit::reexports::calloop;
use std::fmt::Debug;
use tokio::sync::{broadcast, mpsc};

pub trait SyncSenderExt<T> {
    /// Asynchronously sends a message on the channel,
    /// panicking if it cannot be sent.
    ///
    /// This should be used in cases where sending should *never* fail,
    /// or where failing indicates a serious bug.
    fn send_expect(&self, message: T);
}

impl<T> SyncSenderExt<T> for std::sync::mpsc::Sender<T> {
    #[inline]
    fn send_expect(&self, message: T) {
        self.send(message).expect(crate::error::ERR_CHANNEL_SEND);
    }
}

impl<T> SyncSenderExt<T> for calloop::channel::Sender<T> {
    #[inline]
    fn send_expect(&self, message: T) {
        self.send(message).expect(crate::error::ERR_CHANNEL_SEND);
    }
}

impl<T: Debug> SyncSenderExt<T> for broadcast::Sender<T> {
    #[inline]
    fn send_expect(&self, message: T) {
        self.send(message).expect(crate::error::ERR_CHANNEL_SEND);
    }
}

pub trait AsyncSenderExt<T>: Sync + Send + Sized + Clone {
    /// Asynchronously sends a message on the channel,
    /// panicking if it cannot be sent.
    ///
    /// This should be used in cases where sending should *never* fail,
    /// or where failing indicates a serious bug.
    fn send_expect(&self, message: T) -> impl std::future::Future<Output = ()> + Send;

    /// Asynchronously sends a message on the channel,
    /// spawning a task to allow it to be sent in the background,
    /// and panicking if it cannot be sent.
    ///
    /// Note that this function will return *before* the message is sent.
    ///
    /// This should be used in cases where sending should *never* fail,
    /// or where failing indicates a serious bug.
    #[inline]
    fn send_spawn(&self, message: T)
    where
        Self: 'static,
        T: Send + 'static,
    {
        let tx = self.clone();
        spawn(async move { tx.send_expect(message).await });
    }

    /// Shorthand for [`AsyncSenderExt::send_expect`]
    /// when sending a [`ModuleUpdateEvent::Update`].
    #[inline]
    async fn send_update<U: Clone>(&self, update: U)
    where
        Self: AsyncSenderExt<ModuleUpdateEvent<U>>,
    {
        self.send_expect(ModuleUpdateEvent::Update(update)).await;
    }

    /// Shorthand for [`AsyncSenderExt::send_spawn`]
    /// when sending a [`ModuleUpdateEvent::Update`].
    #[inline]
    fn send_update_spawn<U>(&self, update: U)
    where
        Self: AsyncSenderExt<ModuleUpdateEvent<U>> + 'static,
        U: Clone + Send + 'static,
    {
        self.send_spawn(ModuleUpdateEvent::Update(update));
    }
}

impl<T: Send> AsyncSenderExt<T> for mpsc::Sender<T> {
    #[inline]
    async fn send_expect(&self, message: T) {
        self.send(message)
            .await
            .expect(crate::error::ERR_CHANNEL_SEND);
    }
}

pub trait MpscReceiverExt<T> {
    /// Spawns a `GLib` future on the local thread, and calls `rx.recv()`
    /// in a loop, passing the message to `f`.
    ///
    /// This allows use of `GObjects` and futures in the same context.
    fn recv_glib<F>(self, f: F)
    where
        F: FnMut(T) + 'static;
}

impl<T: 'static> MpscReceiverExt<T> for mpsc::Receiver<T> {
    fn recv_glib<F>(mut self, mut f: F)
    where
        F: FnMut(T) + 'static,
    {
        glib::spawn_future_local(async move {
            while let Some(val) = self.recv().await {
                f(val);
            }
        });
    }
}

pub trait BroadcastReceiverExt<T>
where
    T: Debug + Clone + 'static,
{
    /// Spawns a `GLib` future on the local thread, and calls `rx.recv()`
    /// in a loop, passing the message to `f`.
    ///
    /// This allows use of `GObjects` and futures in the same context.
    fn recv_glib<F>(self, f: F)
    where
        F: FnMut(T) + 'static;
}

impl<T> BroadcastReceiverExt<T> for broadcast::Receiver<T>
where
    T: Debug + Clone + 'static,
{
    fn recv_glib<F>(mut self, mut f: F)
    where
        F: FnMut(T) + 'static,
    {
        glib::spawn_future_local(async move {
            loop {
                match self.recv().await {
                    Ok(val) => f(val),
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!("Channel lagged behind by {count}, this may result in unexpected or broken behaviour");
                    }
                    Err(err) => {
                        tracing::error!("{err:?}");
                        break;
                    }
                }
            }
        });
    }
}
