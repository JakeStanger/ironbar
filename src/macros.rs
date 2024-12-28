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
