use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(1);

/// Gets a `usize` ID value that is unique to the entire Ironbar instance.
/// This is just an `AtomicUsize` that increments every time this function is called.
pub fn get_unique_usize() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
