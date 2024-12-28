mod dbus;

use crate::channels::SyncSenderExt;
use crate::{register_fallible_client, spawn};
use color_eyre::{Report, Result};
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
pub struct Client {
    proxy: SwayNcProxy<'static>,
    tx: broadcast::Sender<Event>,
    _rx: broadcast::Receiver<Event>,
}

impl Client {
    pub async fn new() -> Result<Self> {
        let dbus = Box::pin(zbus::Connection::session()).await?;

        let proxy = SwayNcProxy::new(&dbus).await?;
        let (tx, rx) = broadcast::channel(8);

        let mut stream = proxy.receive_subscribe_v2().await?;

        {
            let tx = tx.clone();

            spawn(async move {
                while let Some(ev) = stream.next().await {
                    let ev = ev.body::<Event>().expect("to deserialize");
                    debug!("Received event: {ev:?}");
                    tx.send_expect(ev);
                }
            });
        }

        Ok(Self { proxy, tx, _rx: rx })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    pub async fn state(&self) -> Result<Event> {
        debug!("Getting subscribe data (current state)");
        match self.proxy.get_subscribe_data().await {
            Ok(data) => Ok(data.into()),
            Err(err) => Err(Report::new(err)),
        }
    }

    pub async fn toggle_visibility(&self) {
        debug!("Toggling visibility");
        if let Err(err) = self.proxy.toggle_visibility().await {
            error!("{err:?}");
        }
    }
}

register_fallible_client!(Client, notifications);
