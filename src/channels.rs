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
    fn send_expect(&self, message: T) -> impl Future<Output = ()> + Send;

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
    /// This allows use of `GObjects` and futures in the same context.#
    ///
    /// `deps` is a single reference, or tuple of references of clonable objects,
    /// to be consumed inside the closure.
    /// This avoids needing to `element.clone()` everywhere.
    fn recv_glib<D, Fn>(self, deps: D, f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) + 'static;
}

impl<T: 'static> MpscReceiverExt<T> for mpsc::Receiver<T> {
    fn recv_glib<D, Fn>(mut self, deps: D, mut f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) + 'static,
    {
        let deps = deps.clone_content();
        glib::spawn_future_local(async move {
            while let Some(val) = self.recv().await {
                f(&deps, val);
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
    ///
    /// `deps` is a single reference, or tuple of references of clonable objects,
    /// to be consumed inside the closure.
    /// This avoids needing to `element.clone()` everywhere.
    fn recv_glib<D, Fn>(self, deps: D, f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) + 'static;

    /// Like [`BroadcastReceiverExt::recv_glib`], but the closure must return a [`Future`].
    fn recv_glib_async<D, Fn, F>(self, deps: D, f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) -> F + 'static,
        F: Future;
}

impl<T> BroadcastReceiverExt<T> for broadcast::Receiver<T>
where
    T: Debug + Clone + 'static,
{
    fn recv_glib<D, Fn>(mut self, deps: D, mut f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) + 'static,
    {
        let deps = deps.clone_content();
        glib::spawn_future_local(async move {
            loop {
                match self.recv().await {
                    Ok(val) => f(&deps, val),
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(
                            "Channel lagged behind by {count}, this may result in unexpected or broken behaviour"
                        );
                    }
                    Err(err) => {
                        tracing::error!("{err:?}");
                        break;
                    }
                }
            }
        });
    }

    fn recv_glib_async<D, Fn, F>(mut self, deps: D, mut f: Fn)
    where
        D: Dependency,
        D::Target: Clone + 'static,
        Fn: FnMut(&D::Target, T) -> F + 'static,
        F: Future,
    {
        let deps = deps.clone_content();
        glib::spawn_future_local(async move {
            loop {
                match self.recv().await {
                    Ok(val) => {
                        f(&deps, val).await;
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!(
                            "Channel lagged behind by {count}, this may result in unexpected or broken behaviour"
                        );
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

/// `recv_glib` callback dependency
/// or dependency tuple.
pub trait Dependency: Clone {
    type Target;

    fn clone_content(&self) -> Self::Target;
}

impl Dependency for () {
    type Target = ();

    fn clone_content(&self) -> Self::Target {}
}

impl<'a, T> Dependency for &'a T
where
    T: Clone + 'a,
{
    type Target = T;

    fn clone_content(&self) -> T {
        T::clone(self)
    }
}

macro_rules! impl_dependency {
    ($($idx:tt $t:ident),+) => {
        impl<'a, $($t),+> Dependency for ($(&'a $t),+)
            where
                $($t: Clone + 'a),+
            {
                type Target =  ($($t),+);

                fn clone_content(&self) -> Self::Target {
                     ($(self.$idx.clone()),+)
                }
            }
    };
}

impl_dependency!(0 T1, 1 T2);
impl_dependency!(0 T1, 1 T2, 2 T3);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7, 7 T8);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7, 7 T8, 8 T9);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7, 7 T8, 8 T9, 9 T10);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7, 7 T8, 8 T9, 9 T10, 10 T11);
impl_dependency!(0 T1, 1 T2, 2 T3, 3 T4, 4 T5, 5 T6, 6 T7, 7 T8, 8 T9, 9 T10, 10 T11, 11 T12);
