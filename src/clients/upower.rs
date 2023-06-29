use async_once::AsyncOnce;
use lazy_static::lazy_static;
use std::sync::Arc;
use upower_dbus::UPowerProxy;
use zbus::fdo::PropertiesProxy;

lazy_static! {
    static ref DISPLAY_PROXY: AsyncOnce<Arc<PropertiesProxy<'static>>> = AsyncOnce::new(async {
        let dbus = Box::pin(zbus::Connection::system())
            .await
            .expect("failed to create connection to system bus");

        let device_proxy = UPowerProxy::new(&dbus)
            .await
            .expect("failed to create upower proxy");

        let display_device = device_proxy
            .get_display_device()
            .await
            .unwrap_or_else(|_| panic!("failed to get display device for {device_proxy:?}"));

        let path = display_device.path().to_owned();

        let proxy = PropertiesProxy::builder(&dbus)
            .destination("org.freedesktop.UPower")
            .expect("failed to set proxy destination address")
            .path(path)
            .expect("failed to set proxy path")
            .cache_properties(zbus::CacheProperties::No)
            .build()
            .await
            .expect("failed to build proxy");

        Arc::new(proxy)
    });
}

pub async fn get_display_proxy() -> &'static PropertiesProxy<'static> {
    DISPLAY_PROXY.get().await
}
