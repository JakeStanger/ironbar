use crate::spawn;
use color_eyre::{Report, Result};
use futures_lite::StreamExt;
use std::sync::Arc;
use swayipc_async::{Connection, Event, EventType};
use tokio::sync::Mutex;
use tracing::{info, trace};

type SyncFn<T> = dyn Fn(&T) + Sync + Send;

struct TaskState {
    join_handle: Option<tokio::task::JoinHandle<Result<()>>>,
    // could have been a `HashMap<EventType, Vec<Box<dyn Fn(&Event) + Sync + Send>>>`, but we don't
    // expect enough listeners to justify the constant overhead of a hashmap.
    listeners: Arc<Vec<(EventType, Box<SyncFn<Event>>)>>,
}

pub struct Client {
    connection: Arc<Mutex<Connection>>,
    task_state: Mutex<TaskState>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("client", &"Connection")
            .field("task_state", &format_args!("<...>"))
            .finish()
    }
}

impl Client {
    pub(crate) async fn new() -> Result<Self> {
        // Avoid using `arc_mut!` here because we need tokio Mutex.
        let client = Arc::new(Mutex::new(Connection::new().await?));
        info!("Sway IPC subscription client connected");

        Ok(Self {
            connection: client,
            task_state: Mutex::new(TaskState {
                listeners: Arc::new(Vec::new()),
                join_handle: None,
            }),
        })
    }

    pub fn connection(&self) -> &Arc<Mutex<Connection>> {
        &self.connection
    }

    pub async fn add_listener<T: SwayIpcEvent>(
        &self,
        f: impl Fn(&T) + Sync + Send + 'static,
    ) -> Result<()> {
        self.add_listener_type(
            T::EVENT_TYPE,
            Box::new(move |event| {
                let event = T::from_event(event).expect("event type mismatch");
                f(event)
            }),
        )
        .await
    }

    pub async fn add_listener_type(
        &self,
        event_type: EventType,
        f: Box<SyncFn<Event>>,
    ) -> Result<()> {
        // abort current running task
        let TaskState {
            join_handle,
            listeners,
        } = &mut *self.task_state.lock().await;

        if let Some(handle) = join_handle.take() {
            handle.abort();
            let _ = handle.await;
        }

        // Only the task and self have a reference to listeners, and we just abort the task. This
        // is the only reference to listeners, so we can safely get a mutable reference.
        let listeners_mut = Arc::get_mut(listeners)
            .ok_or_else(|| Report::msg("Failed to get mutable reference to listeners"))?;

        listeners_mut.push((event_type, f));

        // create new client as subscription takes ownership
        let client = Connection::new().await?;

        let event_types = listeners.iter().map(|(t, _)| *t).collect::<Vec<_>>();
        let listeners = listeners.clone();

        let handle = spawn(async move {
            let mut events = client.subscribe(&event_types).await?;

            while let Some(event) = events.next().await {
                trace!("event: {:?}", event);
                let event = event?;
                let ty = sway_event_to_event_type(&event);
                for (t, f) in listeners.iter() {
                    if *t == ty {
                        f(&event);
                    }
                }
            }

            Ok::<(), Report>(())
        });

        *join_handle = Some(handle);

        Ok(())
    }
}

fn sway_event_to_event_type(event: &Event) -> EventType {
    match event {
        Event::Workspace(_) => EventType::Workspace,
        Event::Mode(_) => EventType::Mode,
        Event::Window(_) => EventType::Window,
        Event::BarConfigUpdate(_) => EventType::BarConfigUpdate,
        Event::Binding(_) => EventType::Binding,
        Event::Shutdown(_) => EventType::Shutdown,
        Event::Tick(_) => EventType::Tick,
        Event::BarStateUpdate(_) => EventType::BarStateUpdate,
        Event::Input(_) => EventType::Input,
        _ => todo!(),
    }
}

pub trait SwayIpcEvent {
    const EVENT_TYPE: EventType;
    fn from_event(e: &Event) -> Option<&Self>;
}
macro_rules! sway_ipc_event_impl {
    (@ $($t:tt)*) => { $($t)* };
    ($t:ty, $v:expr, $($m:tt)*) => {
        sway_ipc_event_impl! {@
            impl SwayIpcEvent for $t {
                const EVENT_TYPE: EventType = $v;
                fn from_event(e: &Event) -> Option<&Self> {
                    match e {
                        $($m)* (x) => Some(x),
                        _ => None,
                    }
                }
            }
        }
    };
}

sway_ipc_event_impl!(
    swayipc_async::WorkspaceEvent,
    EventType::Workspace,
    Event::Workspace
);
sway_ipc_event_impl!(swayipc_async::ModeEvent, EventType::Mode, Event::Mode);
sway_ipc_event_impl!(swayipc_async::WindowEvent, EventType::Window, Event::Window);
sway_ipc_event_impl!(
    swayipc_async::BarConfig,
    EventType::BarConfigUpdate,
    Event::BarConfigUpdate
);
sway_ipc_event_impl!(
    swayipc_async::BindingEvent,
    EventType::Binding,
    Event::Binding
);
sway_ipc_event_impl!(
    swayipc_async::ShutdownEvent,
    EventType::Shutdown,
    Event::Shutdown
);
sway_ipc_event_impl!(swayipc_async::TickEvent, EventType::Tick, Event::Tick);
sway_ipc_event_impl!(
    swayipc_async::BarStateUpdateEvent,
    EventType::BarStateUpdate,
    Event::BarStateUpdate
);
sway_ipc_event_impl!(swayipc_async::InputEvent, EventType::Input, Event::Input);
