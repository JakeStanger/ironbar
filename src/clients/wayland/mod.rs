mod ext_workspace;
mod macros;
mod wl_output;
mod wl_seat;

use crate::error::{ERR_CHANNEL_RECV, ExitCode};
use crate::{
    arc_mut, delegate_workspace_group_handle, delegate_workspace_handle,
    delegate_workspace_manager, lock, register_client, spawn, spawn_blocking,
};
#[cfg(feature = "workspaces")]
use std::collections::{HashMap, HashSet};
use std::process::exit;
use std::sync::{Arc, Mutex};

use crate::channels::{AsyncSenderExt, SyncSenderExt};
#[cfg(feature = "workspaces")]
use crate::clients::compositor::{Visibility, Workspace, WorkspaceUpdate};
#[cfg(feature = "workspaces")]
use crate::clients::wayland::ext_workspace::group_handle::WorkspaceGroupHandleData;
#[cfg(feature = "workspaces")]
use crate::clients::wayland::ext_workspace::group_handle::WorkspaceGroupHandleHandler;
#[cfg(feature = "workspaces")]
use crate::clients::wayland::ext_workspace::handle::WorkspaceHandleData;
#[cfg(feature = "workspaces")]
use crate::clients::wayland::ext_workspace::handle::WorkspaceHandleHandler;
#[cfg(feature = "workspaces")]
use crate::clients::wayland::ext_workspace::manager::WorkspaceManagerHandler;
use crate::clients::wayland::ext_workspace::manager::WorkspaceManagerState;
use calloop_channel::Event::Msg;
use cfg_if::cfg_if;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop;
use smithay_client_toolkit::reexports::calloop::EventLoop;
use smithay_client_toolkit::reexports::calloop::channel as calloop_channel;
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::SeatState;
use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_seat, registry_handlers,
};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, trace, warn};
#[cfg(feature = "workspaces")]
use wayland_client::backend::ObjectId;
use wayland_client::globals::{BindError, registry_queue_init};
#[cfg(feature = "workspaces")]
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::{Connection, Proxy, QueueHandle};
#[cfg(feature = "workspaces")]
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1,
    ext_workspace_handle_v1::ExtWorkspaceHandleV1,
};
pub use wl_output::{OutputEvent, OutputEventType};

cfg_if! {
    if #[cfg(any(feature = "focused", feature = "launcher"))] {
        mod wlr_foreign_toplevel;
        use crate::{delegate_foreign_toplevel_handle, delegate_foreign_toplevel_manager};
        use wlr_foreign_toplevel::manager::ToplevelManagerState;
        pub use wlr_foreign_toplevel::{ToplevelEvent, ToplevelHandle, ToplevelInfo};

    }
}

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        mod wlr_data_control;

        use crate::{delegate_data_control_device, delegate_data_control_device_manager, delegate_data_control_offer, delegate_data_control_source};
        use self::wlr_data_control::device::DataControlDevice;
        use self::wlr_data_control::manager::DataControlDeviceManagerState;
        use self::wlr_data_control::source::CopyPasteSource;
        use wayland_client::protocol::wl_seat::WlSeat;

        pub use wlr_data_control::{ClipboardItem, ClipboardValue};

        #[derive(Debug)]
        pub struct DataControlDeviceEntry {
            seat: WlSeat,
            device: DataControlDevice,
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Output(OutputEvent),
    #[cfg(any(feature = "focused", feature = "launcher"))]
    Toplevel(ToplevelEvent),
    #[cfg(feature = "clipboard")]
    Clipboard(ClipboardItem),
    #[cfg(feature = "workspaces")]
    Workspace(WorkspaceUpdate),
}

#[derive(Debug)]
pub enum Request {
    Roundtrip,

    OutputInfoAll,

    #[cfg(any(feature = "focused", feature = "launcher"))]
    ToplevelInfoAll,
    #[cfg(feature = "launcher")]
    ToplevelFocus(usize),
    #[cfg(feature = "launcher")]
    ToplevelMinimize(usize),

