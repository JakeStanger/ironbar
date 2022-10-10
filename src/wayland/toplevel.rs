use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicUsize, Ordering};
use wayland_client::{DispatchData, Main};
use wayland_protocols::wlr::unstable::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{Event, ZwlrForeignToplevelHandleV1};

const STATE_ACTIVE: u32 = 2;
const STATE_FULLSCREEN: u32 = 3;

static COUNTER: AtomicUsize = AtomicUsize::new(1);
fn get_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Default)]
pub struct ToplevelInfo {
    pub id: usize,
    pub app_id: String,
    pub title: String,
    pub active: bool,
    pub fullscreen: bool,

    ready: bool,
}

impl ToplevelInfo {
    fn new() -> Self {
        let id = get_id();
        Self {
            id,
            ..Default::default()
        }
    }
}

pub struct Toplevel;

#[derive(Debug, Clone)]
pub struct ToplevelEvent {
    pub toplevel: ToplevelInfo,
    pub change: ToplevelChange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToplevelChange {
    New,
    Close,
    Title(String),
    Focus(bool),
    Fullscreen(bool),
}

fn toplevel_implem<F>(event: Event, info: &mut ToplevelInfo, implem: &mut F, ddata: DispatchData)
where
    F: FnMut(ToplevelEvent, DispatchData),
{
    let change = match event {
        Event::AppId { app_id } => {
            info.app_id = app_id;
            None
        }
        Event::Title { title } => {
            info.title = title.clone();

            if info.ready {
                Some(ToplevelChange::Title(title))
            } else {
                None
            }
        }
        Event::State { state } => {
            // state is received as a `Vec<u8>` where every 4 bytes make up a `u32`
            // the u32 then represents a value in the `State` enum.
            assert_eq!(state.len() % 4, 0);

            let state = (0..state.len() / 4)
                .map(|i| {
                    let slice: [u8; 4] = state[i * 4..i * 4 + 4]
                        .try_into()
                        .expect("Received invalid state length");
                    u32::from_le_bytes(slice)
                })
                .collect::<HashSet<_>>();

            let new_active = state.contains(&STATE_ACTIVE);
            let new_fullscreen = state.contains(&STATE_FULLSCREEN);

            let change = if info.ready && new_active != info.active {
                Some(ToplevelChange::Focus(new_active))
            } else if info.ready && new_fullscreen != info.fullscreen {
                Some(ToplevelChange::Fullscreen(new_fullscreen))
            } else {
                None
            };

            info.active = new_active;
            info.fullscreen = new_fullscreen;

            change
        }
        Event::Closed => Some(ToplevelChange::Close),
        Event::OutputEnter { output: _ } => None,
        Event::OutputLeave { output: _ } => None,
        Event::Parent { parent: _ } => None,
        Event::Done => {
            assert_ne!(info.app_id, "");
            if info.ready {
                None
            } else {
                info.ready = true;
                Some(ToplevelChange::New)
            }
        }
        _ => unreachable!(),
    };

    if let Some(change) = change {
        let event = ToplevelEvent {
            change,
            toplevel: info.clone(),
        };

        implem(event, ddata);
    }
}

impl Toplevel {
    pub fn init<F>(handle: &Main<ZwlrForeignToplevelHandleV1>, mut callback: F) -> Self
    where
        F: FnMut(ToplevelEvent, DispatchData) + 'static,
    {
        let inner = Arc::new(RwLock::new(ToplevelInfo::new()));

        handle.quick_assign(move |_handle, event, ddata| {
            let mut inner = inner
                .write()
                .expect("Failed to get write lock on toplevel inner state");
            toplevel_implem(event, &mut inner, &mut callback, ddata);
        });

        Self
    }
}
