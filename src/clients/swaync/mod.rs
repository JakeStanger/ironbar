mod dbus;

use crate::channels::{BroadcastChannel, SyncSenderExt};
use crate::{register_fallible_client, spawn, stop_basic};
use dbus::SwayNcProxy;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tracing::{debug, error};
use zbus::Type;
use zbus::export::ordered_stream::OrderedStreamExt;

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
    Stopped(BroadcastChannel<Event>),
    Started {
        channel: BroadcastChannel<Event>,
        proxy: SwayNcProxy<'static>,
        handle: JoinHandle<()>,
    },
}

impl Client {
    pub(super) fn new() -> Self {
        Self::Stopped(BroadcastChannel::new(8))
    }

    fn proxy(&self) -> super::Result<&SwayNcProxy<'static>, zbus::Error> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started { proxy, .. } => Ok(proxy),
        }
    }

    fn tx(&self) -> &broadcast::Sender<Event> {
        match self {
            Client::Stopped(channel) => channel,
            Client::Started { channel, .. } => channel,
        }
    }

    pub async fn toggle_visibility(&self) -> super::Result<(), zbus::Error> {
        debug!("Toggling visibility");
        self.proxy()?.toggle_visibility().await?;
        Ok(())
    }
}

impl super::Client for Client {
    type State = Self::Event;
    type Event = Event;
    type Error = zbus::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped(channel) = self else {
            return Ok(self);
        };

        let dbus = Box::pin(zbus::Connection::session()).await?;

        let proxy = SwayNcProxy::new(&dbus).await?;

        let mut stream = proxy.receive_subscribe_v2().await?;

        let handle = spawn({
            let tx = channel.clone();
            async move {
                while let Some(ev) = stream.next().await {
                    let ev = ev.message().body().deserialize::<Event>();

                    match ev {
                        Ok(ev) => {
                            debug!("Received event: {ev:?}");
                            tx.send_expect(ev);
                        }
                        Err(err) => {
                            error!("Failed to deserialize event: {err:?}");
                            return;
                        }
                    }
                }
            }
        });

        Ok(Self::Started {
            channel,
            proxy,
            handle,
        })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        stop_basic!(self)
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        debug!("Getting subscribe data (current state)");
        let data = self.proxy()?.get_subscribe_data().await?;
        Ok(data.into())
    }

    fn subscribe(&self) -> broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, notifications);
