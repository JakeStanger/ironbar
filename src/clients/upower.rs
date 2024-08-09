use crate::clients::ClientResult;
use crate::register_fallible_client;
use std::sync::Arc;
use upower_dbus::UPowerProxy;
use zbus::fdo::PropertiesProxy;

pub async fn create_display_proxy() -> ClientResult<PropertiesProxy<'static>> {
    let dbus = Box::pin(zbus::Connection::system()).await?;

    let device_proxy = UPowerProxy::new(&dbus).await?;

    let display_device = device_proxy.get_display_device().await?;

    let path = display_device.path().to_owned();

    let proxy = PropertiesProxy::builder(&dbus)
        .destination("org.freedesktop.UPower")
        .expect("failed to set proxy destination address")
        .path(path)
        .expect("failed to set proxy path")
        .cache_properties(zbus::CacheProperties::No)
        .build()
        .await?;

    Ok(Arc::new(proxy))
}

register_fallible_client!(PropertiesProxy<'static>, upower);
