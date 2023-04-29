use super::wayland::{self, ClipboardItem};
use crate::{lock, try_send};
use indexmap::map::Iter;
use indexmap::IndexMap;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use tokio::spawn;
use tokio::sync::mpsc;
use tracing::{debug, trace};

#[derive(Debug)]
pub enum ClipboardEvent {
    Add(Arc<ClipboardItem>),
    Remove(usize),
    Activate(usize),
}

type EventSender = mpsc::Sender<ClipboardEvent>;

/// Clipboard client singleton,
/// to ensure bars don't duplicate requests to the compositor.
pub struct ClipboardClient {
    senders: Arc<Mutex<Vec<(EventSender, usize)>>>,
    cache: Arc<Mutex<ClipboardCache>>,
}

impl ClipboardClient {
    fn new() -> Self {
        trace!("Initializing clipboard client");

        let senders = Arc::new(Mutex::new(Vec::<(EventSender, usize)>::new()));

        let cache = Arc::new(Mutex::new(ClipboardCache::new()));

        {
            let senders = senders.clone();
            let cache = cache.clone();

            spawn(async move {
                let (mut rx, item) = {
                    let wl = wayland::get_client().await;
                    wl.subscribe_clipboard()
                };

                if let Some(item) = item {
                    let senders = lock!(senders);
                    let iter = senders.iter();
                    for (tx, _) in iter {
                        try_send!(tx, ClipboardEvent::Add(item.clone()));
                    }

                    lock!(cache).insert(item, senders.len());
                }

                while let Ok(item) = rx.recv().await {
                    debug!("Received clipboard item (ID: {})", item.id);

                    let (existing_id, cache_size) = {
                        let cache = lock!(cache);
                        (cache.contains(&item), cache.len())
                    };

                    existing_id.map_or_else(
                        || {
                            {
                                let mut cache = lock!(cache);
                                let senders = lock!(senders);
                                cache.insert(item.clone(), senders.len());
                            }
                            let senders = lock!(senders);
                            let iter = senders.iter();
                            for (tx, sender_cache_size) in iter {
                                if cache_size == *sender_cache_size {
                                    let removed_id = lock!(cache)
                                        .remove_ref_first()
                                        .expect("Clipboard cache unexpectedly empty");
                                    try_send!(tx, ClipboardEvent::Remove(removed_id));
                                }
                                try_send!(tx, ClipboardEvent::Add(item.clone()));
                            }
                        },
                        |existing_id| {
                            let senders = lock!(senders);
                            let iter = senders.iter();
                            for (tx, _) in iter {
                                try_send!(tx, ClipboardEvent::Activate(existing_id));
                            }
                        },
                    );
                }
            });
        }

        Self { senders, cache }
    }

    pub fn subscribe(&self, cache_size: usize) -> mpsc::Receiver<ClipboardEvent> {
        let (tx, rx) = mpsc::channel(16);

        {
            let cache = lock!(self.cache);

            let iter = cache.iter();
            for (_, (item, _)) in iter {
                try_send!(tx, ClipboardEvent::Add(item.clone()));
            }
        }

        lock!(self.senders).push((tx, cache_size));

        rx
    }

    pub async fn copy(&self, id: usize) {
        debug!("Copying item with id {id}");

        let item = {
            let cache = lock!(self.cache);
            cache.get(id)
        };

        if let Some(item) = item {
            let wl = wayland::get_client().await;
            wl.copy_to_clipboard(item);
        }

        let senders = lock!(self.senders);
        let iter = senders.iter();
        for (tx, _) in iter {
            try_send!(tx, ClipboardEvent::Activate(id));
        }
    }

    pub fn remove(&self, id: usize) {
        lock!(self.cache).remove(id);

        let senders = lock!(self.senders);
        let iter = senders.iter();
        for (tx, _) in iter {
            try_send!(tx, ClipboardEvent::Remove(id));
        }
    }
}

/// Shared clipboard item cache.
///
/// Items are stored with a number of references,
/// allowing different consumers to 'remove' cached items
/// at different times.
#[derive(Debug)]
struct ClipboardCache {
    cache: IndexMap<usize, (Arc<ClipboardItem>, usize)>,
}

impl ClipboardCache {
    /// Creates a new empty cache.
    fn new() -> Self {
        Self {
            cache: IndexMap::new(),
        }
    }

    /// Gets the entry with key `id` from the cache.
    fn get(&self, id: usize) -> Option<Arc<ClipboardItem>> {
        self.cache.get(&id).map(|(item, _)| item).cloned()
    }

    /// Inserts an entry with `ref_count` initial references.
    fn insert(&mut self, item: Arc<ClipboardItem>, ref_count: usize) -> Option<Arc<ClipboardItem>> {
        self.cache
            .insert(item.id, (item, ref_count))
            .map(|(item, _)| item)
    }

    /// Removes the entry with key `id`.
    /// This ignores references.
    fn remove(&mut self, id: usize) -> Option<Arc<ClipboardItem>> {
        self.cache.shift_remove(&id).map(|(item, _)| item)
    }

    /// Removes a reference to the entry with key `id`.
    ///
    /// If the reference count reaches zero, the entry
    /// is removed from the cache.
    fn remove_ref(&mut self, id: usize) {
        if let Some(entry) = self.cache.get_mut(&id) {
            entry.1 -= 1;

            if entry.1 == 0 {
                self.cache.shift_remove(&id);
            }
        }
    }

    /// Removes a reference to the first entry.
    ///
    /// If the reference count reaches zero, the entry
    /// is removed from the cache.
    fn remove_ref_first(&mut self) -> Option<usize> {
        if let Some((id, _)) = self.cache.first() {
            let id = *id;
            self.remove_ref(id);
            Some(id)
        } else {
            None
        }
    }

    /// Checks if an item with matching mime type and value
    /// already exists in the cache.
    fn contains(&self, item: &ClipboardItem) -> Option<usize> {
        self.cache.values().find_map(|(it, _)| {
            if it.mime_type == item.mime_type && it.value == item.value {
                Some(it.id)
            } else {
                None
            }
        })
    }

    /// Gets the current number of items in the cache.
    fn len(&self) -> usize {
        self.cache.len()
    }

    fn iter(&self) -> Iter<'_, usize, (Arc<ClipboardItem>, usize)> {
        self.cache.iter()
    }
}

lazy_static! {
    static ref CLIENT: ClipboardClient = ClipboardClient::new();
}

pub fn get_client() -> &'static ClipboardClient {
    &CLIENT
}
