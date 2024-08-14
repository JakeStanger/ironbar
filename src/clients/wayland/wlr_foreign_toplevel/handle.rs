use super::manager::ToplevelManagerState;
use crate::{lock, Ironbar};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tracing::trace;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::{
    Event, ZwlrForeignToplevelHandleV1,
};

#[derive(Debug, Clone)]
pub struct ToplevelHandle {
    pub handle: ZwlrForeignToplevelHandleV1,
}

impl PartialEq for ToplevelHandle {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl ToplevelHandle {
    pub fn info(&self) -> Option<ToplevelInfo> {
        trace!("Retrieving handle info");

        let data = self.handle.data::<ToplevelHandleData>()?;
        data.info()
    }

    pub fn focus(&self, seat: &WlSeat) {
        trace!("Activating handle");
        self.handle.activate(seat);
    }
}

#[derive(Debug, Default)]
pub struct ToplevelHandleData {
    pub inner: Arc<Mutex<ToplevelHandleDataInner>>,
}

impl ToplevelHandleData {
    fn info(&self) -> Option<ToplevelInfo> {
        lock!(self.inner).current_info.clone()
    }
}

#[derive(Debug, Default)]
pub struct ToplevelHandleDataInner {
    initial_done: bool,
    closed: bool,
    output: Option<WlOutput>,

    current_info: Option<ToplevelInfo>,
    pending_info: ToplevelInfo,
}

#[derive(Debug, Clone)]
pub struct ToplevelInfo {
    pub id: usize,
    pub app_id: String,
    pub title: String,
    pub fullscreen: bool,
    pub focused: bool,
}

impl Default for ToplevelInfo {
    fn default() -> Self {
        Self {
            id: Ironbar::unique_id(),
            app_id: String::new(),
            title: String::new(),
            fullscreen: false,
            focused: false,
        }
    }
}

pub trait ToplevelHandleDataExt {
    fn toplevel_handle_data(&self) -> &ToplevelHandleData;
}

impl ToplevelHandleDataExt for ToplevelHandleData {
    fn toplevel_handle_data(&self) -> &ToplevelHandleData {
        self
    }
}

pub trait ToplevelHandleHandler: Sized {
    fn new_handle(&mut self, conn: &Connection, qh: &QueueHandle<Self>, handle: ToplevelHandle);

    fn update_handle(&mut self, conn: &Connection, qh: &QueueHandle<Self>, handle: ToplevelHandle);

    fn remove_handle(&mut self, conn: &Connection, qh: &QueueHandle<Self>, handle: ToplevelHandle);
}

impl<D, U> Dispatch<ZwlrForeignToplevelHandleV1, U, D> for ToplevelManagerState
where
    D: Dispatch<ZwlrForeignToplevelHandleV1, U> + ToplevelHandleHandler,
    U: ToplevelHandleDataExt,
{
    fn event(
        state: &mut D,
        handle: &ZwlrForeignToplevelHandleV1,
        event: Event,
        data: &U,
        conn: &Connection,
        qh: &QueueHandle<D>,
    ) {
        const STATE_ACTIVE: u32 = 2;
        const STATE_FULLSCREEN: u32 = 3;

        let data = data.toplevel_handle_data();

        trace!("Processing handle event: {event:?}");

        match event {
            Event::Title { title } => {
                lock!(data.inner).pending_info.title = title;
            }
            Event::AppId { app_id } => lock!(data.inner).pending_info.app_id = app_id,
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

                lock!(data.inner).pending_info.focused = state.contains(&STATE_ACTIVE);
                lock!(data.inner).pending_info.fullscreen = state.contains(&STATE_FULLSCREEN);
            }
            Event::OutputEnter { output } => lock!(data.inner).output = Some(output),
            Event::OutputLeave { output: _ } => lock!(data.inner).output = None,
            Event::Closed => {
                lock!(data.inner).closed = true;
                state.remove_handle(
                    conn,
                    qh,
                    ToplevelHandle {
                        handle: handle.clone(),
                    },
                );
            }
            Event::Done if !lock!(data.inner).closed => {
                {
                    let pending_info = lock!(data.inner).pending_info.clone();
                    lock!(data.inner).current_info = Some(pending_info);
                }

                if lock!(data.inner).initial_done {
                    state.update_handle(
                        conn,
                        qh,
                        ToplevelHandle {
                            handle: handle.clone(),
                        },
                    );
                } else {
                    lock!(data.inner).initial_done = true;
                    state.new_handle(
                        conn,
                        qh,
                        ToplevelHandle {
                            handle: handle.clone(),
                        },
                    );
                }
            }
            _ => {}
        }

        trace!("Event processed");
    }
}