    #[cfg(feature = "clipboard")]
    CopyToClipboard(ClipboardItem),
    #[cfg(feature = "clipboard")]
    ClipboardItem,

    #[cfg(feature = "workspaces")]
    WorkspaceInfoAll,
    #[cfg(feature = "workspaces")]
    WorkspaceActivate(i64),
    #[cfg(feature = "workspaces")]
    WorkspaceDeactivate(i64),
    #[cfg(feature = "workspaces")]
    WorkspaceToggle(i64),
    #[cfg(feature = "workspaces")]
    WorkspaceActivateExclusive(i64),
}

#[derive(Debug)]
pub enum Response {
    /// An empty success response
    Ok,

    OutputInfoAll(Vec<smithay_client_toolkit::output::OutputInfo>),

    #[cfg(any(feature = "focused", feature = "launcher"))]
    ToplevelInfoAll(Vec<ToplevelInfo>),

    #[cfg(feature = "clipboard")]
    ClipboardItem(Option<ClipboardItem>),

    #[cfg(feature = "workspaces")]
    WorkspaceInfoAll(Vec<Workspace>),
}

#[derive(Debug)]
#[allow(dead_code)]
struct BroadcastChannel<T>(broadcast::Sender<T>, Arc<Mutex<broadcast::Receiver<T>>>);

impl<T> From<(broadcast::Sender<T>, broadcast::Receiver<T>)> for BroadcastChannel<T> {
    fn from(value: (broadcast::Sender<T>, broadcast::Receiver<T>)) -> Self {
        Self(value.0, arc_mut!(value.1))
    }
}

#[cfg(feature = "workspaces")]
#[derive(Debug, Clone)]
struct WorkspaceSnapshot {
    workspace: Workspace,
    urgent: bool,
}

#[cfg(feature = "workspaces")]
#[derive(Debug, Default)]
struct WorkspaceState {
    groups: HashMap<ObjectId, ExtWorkspaceGroupHandleV1>,
    workspaces: HashMap<ObjectId, ExtWorkspaceHandleV1>,
    group_for_workspace: HashMap<ObjectId, ObjectId>,
    last_snapshot: Vec<WorkspaceSnapshot>,
    initialized: bool,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Failed to bind to '{name}' global (likely missing protocol support). Following modules will not work: {modules:?}\n{error}"
    )]
    UnsupportedProtocol {
        name: &'static str,
        modules: &'static [&'static str],
        error: BindError,
    },
    #[error("failed to dispatch pending wayland events: {0}")]
    Dispatch(#[from] calloop::Error),
}

#[derive(Debug)]
pub struct Client {
    tx: calloop_channel::Sender<Request>,
    rx: Arc<Mutex<std::sync::mpsc::Receiver<Response>>>,

    output_channel: BroadcastChannel<OutputEvent>,
    #[cfg(any(feature = "focused", feature = "launcher"))]
    toplevel_channel: BroadcastChannel<ToplevelEvent>,
    #[cfg(feature = "clipboard")]
    clipboard_channel: BroadcastChannel<ClipboardItem>,
    #[cfg(feature = "workspaces")]
    workspace_channel: BroadcastChannel<WorkspaceUpdate>,
}

