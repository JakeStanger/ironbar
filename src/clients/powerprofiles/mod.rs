use tokio::sync::broadcast;

use crate::{clients::powerprofiles::dbus::PowerProfilesProxy, register_fallible_client};

mod dbus;

pub(crate) type Event = ();

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

    /// Get [Some] profile if the `current` profile is different from the one loaded.
    pub(crate) async fn get_new_active_profile(
        &self,
        current: &str,
    ) -> super::Result<Option<Box<str>>, zbus::Error> {
        let profile = self.proxy()?.active_profile().await?;

        Ok(if *profile == *current {
            None
        } else {
            Some(profile)
        })
    }

    /// Set a new profile.
    pub(crate) async fn set_profile(&self, value: &str) -> super::Result<(), zbus::Error> {
        self.proxy()?
            .set_active_profile(value.into())
            .await
            .map_err(Into::into)
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

    /// Get [Some] reason for the performance being degraded if different from `current`.
    pub(crate) async fn get_new_performance_degraded_reason(
        &self,
        current: &str,
    ) -> super::Result<Option<Box<str>>, zbus::Error> {
        let reason = self.proxy()?.performance_degraded().await?;

        Ok(if *reason == *current {
            None
        } else {
            Some(reason)
        })
    }
}

impl super::Client for Client {
    type State = ();

    type Event = ();

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

        Ok(Self::Started { tx, _rx, proxy })
    }

    async fn stop(self) -> Result<Self, Self::Error> {
        let Self::Started { tx, _rx, .. } = self else {
            return Ok(self);
        };

        Ok(Self::Stopped { tx, _rx })
    }

    async fn state(&self) -> super::Result<Self::State, Self::Error> {
        Ok(())
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Self::Event> {
        self.tx().subscribe()
    }
}

register_fallible_client!(Client, power_profiles);
