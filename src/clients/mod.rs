use crate::{Ironbar, await_sync};
use color_eyre::Result;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(feature = "workspaces")]
pub mod compositor;
#[cfg(feature = "keyboard")]
pub mod libinput;
#[cfg(feature = "cairo")]
pub mod lua;
#[cfg(feature = "music")]
pub mod music;
#[cfg(feature = "network_manager")]
pub mod networkmanager;
#[cfg(feature = "sway")]
pub mod sway;
#[cfg(feature = "notifications")]
pub mod swaync;
#[cfg(feature = "sys_info")]
pub mod sysinfo;
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
    #[cfg(feature = "sway")]
    sway: Option<Arc<sway::Client>>,
    #[cfg(feature = "hyprland")]
    hyprland: Option<Arc<compositor::hyprland::Client>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Arc<clipboard::Client>>,
    #[cfg(feature = "keyboard")]
    libinput: HashMap<Box<str>, Arc<libinput::Client>>,
    #[cfg(any(feature = "keyboard+sway", feature = "keyboard+hyprland"))]
    keyboard_layout: Option<Arc<dyn compositor::KeyboardLayoutClient>>,
    #[cfg(feature = "cairo")]
    lua: Option<Rc<lua::LuaEngine>>,
    #[cfg(feature = "music")]
    music: HashMap<music::ClientType, Arc<dyn music::MusicClient>>,
    #[cfg(feature = "network_manager")]
    network_manager: Option<Arc<networkmanager::Client>>,
    #[cfg(feature = "notifications")]
    notifications: Option<Arc<swaync::Client>>,
    #[cfg(feature = "sys_info")]
    sys_info: Option<Arc<sysinfo::Client>>,
    #[cfg(feature = "tray")]
    tray: Option<Arc<tray::Client>>,
    #[cfg(feature = "upower")]
    upower: Option<Arc<zbus::fdo::PropertiesProxy<'static>>>,
    #[cfg(feature = "volume")]
    volume: Option<Arc<volume::Client>>,
}

pub type ClientResult<T> = Result<Arc<T>>;

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
    pub fn workspaces(&mut self) -> ClientResult<dyn compositor::WorkspaceClient> {
        let client = if let Some(workspaces) = &self.workspaces {
            workspaces.clone()
        } else {
            let client = compositor::Compositor::create_workspace_client(self)?;
            self.workspaces.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(any(feature = "keyboard+sway", feature = "keyboard+hyprland"))]
    pub fn keyboard_layout(&mut self) -> ClientResult<dyn compositor::KeyboardLayoutClient> {
        let client = if let Some(keyboard_layout) = &self.keyboard_layout {
            keyboard_layout.clone()
        } else {
            let client = compositor::Compositor::create_keyboard_layout_client(self)?;
            self.keyboard_layout.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "sway")]
    pub fn sway(&mut self) -> ClientResult<sway::Client> {
        let client = if let Some(client) = &self.sway {
            client.clone()
        } else {
            let client = await_sync(async { sway::Client::new().await })?;
            let client = Arc::new(client);
            self.sway.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "hyprland")]
    pub fn hyprland(&mut self) -> Arc<compositor::hyprland::Client> {
        if let Some(client) = &self.hyprland {
            client.clone()
        } else {
            let client = Arc::new(compositor::hyprland::Client::new());
            self.hyprland.replace(client.clone());
            client
        }
    }

    #[cfg(feature = "cairo")]
    pub fn lua(&mut self, config_dir: &Path) -> Rc<lua::LuaEngine> {
        self.lua
            .get_or_insert_with(|| Rc::new(lua::LuaEngine::new(config_dir)))
            .clone()
    }

    #[cfg(feature = "keyboard")]
    pub fn libinput(&mut self, seat: &str) -> ClientResult<libinput::Client> {
        if let Some(client) = self.libinput.get(seat) {
            Ok(client.clone())
        } else {
            let client = libinput::Client::init(seat.to_string())?;
            self.libinput.insert(seat.into(), client.clone());
            Ok(client)
        }
    }

    #[cfg(feature = "music")]
    pub fn music(&mut self, client_type: music::ClientType) -> Arc<dyn music::MusicClient> {
        self.music
            .entry(client_type.clone())
            .or_insert_with(|| music::create_client(client_type))
            .clone()
    }

    #[cfg(feature = "network_manager")]
    pub fn network_manager(&mut self) -> ClientResult<networkmanager::Client> {
        if let Some(client) = &self.network_manager {
            Ok(client.clone())
        } else {
            let client = await_sync(async move { networkmanager::create_client().await })?;
            self.network_manager = Some(client.clone());
            Ok(client)
        }
    }

    #[cfg(feature = "notifications")]
    pub fn notifications(&mut self) -> ClientResult<swaync::Client> {
        let client = if let Some(client) = &self.notifications {
            client.clone()
        } else {
            let client = await_sync(async { swaync::Client::new().await })?;
            let client = Arc::new(client);
            self.notifications.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "sys_info")]
    pub fn sys_info(&mut self) -> Arc<sysinfo::Client> {
        self.sys_info
            .get_or_insert_with(|| {
                let client = Arc::new(sysinfo::Client::new());
                Ironbar::variable_manager().register_namespace("sysinfo", client.clone());
                client
            })
            .clone()
    }

    #[cfg(feature = "tray")]
    pub fn tray(&mut self) -> ClientResult<tray::Client> {
        let client = if let Some(client) = &self.tray {
            client.clone()
        } else {
            let client = await_sync(async { tray::Client::new().await })?;
            let client = Arc::new(client);
            self.tray.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "upower")]
    pub fn upower(&mut self) -> ClientResult<zbus::fdo::PropertiesProxy<'static>> {
        let client = if let Some(client) = &self.upower {
            client.clone()
        } else {
            let client = await_sync(async { upower::create_display_proxy().await })?;
            self.upower.replace(client.clone());
            client
        };

        Ok(client)
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

/// Types implementing this trait
/// indicate that they provide a singleton client instance of type `T`,
/// which may fail to be created.
pub trait ProvidesFallibleClient<T: ?Sized> {
    /// Returns a singleton client instance of type `T`.
    fn try_provide(&self) -> ClientResult<T>;
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

/// Generates a `ProvidesClient` impl block on `WidgetContext`
/// for the provided `$ty` (first argument) client type.
///
/// The implementation calls `$method` (second argument)
/// on the `Clients` struct to obtain the client instance.
///
/// # Example
/// `register_client!(Client, clipboard);`
#[macro_export]
macro_rules! register_fallible_client {
    ($ty:ty, $method:ident) => {
        impl<TSend, TReceive> $crate::clients::ProvidesFallibleClient<$ty>
            for $crate::modules::WidgetContext<TSend, TReceive>
        where
            TSend: Clone,
        {
            fn try_provide(&self) -> color_eyre::Result<std::sync::Arc<$ty>> {
                self.ironbar.clients.borrow_mut().$method()
            }
        }
    };
}