impl Client {
    pub(crate) fn new() -> Self {
        let (event_tx, mut event_rx) = mpsc::channel(32);

        let (request_tx, request_rx) = calloop_channel::channel();
        let (response_tx, response_rx) = std::sync::mpsc::channel();

        let output_channel = broadcast::channel(32);
        #[cfg(any(feature = "focused", feature = "launcher"))]
        let toplevel_channel = broadcast::channel(32);

        #[cfg(feature = "clipboard")]
        let clipboard_channel = broadcast::channel(32);
        #[cfg(feature = "workspaces")]
        let workspace_channel = broadcast::channel(32);

        spawn_blocking(move || {
            Environment::spawn(event_tx, request_rx, response_tx);
        });

        // listen to events
        {
            let output_tx = output_channel.0.clone();
            #[cfg(any(feature = "focused", feature = "launcher"))]
            let toplevel_tx = toplevel_channel.0.clone();

            #[cfg(feature = "clipboard")]
            let clipboard_tx = clipboard_channel.0.clone();
            #[cfg(feature = "workspaces")]
            let workspace_tx = workspace_channel.0.clone();

            spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    match event {
                        Event::Output(event) => output_tx.send_expect(event),
                        #[cfg(any(feature = "focused", feature = "launcher"))]
                        Event::Toplevel(event) => toplevel_tx.send_expect(event),
                        #[cfg(feature = "clipboard")]
                        Event::Clipboard(item) => clipboard_tx.send_expect(item),
                        #[cfg(feature = "workspaces")]
                        Event::Workspace(update) => workspace_tx.send_expect(update),
                    }
                }
            });
        }

        Self {
            tx: request_tx,
            rx: arc_mut!(response_rx),

            output_channel: output_channel.into(),
            #[cfg(any(feature = "focused", feature = "launcher"))]
            toplevel_channel: toplevel_channel.into(),
            #[cfg(feature = "clipboard")]
            clipboard_channel: clipboard_channel.into(),
            #[cfg(feature = "workspaces")]
            workspace_channel: workspace_channel.into(),
        }
    }

    /// Sends a request to the environment event loop,
    /// and returns the response.
    fn send_request(&self, request: Request) -> Response {
        // Serialize request/response pairs so concurrent callers cannot
        // consume each other's responses from the shared channel.
        let rx = &mut *lock!(self.rx);
        self.tx.send_expect(request);
        rx.recv().expect(ERR_CHANNEL_RECV)
    }

    /// Sends a round-trip request to the client,
    /// forcing it to send/receive any events in the queue.
    pub(crate) fn roundtrip(&self) -> Response {
        self.send_request(Request::Roundtrip)
    }
}

#[derive(Debug)]
pub struct Environment {
    registry_state: RegistryState,
    output_state: OutputState,
    seat_state: SeatState,

    queue_handle: QueueHandle<Self>,

    event_tx: mpsc::Sender<Event>,
    response_tx: std::sync::mpsc::Sender<Response>,

    // local state
    #[cfg(any(feature = "focused", feature = "launcher"))]
    handles: Vec<ToplevelHandle>,

    // -- clipboard --
    #[cfg(feature = "clipboard")]
    data_control_device_manager_state: Option<DataControlDeviceManagerState>,

    #[cfg(feature = "clipboard")]
    data_control_devices: Vec<DataControlDeviceEntry>,
    #[cfg(feature = "clipboard")]
    copy_paste_sources: Vec<CopyPasteSource>,

    // -- workspaces
    workspace_manager_state: Option<WorkspaceManagerState>,
    #[cfg(feature = "workspaces")]
    workspace_state: WorkspaceState,

    // local state
    #[cfg(feature = "clipboard")]
    clipboard: Arc<Mutex<Option<ClipboardItem>>>,
}

delegate_registry!(Environment);

delegate_output!(Environment);
delegate_seat!(Environment);

cfg_if! {
    if #[cfg(any(feature = "focused", feature = "launcher"))] {
        delegate_foreign_toplevel_manager!(Environment);
        delegate_foreign_toplevel_handle!(Environment);
    }
}

cfg_if! {
    if #[cfg(feature = "clipboard")] {
        delegate_data_control_device_manager!(Environment);
        delegate_data_control_device!(Environment);
        delegate_data_control_offer!(Environment);
        delegate_data_control_source!(Environment);
    }
}

delegate_workspace_manager!(Environment);
delegate_workspace_group_handle!(Environment);
delegate_workspace_handle!(Environment);

