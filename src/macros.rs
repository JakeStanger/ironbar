/// Provides implementations of methods required by the `Module` trait
/// which cannot be included as part of the trait.
///
/// This removes the need to add the same boilerplate method definitions
/// to every module implementation.
///
/// # Usage:
///
/// ```rs
/// impl Module for ClockModule {
///    type SendMessage = DateTime<Local>;
///    type ReceiveMessage = ();
///
///    module_impl!("clock");
/// }
#[macro_export]
macro_rules! module_impl {
    ($name:literal) => {
        fn name() -> &'static str {
            $name
        }

        fn take_common(&mut self) -> $crate::config::CommonConfig {
            self.common.take().unwrap_or_default()
        }
    };
}

/// Sends a message on an asynchronous `Sender` using `send()`
/// Panics if the message cannot be sent.
///
/// # Usage:
///
/// ```rs
/// send_async!(tx, "my message");
/// ```
#[macro_export]
macro_rules! send_async {
    ($tx:expr, $msg:expr) => {
        $tx.send($msg).await.expect($crate::error::ERR_CHANNEL_SEND)
    };
}

/// Sends a message on a synchronous `Sender` using `send()`
/// Panics if the message cannot be sent.
///
/// # Usage:
///
/// ```rs
/// send!(tx, "my message");
/// ```
#[macro_export]
macro_rules! send {
    ($tx:expr, $msg:expr) => {
        $tx.send($msg).expect($crate::error::ERR_CHANNEL_SEND)
    };
}

/// Sends a message on a synchronous `Sender` using `try_send()`
/// Panics if the message cannot be sent.
///
/// # Usage:
///
/// ```rs
/// try_send!(tx, "my message");
/// ```
#[macro_export]
macro_rules! try_send {
    ($tx:expr, $msg:expr) => {
        $tx.try_send($msg).expect($crate::error::ERR_CHANNEL_SEND)
    };
}

/// Sends a message, wrapped inside a `ModuleUpdateEvent::Update` variant,
/// on an asynchronous `Sender` using `send()`.
///
/// This is a convenience wrapper around `send_async`
/// to avoid needing to write the full enum every time.
///
/// Panics if the message cannot be sent.
///
/// # Usage:
///
/// ```rs
/// module_update!(tx, "my event");
/// ```
#[macro_export]
macro_rules! module_update {
    ($tx:expr, $msg:expr) => {
        send_async!($tx, $crate::modules::ModuleUpdateEvent::Update($msg))
    };
}

/// Spawns a `GLib` future on the local thread, and calls `rx.recv()`
/// in a loop.
///
/// This allows use of `GObjects` and futures in the same context.
///
/// For use with receivers which return a `Result`.
///
/// # Example
///
/// ```rs
/// let (tx, mut rx) = broadcast::channel(32);
/// glib_recv(rx, msg => println!("{msg}"));
/// ```
#[macro_export]
macro_rules! glib_recv {
    ($rx:expr, $func:ident) => { glib_recv!($rx, ev => $func(ev)) };

    ($rx:expr, $val:ident => $expr:expr) => {{
        glib::spawn_future_local(async move {
            // re-delcare in case ie `context.subscribe()` is passed directly
            let mut rx = $rx;
            loop {
                match rx.recv().await {
                    Ok($val) => $expr,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        tracing::warn!("Channel lagged behind by {count}, this may result in unexpected or broken behaviour");
                    }
                    Err(err) => {
                        tracing::error!("{err:?}");
                        break;
                    }
                }
            }
        });
    }};
}

/// Spawns a `GLib` future on the local thread, and calls `rx.recv()`
/// in a loop.
///
/// This allows use of `GObjects` and futures in the same context.
///
/// For use with receivers which return an `Option`,
/// such as Tokio's `mpsc` channel.
///
/// # Example
///
/// ```rs
/// let (tx, mut rx) = broadcast::channel(32);
/// glib_recv_mpsc(rx, msg => println!("{msg}"));
/// ```
#[macro_export]
macro_rules! glib_recv_mpsc {
    ($rx:expr, $func:ident) => { glib_recv_mpsc!($rx, ev => $func(ev)) };

    ($rx:expr, $val:ident => $expr:expr) => {{
        glib::spawn_future_local(async move {
            // re-delcare in case ie `context.subscribe()` is passed directly
            let mut rx = $rx;
            while let Some($val) = rx.recv().await {
                $expr
            }
        });
    }};
}

/// Locks a `Mutex`.
/// Panics if the `Mutex` cannot be locked.
///
/// # Usage:
///
/// ```rs
/// let mut val = lock!(my_mutex);
/// ```
#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {{
        tracing::trace!("Locking {}", std::stringify!($mutex));
        $mutex.lock().expect($crate::error::ERR_MUTEX_LOCK)
    }};
}

/// Gets a read lock on a `RwLock`.
/// Panics if the `RwLock` cannot be locked.
///
/// # Usage:
///
/// ```rs
/// let val = read_lock!(my_rwlock);
/// ```
#[macro_export]
macro_rules! read_lock {
    ($rwlock:expr) => {
        $rwlock.read().expect($crate::error::ERR_READ_LOCK)
    };
}

/// Gets a write lock on a `RwLock`.
/// Panics if the `RwLock` cannot be locked.
///
/// # Usage:
///
/// ```rs
/// let mut val = write_lock!(my_rwlock);
/// ```
#[macro_export]
macro_rules! write_lock {
    ($rwlock:expr) => {
        $rwlock.write().expect($crate::error::ERR_WRITE_LOCK)
    };
}

/// Wraps `val` in a new `Arc<Mutex<T>>`.
///
/// # Usage:
///
/// ```rs
/// let val = arc_mut!(MyService::new());
/// ```
///
#[macro_export]
macro_rules! arc_mut {
    ($val:expr) => {
        std::sync::Arc::new(std::sync::Mutex::new($val))
    };
}

/// Wraps `val` in a new `Arc<RwLock<T>>`.
///
/// # Usage:
///
/// ```rs
/// let val = arc_rw!(MyService::new());
/// ```
///
#[macro_export]
macro_rules! arc_rw {
    ($val:expr) => {
        std::sync::Arc::new(std::sync::RwLock::new($val))
    };
}

/// Wraps `val` in a new `Rc<RefCell<T>>`.
///
/// # Usage
///
/// ```rs
/// let val = rc_mut!(MyService::new())
/// ```
#[macro_export]
macro_rules! rc_mut {
    ($val:expr) => {
        std::rc::Rc::new(std::cell::RefCell::new($val))
    };
}
