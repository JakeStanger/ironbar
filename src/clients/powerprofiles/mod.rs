use futures_lite::StreamExt;
use tokio::{sync::broadcast, task::JoinHandle};
use tracing::error;
use zbus::{Str, fdo::PropertiesProxy, proxy::CacheProperties};

use crate::{
    channels::SyncSenderExt,
    clients::powerprofiles::dbus::{
        ACTIVE_PROFILE, OBJECT_PATH, PERFORMANCE_DEGRADED, PowerProfilesProxy, SERVICE,
    },
    register_fallible_client, spawn,
};

mod dbus;

/// Set of events that can update the Client's [State].
#[derive(Debug, Clone)]
pub(crate) enum Event {
    ActiveProfile(Box<str>),
    PerformanceDegraded(Option<Box<str>>),
}

/// State of the Power Profiles Client
///
/// The active profile is listed under `active_profile`.
/// If a performance degradation is currently taking place then `performance_degraded` is [Some] with the reason. Otherwise it is set to [None].
pub(crate) struct State {
    pub(crate) active_profile: Box<str>,
    pub(crate) performance_degraded: Option<Box<str>>,
}

/// D-Bus client for the power profiles.
///
/// Wraps around D-Bus properties.
#[derive(Debug)]
pub(crate) enum Client {
    Stopped {
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
    },
    Started {
        tx: broadcast::Sender<Event>,
        _rx: broadcast::Receiver<Event>,
        proxy: PowerProfilesProxy<'static>,
        handle: JoinHandle<()>,
    },
}

impl Client {
    pub(crate) fn new() -> Self {
        let (tx, rx) = broadcast::channel(4);
        Self::Stopped { tx, _rx: rx }
    }

    fn proxy(&self) -> super::Result<&PowerProfilesProxy<'static>, zbus::Error> {
        match self {
            Client::Stopped { .. } => Err(super::Error::InvalidState),
            Client::Started { proxy, .. } => Ok(proxy),
        }
    }

    fn tx(&self) -> &broadcast::Sender<Event> {
        match self {
            Client::Stopped { tx, .. } => tx,
            Client::Started { tx, .. } => tx,
        }
    }

    /// Retrieve the active profile.
    async fn active_profile(&self) -> super::Result<Box<str>, zbus::Error> {
        Ok(self.proxy()?.active_profile().await?)
    }

    /// Set a new profile.
    pub(crate) async fn set_profile(&self, value: &str) -> super::Result<(), zbus::Error> {
        Ok(self.proxy()?.set_active_profile(value.into()).await?)
    }

    /// Retrieve all the available profiles from D-Bus.
    pub(crate) async fn available_profiles(&self) -> super::Result<Vec<Box<str>>, zbus::Error> {
        let array = self.proxy()?.profiles().await?;
        let mut profiles = Vec::with_capacity(array.len());

        for mut dict in array.into_iter() {
            const PROFILE_KEY: &str = "Profile";

            if let Some(entry) = dict.remove(PROFILE_KEY) {
                profiles.push(entry);
            }
        }

        Ok(profiles)
    }

    /// Retrieve the performance degraded reason.
    ///
    /// Set to [None] if no performance degradation is taking place.
    async fn performance_degraded_reason(&self) -> super::Result<Option<Box<str>>, zbus::Error> {
        let perf_deg = self.proxy()?.performance_degraded().await?;

        Ok(if perf_deg.is_empty() {
            None
        } else {
            Some(perf_deg)
        })
    }
}

impl super::Client for Client {
    type State = State;

    type Event = Event;

    type Error = zbus::Error;

    fn is_started(&self) -> bool {
        matches!(self, Self::Started { .. })
    }

    async fn start(self) -> Result<Self, Self::Error> {
        let Self::Stopped { tx, _rx } = self else {
            return Ok(self);
        };

        let dbus = Box::pin(zbus::Connection::system()).await?;

        let proxy = PowerProfilesProxy::new(&dbus).await?;

        let properties_proxy = PropertiesProxy::builder(&dbus)
            .destination(SERVICE)
            .path(OBJECT_PATH)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        let mut stream = properties_proxy.receive_properties_changed().await?;

        let handle = spawn({
            let tx = tx.clone();

            async move {
                while let Some(event) = stream.next().await {
                    let args = match event.args() {
                        Ok(args) => args,
                        Err(err) => {
                            error!("could not receive args for event: {err}");
                            continue;
                        }
                    };

                    for (key, value) in args.changed_properties {
                        match key {
                            ACTIVE_PROFILE => {
                                let active_profile = value.downcast::<Str>().unwrap_or_default();
                                if active_profile.is_empty() {
                                    error!("empty profile received");
                                    continue;
                                }

                                tx.send_expect(Event::ActiveProfile(Box::from(
                                    active_profile.as_str(),
                                )))
                            }
                            PERFORMANCE_DEGRADED => {
                                let perf_deg = value.downcast::<Str>().unwrap_or_default();
                                tx.send_expect(Event::PerformanceDegraded(
                                    if perf_deg.is_empty() {
                                        None
                                    } else {
                                        Some(Box::from(perf_deg.as_str()))
                                    },
                                ));
                            }
                            _ => (),
                        }
                    }
                }
            }
        });

        Ok(Self::Started {
            tx,
            _rx,
            proxy,
            handle,
        })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started {
            tx, _rx, handle, ..
        } = self
        else {
            return Ok(self);
        };

        handle.abort();

        Ok(Self::Stopped { tx, _rx })
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        Ok(State {
            active_profile: self.active_profile().await?,
            performance_degraded: self.performance_degraded_reason().await?,
        })
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, power_profiles);