impl Environment {
    pub fn spawn(
        event_tx: mpsc::Sender<Event>,
        request_rx: calloop_channel::Channel<Request>,
        response_tx: std::sync::mpsc::Sender<Response>,
    ) {
        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
        let (globals, queue) =
            registry_queue_init(&conn).expect("Failed to retrieve Wayland globals");

        let qh = queue.handle();
        let mut event_loop = EventLoop::<Self>::try_new().expect("Failed to create new event loop");

        WaylandSource::new(conn, queue)
            .insert(event_loop.handle())
            .expect("Failed to insert Wayland event queue into event loop");

        let loop_handle = event_loop.handle();

        // Initialize the registry handling
        // so other parts of Smithay's client toolkit may bind globals.
        let registry_state = RegistryState::new(&globals);

        let output_state = OutputState::new(&globals, &qh);
        let seat_state = SeatState::new(&globals, &qh);
        #[cfg(any(feature = "focused", feature = "launcher"))]
        if let Err(error) = ToplevelManagerState::bind(&globals, &qh) {
            error!(
                "{}",
                Error::UnsupportedProtocol {
                    error,
                    name: "wlr_foreign_toplevel_manager",
                    modules: &["launcher", "focused"]
                }
            );
        }

        #[cfg(feature = "clipboard")]
        let data_control_device_manager_state =
            match DataControlDeviceManagerState::bind(&globals, &qh) {
                Ok(state) => Some(state),
                Err(error) => {
                    error!(
                        "{}",
                        Error::UnsupportedProtocol {
                            error,
                            name: "wlr_data_control_device",
                            modules: &["clipboard"]
                        }
                    );
                    None
                }
            };

        let workspace_manager_state = match WorkspaceManagerState::bind(&globals, &qh) {
            Ok(state) => {
                debug!("ext-workspace manager is available");
                Some(state)
            }
            Err(error) => {
                error!(
                    "{}",
                    Error::UnsupportedProtocol {
                        error,
                        name: "ext_workspace",
                        modules: &["workspaces"]
                    }
                );
                None
            }
        };

        let mut env = Self {
            registry_state,
            output_state,
            seat_state,
            #[cfg(feature = "clipboard")]
            data_control_device_manager_state,
            workspace_manager_state,
            #[cfg(feature = "workspaces")]
            workspace_state: WorkspaceState::default(),
            queue_handle: qh,
            event_tx,
            response_tx,
            #[cfg(any(feature = "focused", feature = "launcher"))]
            handles: vec![],

            #[cfg(feature = "clipboard")]
            data_control_devices: vec![],
            #[cfg(feature = "clipboard")]
            copy_paste_sources: vec![],
            #[cfg(feature = "clipboard")]
            clipboard: arc_mut!(None),
        };

        loop_handle
            .insert_source(request_rx, Self::on_request)
            .expect("to be able to insert source");

        loop {
            trace!("Dispatching event loop");
            if let Err(err) = event_loop.dispatch(None, &mut env) {
                error!("{}", Error::Dispatch(err));
                exit(ExitCode::WaylandDispatchError as i32)
            }
        }
    }

