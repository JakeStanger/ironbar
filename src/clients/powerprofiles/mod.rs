use std::sync::Arc;
use zbus::Result;

use crate::{
    clients::{ClientResult, powerprofiles::dbus::PowerProfilesProxy},
    register_fallible_client,
};

mod dbus;

/// D-Bus client for the power profiles.
///
/// Wraps around D-Bus properties.
#[derive(Debug)]
pub(crate) struct Client {
    proxy: Arc<PowerProfilesProxy<'static>>,
}

impl Client {
    pub(crate) async fn new() -> ClientResult<Self> {
        let dbus = zbus::Connection::system().await?;

        let proxy = Arc::new(PowerProfilesProxy::new(&dbus).await?);

        Ok(Arc::new(Self { proxy }))
    }

    /// Get [Some] profile if the `current` profile is different from the one loaded.
    pub(crate) async fn get_new_active_profile(&self, current: &str) -> Result<Option<Box<str>>> {
        let profile = self.proxy.active_profile().await?;

        Ok(if profile == current {
            None
        } else {
            Some(Box::from(profile.as_str()))
        })
    }

    /// Set a new profile.
    pub(crate) async fn set_profile(&self, value: &str) -> Result<()> {
        self.proxy.set_active_profile(value.into()).await
    }

    /// Retrieve all the available profiles.
    pub(crate) async fn available_profiles(&self) -> Result<Vec<Box<str>>> {
        let array = self.proxy.profiles().await?;
        let mut profiles = Vec::with_capacity(array.len());

        for dict in array.iter() {
            const PROFILE_KEY: &str = "Profile";

            if let Some(entry) = dict.get(&PROFILE_KEY.into()) {
                profiles.push(Box::from(entry.as_str()));
            }
        }

        Ok(profiles)
    }

    /// Get [Some] reason for the performance being degraded if different from `current`.
    pub(crate) async fn get_new_performance_degraded_reason(
        &self,
        current: &str,
    ) -> Result<Option<Box<str>>> {
        let reason = self.proxy.performance_degraded().await?;

        Ok(if reason == current {
            None
        } else {
            Some(Box::from(reason.as_str()))
        })
    }
}

register_fallible_client!(Client, power_profiles);
