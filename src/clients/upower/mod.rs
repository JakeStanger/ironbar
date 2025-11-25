mod dbus;

use crate::clients::ClientResult;
use crate::{await_sync, register_fallible_client};
use dbus::UPowerProxy;
use miette::IntoDiagnostic;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;

pub use dbus::BatteryState;

#[derive(Debug)]
pub struct Client {
    proxy: PropertiesProxy<'static>,
    pub interface_name: InterfaceName<'static>,
}

impl Client {
    pub async fn new() -> ClientResult<Self> {
        let dbus = Box::pin(zbus::Connection::system())
            .await
            .into_diagnostic()?;

        let device_proxy = UPowerProxy::new(&dbus).await.into_diagnostic()?;

        let display_device = device_proxy.get_display_device().await.into_diagnostic()?;

        let path = display_device.inner().path();

        let proxy = PropertiesProxy::builder(&dbus)
            .destination("org.freedesktop.UPower")
            .expect("failed to set proxy destination address")
            .path(path)
            .expect("failed to set proxy path")
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .into_diagnostic()?;

        let interface_name = InterfaceName::from_static_str("org.freedesktop.UPower.Device")
            .expect("failed to create zbus InterfaceName");

        Ok(Arc::new(Self {
            proxy,
            interface_name,
        }))
    }
}

impl Deref for Client {
    type Target = PropertiesProxy<'static>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

#[cfg(feature = "ipc")]
impl crate::ironvar::Namespace for Client {
    fn get(&self, key: &str) -> Option<String> {
        let value =
            await_sync(async { self.proxy.get(self.interface_name.clone(), key).await }).ok();
        value.map(|v| v.to_string())
    }

    fn list(&self) -> Vec<String> {
        self.get_all().keys().map(ToString::to_string).collect()
    }

    fn get_all(&self) -> HashMap<Box<str>, String> {
        let properties =
            await_sync(async { self.proxy.get_all(self.interface_name.clone()).await })
                .ok()
                .unwrap_or_default();

        properties
            .into_iter()
            .map(|(k, v)| (k.into(), v.to_string()))
            .collect()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

register_fallible_client!(Client, upower);