    /// Processes a request from the client
    /// and sends the response.
    fn on_request(event: calloop_channel::Event<Request>, _metadata: &mut (), env: &mut Self) {
        trace!("Request: {event:?}");

        match event {
            Msg(Request::Roundtrip) => {
                debug!("received roundtrip request");
                env.response_tx.send_expect(Response::Ok);
            }
            Msg(Request::OutputInfoAll) => {
                let infos = env.output_info_all();
                env.response_tx.send_expect(Response::OutputInfoAll(infos));
            }
            #[cfg(any(feature = "focused", feature = "launcher"))]
            Msg(Request::ToplevelInfoAll) => {
                let infos = env
                    .handles
                    .iter()
                    .filter_map(ToplevelHandle::info)
                    .collect();

                env.response_tx
                    .send_expect(Response::ToplevelInfoAll(infos));
            }
            #[cfg(feature = "launcher")]
            Msg(Request::ToplevelFocus(id)) => {
                let handle = env
                    .handles
                    .iter()
                    .find(|handle| handle.info().is_some_and(|info| info.id == id));

                if let Some(handle) = handle {
                    let seat = env.default_seat();
                    handle.focus(&seat);
                }

                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "launcher")]
            Msg(Request::ToplevelMinimize(id)) => {
                let handle = env
                    .handles
                    .iter()
                    .find(|handle| handle.info().is_some_and(|info| info.id == id));

                if let Some(handle) = handle {
                    handle.minimize();
                }

                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "clipboard")]
            Msg(Request::CopyToClipboard(item)) => {
                env.copy_to_clipboard(item);
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "clipboard")]
            Msg(Request::ClipboardItem) => {
                let item = lock!(env.clipboard).clone();
                env.response_tx.send_expect(Response::ClipboardItem(item));
            }
            #[cfg(feature = "workspaces")]
            Msg(Request::WorkspaceInfoAll) => {
                let workspaces = env.workspace_info_all();
                env.response_tx
                    .send_expect(Response::WorkspaceInfoAll(workspaces));
            }
            #[cfg(feature = "workspaces")]
            Msg(Request::WorkspaceActivate(id)) => {
                env.workspace_activate(id);
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "workspaces")]
            Msg(Request::WorkspaceDeactivate(id)) => {
                env.workspace_deactivate(id);
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "workspaces")]
            Msg(Request::WorkspaceToggle(id)) => {
                env.workspace_toggle(id);
                env.response_tx.send_expect(Response::Ok);
            }
            #[cfg(feature = "workspaces")]
            Msg(Request::WorkspaceActivateExclusive(id)) => {
                env.workspace_activate_exclusive(id);
                env.response_tx.send_expect(Response::Ok);
            }
            calloop_channel::Event::Closed => error!("request channel unexpectedly closed"),
        }
    }
}

#[cfg(feature = "workspaces")]
impl Environment {
    fn workspace_info_all(&self) -> Vec<Workspace> {
        if self.workspace_manager_state.is_none() {
            debug!("workspace_info_all requested but ext-workspace manager is unavailable");
        }
        self.workspace_state
            .last_snapshot
            .iter()
            .map(|snapshot| snapshot.workspace.clone())
            .collect()
    }

    fn workspace_activate(&mut self, id: i64) {
        let Some(manager) = self.workspace_manager_state.as_ref() else {
            debug!("workspace activate requested but ext-workspace is unavailable");
            return;
        };

        let Some(handle) = self.workspace_handle_by_internal_id(id) else {
            debug!("workspace handle not found for id {id}");
            return;
        };

        if self.workspace_is_active(&handle) {
            return;
        }

        handle.activate();
        manager.commit();
    }

    fn workspace_deactivate(&mut self, id: i64) {
        let Some(manager) = self.workspace_manager_state.as_ref() else {
            debug!("workspace deactivate requested but ext-workspace is unavailable");
            return;
        };

        let Some(handle) = self.workspace_handle_by_internal_id(id) else {
            debug!("workspace handle not found for id {id}");
            return;
        };

        if !self.workspace_is_active(&handle) {
            return;
        }

        handle.deactivate();
        manager.commit();
    }

    fn workspace_toggle(&mut self, id: i64) {
        let Some(handle) = self.workspace_handle_by_internal_id(id) else {
            debug!("workspace handle not found for id {id}");
            return;
        };

        if self.workspace_is_active(&handle) {
            self.workspace_deactivate(id);
        } else {
            self.workspace_activate(id);
        }
    }

    fn workspace_activate_exclusive(&mut self, id: i64) {
        let Some(manager) = self.workspace_manager_state.as_ref() else {
            debug!("workspace activate exclusive requested but ext-workspace is unavailable");
            return;
        };

        let Some(clicked) = self.workspace_handle_by_internal_id(id) else {
            debug!("workspace handle not found for id {id}");
            return;
        };

        let clicked_id = clicked.id();
        let clicked_group = self
            .workspace_state
            .group_for_workspace
            .get(&clicked_id)
            .cloned();
        let clicked_outputs = self.workspace_output_ids_for_workspace(&clicked);

        let mut to_deactivate = Vec::new();
        for handle in self.workspace_state.workspaces.values() {
            if handle.id() == clicked_id {
                continue;
            }

            let same_output = if !clicked_outputs.is_empty() {
                !self
                    .workspace_output_ids_for_workspace(handle)
                    .is_disjoint(&clicked_outputs)
            } else if let Some(clicked_group) = clicked_group.as_ref() {
                self.workspace_state
                    .group_for_workspace
                    .get(&handle.id())
                    .is_some_and(|group| group == clicked_group)
            } else {
                false
            };

            if same_output {
                to_deactivate.push(handle.clone());
            }
        }

        for handle in to_deactivate {
            handle.deactivate();
        }

        clicked.activate();
        manager.commit();
    }

