mod dbus;

use crate::channels::SyncSenderExt;
use crate::{register_fallible_client, spawn};
use dbus::SwayNcProxy;
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{debug, error};
use zbus::export::ordered_stream::OrderedStreamExt;
use zbus::zvariant::Type;

#[derive(Debug, Clone, Copy, Type, Deserialize)]
#[allow(dead_code)]
pub struct Event {
    pub count: u32,
    pub dnd: bool,
    pub cc_open: bool,
    pub inhibited: bool,
}

type GetSubscribeData = (bool, bool, u32, bool);

/// Converts the data returned from
/// `get_subscribe_data` into an event for convenience.
impl From<GetSubscribeData> for Event {
    fn from((dnd, cc_open, count, inhibited): (bool, bool, u32, bool)) -> Self {
        Self {
            count,
            dnd,
            cc_open,
            inhibited,
        }
    }
}

#[derive(Debug)]
pub enum Client {
    Stopped {
        tx: broadcast::Sender<Event>,
    },
    Started {
        tx: broadcast::Sender<Event>,
        proxy: SwayNcProxy<'static>,
    },
}

impl Client {
    pub(super) fn new() -> Self {
        let (tx, rx) = broadcast::channel(8);
        std::mem::forget(rx);

        Self::Stopped { tx }
    }

    fn proxy(&self) -> &SwayNcProxy<'static> {
        match self {
            Client::Stopped { .. } => panic!("client stopped"),
            Client::Started { proxy, .. } => proxy,
        }
    }

    fn tx(&self) -> &broadcast::Sender<Event> {
        match self {
            Client::Stopped { .. } => panic!("client stopped"),
            Client::Started { tx, .. } => tx,
        }
    }

    pub async fn toggle_visibility(&self) {
        debug!("Toggling visibility");
        if let Err(err) = self.proxy().toggle_visibility().await {
            error!("{err:?}");
        }
    }
}

impl super::Client for Client {
    type State = zbus::Result<Self::Event>;
    type Event = Event;
    type Error = zbus::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped { tx } = self else {
            return Ok(self);
        };

        let dbus = Box::pin(zbus::Connection::session()).await?;

        let proxy = SwayNcProxy::new(&dbus).await?;

        let mut stream = proxy.receive_subscribe_v2().await?;

        spawn({
            let tx = tx.clone();
            async move {
                while let Some(ev) = stream.next().await {
                    let ev = ev
                        .message()
                        .body()
                        .deserialize::<Event>()
                        .expect("to deserialize");
                    debug!("Received event: {ev:?}");
                    tx.send_expect(ev);
                }
            }
        });

        Ok(Self::Started { tx, proxy })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started { tx, proxy: _ } = self else {
            return Ok(self);
        };

        Ok(Self::Stopped { tx })
    }

    async fn state(&self) -> Self::State {
        debug!("Getting subscribe data (current state)");
        match self.proxy().get_subscribe_data().await {
            Ok(data) => Ok(data.into()),
            Err(err) => Err(err),
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, notifications);
