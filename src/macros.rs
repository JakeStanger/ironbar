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

/// Sends a message on an synchronous `Sender` using `send()`
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

/// Sends a message on an synchronous `Sender` using `try_send()`
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
    ($rx:expr, $val:ident => $expr:expr) => {{
        glib::spawn_future_local(async move {
            // re-delcare in case ie `context.subscribe()` is passed directly
            let mut rx = $rx;
            while let Ok($val) = rx.recv().await {
                $expr
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