    fn workspace_handle_by_internal_id(&self, id: i64) -> Option<ExtWorkspaceHandleV1> {
        self.workspace_state
            .workspaces
            .values()
            .find(|handle| {
                handle
                    .data::<WorkspaceHandleData>()
                    .is_some_and(|data| data.info().internal_id == id)
            })
            .cloned()
    }

    fn workspace_is_active(&self, handle: &ExtWorkspaceHandleV1) -> bool {
        handle
            .data::<WorkspaceHandleData>()
            .is_some_and(|data| data.info().state.bits() & 1 != 0)
    }

    fn workspace_group_created(&mut self, group: ExtWorkspaceGroupHandleV1) {
        debug!("workspace group created: {:?}", group.id());
        self.workspace_state.groups.insert(group.id(), group);
    }

    fn workspace_group_removed(&mut self, group: ExtWorkspaceGroupHandleV1) {
        let group_id = group.id();
        debug!("workspace group removed: {group_id:?}");
        self.workspace_state.groups.remove(&group_id);
        self.workspace_state
            .group_for_workspace
            .retain(|_, mapped_group| mapped_group != &group_id);
    }

    fn workspace_created(&mut self, workspace: ExtWorkspaceHandleV1) {
        trace!("workspace handle created: {:?}", workspace.id());
        self.workspace_state
            .workspaces
            .insert(workspace.id(), workspace);
    }

    fn workspace_removed(&mut self, workspace: ExtWorkspaceHandleV1) {
        let workspace_id = workspace.id();
        debug!("workspace handle removed: {workspace_id:?}");
        self.workspace_state.workspaces.remove(&workspace_id);
        self.workspace_state
            .group_for_workspace
            .remove(&workspace_id);
    }

    fn workspace_group_workspace_enter(
        &mut self,
        group: ExtWorkspaceGroupHandleV1,
        workspace: ExtWorkspaceHandleV1,
    ) {
        trace!(
            "workspace entered group: workspace={:?} group={:?}",
            workspace.id(),
            group.id()
        );
        self.workspace_state
            .group_for_workspace
            .insert(workspace.id(), group.id());
    }

    fn workspace_group_workspace_leave(
        &mut self,
        group: ExtWorkspaceGroupHandleV1,
        workspace: ExtWorkspaceHandleV1,
    ) {
        let workspace_id = workspace.id();
        trace!(
            "workspace left group: workspace={workspace_id:?} group={:?}",
            group.id()
        );
        if self
            .workspace_state
            .group_for_workspace
            .get(&workspace_id)
            .is_some_and(|mapped_group| mapped_group == &group.id())
        {
            self.workspace_state
                .group_for_workspace
                .remove(&workspace_id);
        }
    }

    fn workspace_done(&mut self) {
        if self.workspace_manager_state.is_none() {
            return;
        }

        let snapshot = self.build_workspace_snapshot();
        debug!(
            "workspace done: groups={} workspaces={} mapped={} snapshot={}",
            self.workspace_state.groups.len(),
            self.workspace_state.workspaces.len(),
            self.workspace_state.group_for_workspace.len(),
            snapshot.len()
        );
        self.apply_workspace_snapshot(snapshot);
    }

    fn workspace_finished(&mut self) {
        warn!("ext-workspace manager finished");
    }

