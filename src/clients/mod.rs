use crate::Ironbar;
use std::sync::Arc;

#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(feature = "workspaces")]
pub mod compositor;
#[cfg(feature = "music")]
pub mod music;
#[cfg(feature = "networkmanager")]
pub mod networkmanager;
#[cfg(feature = "notifications")]
pub mod swaync;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "upower")]
pub mod upower;
#[cfg(feature = "volume")]
pub mod volume;
pub mod wayland;

/// Singleton wrapper consisting of
/// all the singleton client types used by modules.
#[derive(Debug, Default)]
pub struct Clients {
    wayland: Option<Arc<wayland::Client>>,
    #[cfg(feature = "workspaces")]
    workspaces: Option<Arc<dyn compositor::WorkspaceClient>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Arc<clipboard::Client>>,
    #[cfg(feature = "music")]
    music: std::collections::HashMap<music::ClientType, Arc<dyn music::MusicClient>>,
    #[cfg(feature = "networkmanager")]
    networkmanager: Option<Arc<networkmanager::Client>>,
    #[cfg(feature = "notifications")]
    notifications: Option<Arc<swaync::Client>>,
    #[cfg(feature = "tray")]
    tray: Option<Arc<tray::Client>>,
    #[cfg(feature = "upower")]
    upower: Option<Arc<zbus::fdo::PropertiesProxy<'static>>>,
    #[cfg(feature = "volume")]
    volume: Option<Arc<volume::Client>>,
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn wayland(&mut self) -> Arc<wayland::Client> {
        self.wayland
            .get_or_insert_with(|| Arc::new(wayland::Client::new()))
            .clone()
    }

    #[cfg(feature = "clipboard")]
    pub fn clipboard(&mut self) -> Arc<clipboard::Client> {
        let wayland = self.wayland();

        self.clipboard
            .get_or_insert_with(|| Arc::new(clipboard::Client::new(wayland)))
            .clone()
    }

    #[cfg(feature = "workspaces")]
    pub fn workspaces(&mut self) -> Arc<dyn compositor::WorkspaceClient> {
        // TODO: Error handling here isn't great - should throw a user-friendly error & exit
        self.workspaces
            .get_or_insert_with(|| {
                compositor::Compositor::create_workspace_client().expect("to be valid compositor")
            })
            .clone()
    }

    #[cfg(feature = "music")]
    pub fn music(&mut self, client_type: music::ClientType) -> Arc<dyn music::MusicClient> {
        self.music
            .entry(client_type.clone())
            .or_insert_with(|| music::create_client(client_type))
            .clone()
    }

    #[cfg(feature = "networkmanager")]
    pub fn networkmanager(&mut self) -> Arc<networkmanager::Client> {
        self.networkmanager
            .get_or_insert_with(networkmanager::create_client)
            .clone()
    }

    #[cfg(feature = "notifications")]
    pub fn notifications(&mut self) -> Arc<swaync::Client> {
        self.notifications
            .get_or_insert_with(|| {
                Arc::new(crate::await_sync(async { swaync::Client::new().await }))
            })
            .clone()
    }

    #[cfg(feature = "tray")]
    pub fn tray(&mut self) -> Arc<tray::Client> {
        // TODO: Error handling here isn't great - should throw a user-friendly error
        self.tray
            .get_or_insert_with(|| {
                Arc::new(crate::await_sync(async {
                    let service_name =
                        format!("{}-{}", env!("CARGO_CRATE_NAME"), Ironbar::unique_id());

                    tray::Client::new(&service_name)
                        .await
                        .expect("to be able to start client")
                }))
            })
            .clone()
    }

    #[cfg(feature = "upower")]
    pub fn upower(&mut self) -> Arc<zbus::fdo::PropertiesProxy<'static>> {
        self.upower
            .get_or_insert_with(|| {
                crate::await_sync(async { upower::create_display_proxy().await })
            })
            .clone()
    }

    #[cfg(feature = "volume")]
    pub fn volume(&mut self) -> Arc<volume::Client> {
        self.volume
            .get_or_insert_with(volume::create_client)
            .clone()
    }
}

/// Types implementing this trait
/// indicate that they provide a singleton client instance of type `T`.
pub trait ProvidesClient<T: ?Sized> {
    /// Returns a singleton client instance of type `T`.
    fn provide(&self) -> Arc<T>;
}

/// Generates a `ProvidesClient` impl block on `WidgetContext`
/// for the provided `$ty` (first argument) client type.
///
/// The implementation calls `$method` (second argument)
/// on the `Clients` struct to obtain the client instance.
///
/// # Example
/// `register_client!(Client, clipboard);`
#[macro_export]
macro_rules! register_client {
    ($ty:ty, $method:ident) => {
        impl<TSend, TReceive> $crate::clients::ProvidesClient<$ty>
            for $crate::modules::WidgetContext<TSend, TReceive>
        where
            TSend: Clone,
        {
            fn provide(&self) -> std::sync::Arc<$ty> {
                self.ironbar.clients.borrow_mut().$method()
            }
        }
    };
}
