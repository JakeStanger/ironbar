use super::wayland::{self, ClipboardItem};
use crate::channels::AsyncSenderExt;
use crate::{arc_mut, lock, register_client, spawn};
use indexmap::map::Iter;
use indexmap::IndexMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, trace};

#[derive(Debug)]
pub enum ClipboardEvent {
    Add(ClipboardItem),
    Remove(usize),
    Activate(usize),
}

type EventSender = mpsc::Sender<ClipboardEvent>;

/// Clipboard client singleton,
/// to ensure bars don't duplicate requests to the compositor.
#[derive(Debug)]
pub struct Client {
    wayland: Arc<wayland::Client>,

    senders: Arc<Mutex<Vec<(EventSender, usize)>>>,
    cache: Arc<Mutex<ClipboardCache>>,
}

impl Client {
    pub(crate) fn new(wl: Arc<wayland::Client>) -> Self {
        trace!("Initializing clipboard client");

        let senders = arc_mut!(Vec::<(EventSender, usize)>::new());

        let cache = arc_mut!(ClipboardCache::new());

        {
            let senders = senders.clone();
            let cache = cache.clone();
            let wl = wl.clone();

            spawn(async move {
                let item = wl.clipboard_item();
                let mut rx = wl.subscribe_clipboard();

                if let Some(item) = item {
                    let senders = lock!(senders);
                    let iter = senders.iter();
                    for (tx, _) in iter {
                        tx.send_spawn(ClipboardEvent::Add(item.clone()));
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
                                    tx.send_spawn(ClipboardEvent::Remove(removed_id));
                                }
                                tx.send_spawn(ClipboardEvent::Add(item.clone()));
                            }
                        },
                        |existing_id| {
                            let senders = lock!(senders);
                            let iter = senders.iter();
                            for (tx, _) in iter {
                                tx.send_spawn(ClipboardEvent::Activate(existing_id));
                            }
                        },
                    );
                }
            });
        }

        Self {
            wayland: wl,
            senders,
            cache,
        }
    }

    pub fn subscribe(&self, cache_size: usize) -> mpsc::Receiver<ClipboardEvent> {
        let (tx, rx) = mpsc::channel(16);

        {
            let cache = lock!(self.cache);

            let iter = cache.iter();
            for (_, (item, _)) in iter {
                tx.send_spawn(ClipboardEvent::Add(item.clone()));
            }
        }

        lock!(self.senders).push((tx, cache_size));

        rx
    }

    pub fn copy(&self, id: usize) {
        debug!("Copying item with id {id}");

        let item = {
            let cache = lock!(self.cache);
            cache.get(id)
        };

        if let Some(item) = item {
            self.wayland.copy_to_clipboard(item);
        }

        let senders = lock!(self.senders);
        let iter = senders.iter();
        for (tx, _) in iter {
            tx.send_spawn(ClipboardEvent::Activate(id));
        }
    }

    pub fn remove(&self, id: usize) {
        lock!(self.cache).remove(id);

        let senders = lock!(self.senders);
        let iter = senders.iter();
        for (tx, _) in iter {
            tx.send_spawn(ClipboardEvent::Remove(id));
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
    cache: IndexMap<usize, (ClipboardItem, usize)>,
}

impl ClipboardCache {
    /// Creates a new empty cache.
    fn new() -> Self {
        Self {
            cache: IndexMap::new(),
        }
    }

    /// Gets the entry with key `id` from the cache.
    fn get(&self, id: usize) -> Option<ClipboardItem> {
        self.cache.get(&id).map(|(item, _)| item).cloned()
    }

    /// Inserts an entry with `ref_count` initial references.
    fn insert(&mut self, item: ClipboardItem, ref_count: usize) -> Option<ClipboardItem> {
        self.cache
            .insert(item.id, (item, ref_count))
            .map(|(item, _)| item)
    }

    /// Removes the entry with key `id`.
    /// This ignores references.
    fn remove(&mut self, id: usize) -> Option<ClipboardItem> {
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

    fn iter(&self) -> Iter<'_, usize, (ClipboardItem, usize)> {
        self.cache.iter()
    }
}

register_client!(Client, clipboard);