    fn build_workspace_snapshot(&self) -> Vec<WorkspaceSnapshot> {
        let mut snapshots = Vec::new();

        for (workspace_id, workspace_handle) in &self.workspace_state.workspaces {
            let Some(data) = workspace_handle.data::<WorkspaceHandleData>() else {
                continue;
            };
            let info = data.info();

            let Some(group_id) = self.workspace_state.group_for_workspace.get(workspace_id) else {
                continue;
            };

            let Some(group_handle) = self.workspace_state.groups.get(group_id) else {
                continue;
            };

            let monitor = self.output_name_from_group(group_handle);
            let index = if let Some(index) = info.coordinates.first() {
                *index as i64
            } else if let Ok(parsed) = info.name.parse::<i64>() {
                parsed
            } else {
                info.internal_id
            };

            let bits = info.state.bits();
            let is_hidden = bits & 4 != 0;
            let is_active = bits & 1 != 0;
            let urgent = bits & 2 != 0;

            let visibility = if is_hidden {
                Visibility::Hidden
            } else if is_active {
                Visibility::focused()
            } else {
                Visibility::visible()
            };

            snapshots.push(WorkspaceSnapshot {
                workspace: Workspace {
                    id: info.internal_id,
                    index,
                    name: info.name,
                    monitor,
                    visibility,
                },
                urgent,
            });
        }

        snapshots.sort_by(|a, b| {
            (
                a.workspace.monitor.as_str(),
                a.workspace.index,
                a.workspace.name.as_str(),
            )
                .cmp(&(
                    b.workspace.monitor.as_str(),
                    b.workspace.index,
                    b.workspace.name.as_str(),
                ))
        });

        snapshots
    }

    fn apply_workspace_snapshot(&mut self, snapshot: Vec<WorkspaceSnapshot>) {
        if !self.workspace_state.initialized {
            let workspaces = snapshot
                .iter()
                .map(|entry| entry.workspace.clone())
                .collect::<Vec<_>>();

            self.workspace_state.last_snapshot = snapshot.clone();
            self.workspace_state.initialized = true;
            debug!(
                "workspace init snapshot contains {} workspaces",
                workspaces.len()
            );
            self.event_tx
                .send_spawn(Event::Workspace(WorkspaceUpdate::Init(workspaces)));

            for entry in snapshot {
                if entry.urgent {
                    self.event_tx
                        .send_spawn(Event::Workspace(WorkspaceUpdate::Urgent {
                            id: entry.workspace.id,
                            urgent: true,
                        }));
                }
            }

            return;
        }

        let old_snapshot = self.workspace_state.last_snapshot.clone();
        let mut updates = Vec::new();

        let mut old_by_id = HashMap::new();
        for entry in &old_snapshot {
            old_by_id.insert(entry.workspace.id, entry);
        }

        let mut new_by_id = HashMap::new();
        for entry in &snapshot {
            new_by_id.insert(entry.workspace.id, entry);
        }

        for entry in &snapshot {
            match old_by_id.get(&entry.workspace.id) {
                None => updates.push(WorkspaceUpdate::Add(entry.workspace.clone())),
                Some(old_entry) => {
                    if entry.workspace.name != old_entry.workspace.name {
                        updates.push(WorkspaceUpdate::Rename {
                            id: entry.workspace.id,
                            name: entry.workspace.name.clone(),
                        });
                    }

                    if entry.workspace.monitor != old_entry.workspace.monitor
                        || entry.workspace.index != old_entry.workspace.index
                    {
                        updates.push(WorkspaceUpdate::Move(entry.workspace.clone()));
                    }

                    if entry.urgent != old_entry.urgent {
                        updates.push(WorkspaceUpdate::Urgent {
                            id: entry.workspace.id,
                            urgent: entry.urgent,
                        });
                    }
                }
            }
        }

        for entry in &old_snapshot {
            if !new_by_id.contains_key(&entry.workspace.id) {
                updates.push(WorkspaceUpdate::Remove(entry.workspace.id));
            }
        }

        let mut old_focused = HashMap::new();
        for entry in &old_snapshot {
            if entry.workspace.visibility.is_focused() {
                old_focused.insert(entry.workspace.id, entry.workspace.clone());
            }
        }

        let mut new_focused = HashMap::new();
        for entry in &snapshot {
            if entry.workspace.visibility.is_focused() {
                new_focused.insert(entry.workspace.id, entry.workspace.clone());
            }
        }

        // manually focus without old here and explicitly use unfocus later on. to comply with existing hyprland behavior but allow to go from like 5 Focused to 1 Focused workspace
        for (id, new_workspace) in &new_focused {
            if !old_focused.contains_key(id) {
                updates.push(WorkspaceUpdate::Focus {
                    old: None,
                    new: new_workspace.clone(),
                });
            }
        }

        for (id, old_workspace) in &old_focused {
            if !new_focused.contains_key(id) {
                updates.push(WorkspaceUpdate::Unfocus(old_workspace.clone()));
            }
        }

        self.workspace_state.last_snapshot = snapshot;

        debug!("emitting {} workspace updates", updates.len());
        for update in updates {
            self.event_tx.send_spawn(Event::Workspace(update));
        }
    }

