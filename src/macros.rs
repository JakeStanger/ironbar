/// Sends a message on an asynchronous `Sender` using `send()`
/// Panics if the message cannot be sent.
///
/// Usage:
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
/// Usage:
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
/// Usage:
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

/// Locks a `Mutex`.
/// Panics if the `Mutex` cannot be locked.
///
/// Usage:
///
/// ```rs
/// let mut val = lock!(my_mutex);
/// ```
#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex.lock().expect($crate::error::ERR_MUTEX_LOCK)
    };
}

/// Gets a read lock on a `RwLock`.
/// Panics if the `RwLock` cannot be locked.
///
/// Usage:
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
/// Usage:
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