    fn output_name_from_group(&self, group: &ExtWorkspaceGroupHandleV1) -> String {
        let Some(data) = group.data::<WorkspaceGroupHandleData>() else {
            return String::new();
        };

        for output in data.outputs() {
            if let Some(info) = self.output_state.info(&output) {
                if let Some(name) = info.name {
                    return name;
                }
            }
        }

        String::new()
    }

    fn workspace_output_ids_for_workspace(
        &self,
        workspace: &ExtWorkspaceHandleV1,
    ) -> HashSet<ObjectId> {
        let Some(group_id) = self
            .workspace_state
            .group_for_workspace
            .get(&workspace.id())
        else {
            return HashSet::new();
        };
        let Some(group) = self.workspace_state.groups.get(group_id) else {
            return HashSet::new();
        };
        let Some(data) = group.data::<WorkspaceGroupHandleData>() else {
            return HashSet::new();
        };

        data.outputs()
            .into_iter()
            .map(|output| output.id())
            .collect()
    }
}

impl ProvidesRegistryState for Environment {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

#[cfg(feature = "workspaces")]
impl WorkspaceManagerHandler for Environment {
    fn workspace_group_created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        group: ExtWorkspaceGroupHandleV1,
    ) {
        self.workspace_group_created(group);
    }

    fn workspace_created(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        workspace: ExtWorkspaceHandleV1,
    ) {
        self.workspace_created(workspace);
    }

    fn workspace_done(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        self.workspace_done();
    }

    fn workspace_finished(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>) {
        self.workspace_finished();
    }
}

#[cfg(feature = "workspaces")]
impl WorkspaceGroupHandleHandler for Environment {
    fn workspace_group_output_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _group: ext_workspace::group_handle::WorkspaceGroupHandle,
        _output: WlOutput,
    ) {
    }

    fn workspace_group_output_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _group: ext_workspace::group_handle::WorkspaceGroupHandle,
        _output: WlOutput,
    ) {
    }

    fn workspace_group_workspace_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        group: ext_workspace::group_handle::WorkspaceGroupHandle,
        workspace: ExtWorkspaceHandleV1,
    ) {
        self.workspace_group_workspace_enter(group.handle, workspace);
    }

    fn workspace_group_workspace_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        group: ext_workspace::group_handle::WorkspaceGroupHandle,
        workspace: ExtWorkspaceHandleV1,
    ) {
        self.workspace_group_workspace_leave(group.handle, workspace);
    }

    fn workspace_group_removed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        group: ext_workspace::group_handle::WorkspaceGroupHandle,
    ) {
        self.workspace_group_removed(group.handle);
    }
}

#[cfg(feature = "workspaces")]
impl WorkspaceHandleHandler for Environment {
    fn workspace_removed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        handle: ext_workspace::handle::WorkspaceHandle,
    ) {
        self.workspace_removed(handle.handle);
    }
}

register_client!(Client, wayland);
