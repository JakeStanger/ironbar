use crate::{Ironbar, await_sync};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::broadcast;

#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(feature = "brightness")]
pub mod brightness;
#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(any(
    feature = "bindmode",
    feature = "hyprland",
    feature = "keyboard",
    feature = "workspaces",
))]
pub mod compositor;
#[cfg(feature = "inhibit")]
pub mod inhibit;
#[cfg(feature = "keyboard")]
pub mod libinput;
#[cfg(feature = "cairo")]
pub mod lua;
#[cfg(feature = "music")]
pub mod music;
#[cfg(feature = "network_manager")]
pub mod networkmanager;
pub mod outputs;
#[cfg(feature = "sway")]
pub mod sway;
#[cfg(feature = "notifications")]
pub mod swaync;
#[cfg(feature = "sys_info")]
pub mod sysinfo;
#[cfg(feature = "tray")]
pub mod tray;
#[cfg(feature = "battery")]
pub mod upower;
#[cfg(feature = "volume")]
pub mod volume;
pub mod wayland;

#[derive(Debug, thiserror::Error)]
pub enum Error<E: std::error::Error + Send + Sync> {
    #[error("Failed to get client from registry")]
    Registry,
    #[error("Failed to start client")]
    Start(#[source] E),
    #[allow(dead_code)]
    #[error("Failed to stop client")]
    Stop(#[source] E),
}

pub type Result<T, E> = std::result::Result<T, Error<E>>;

pub enum RegistryEntry<T> {
    Owned(T),
    Shared(Arc<T>),
}

pub trait Client: Debug + Any + Sized
where
    <Self as Client>::Error: std::error::Error + Send + Sync,
{
    /// The current client data, used to initialize a module.
    type State;
    /// The type of event emitted by the client.
    type Event;
    /// The general error type raised by the client,
    /// eg `zbus::Error` for DBus-based clients.
    type Error;

    /// Returns whether the client has been initialized
    /// and is currently in a running state.
    fn is_started(&self) -> bool;

    /// Initializes the client,
    /// creating any connections that should persist for its lifetime.
    ///
    /// This takes `self`, allowing for the client to be mutated freely.
    /// The mutated client is then returned.
    ///
    /// If the client is already running, this should be a successful no-op.
    fn start(self) -> impl Future<Output = std::result::Result<Self, Self::Error>>;

    /// Stops the client, closing any open connections
    /// and clearing any internal state.
    ///
    /// This takes `self`, allowing for the client to be mutated freely.
    /// The mutated client is then returned.
    ///
    /// If the client is already stopped, this should be a successful no-op.
    #[allow(dead_code)]
    fn stop(self) -> impl Future<Output = std::result::Result<Self, Self::Error>>;

    /// Gets the current data for the system.
    /// This should be used when initializing a module.
    fn state(&self) -> impl Future<Output = Self::State>;

    /// Returns a broadcast receiver for the client's events.
    fn subscribe(&self) -> broadcast::Receiver<Self::Event>;
}

/// Singleton wrapper consisting of
/// all the singleton client types used by modules.
#[derive(Debug, Default)]
pub struct Clients {
    registry: HashMap<TypeId, Box<dyn Any>>,

    // -- old - to sort -- \\

    wayland: Option<Arc<wayland::Client>>,
    outputs: Option<Arc<outputs::Client>>,
    #[cfg(feature = "workspaces")]
    workspaces: Option<Arc<dyn compositor::WorkspaceClient>>,
    #[cfg(feature = "sway")]
    sway: Option<Arc<sway::Client>>,
    #[cfg(feature = "hyprland")]
    hyprland: Option<Arc<compositor::hyprland::Client>>,
    #[cfg(feature = "bindmode")]
    bindmode: Option<Arc<dyn compositor::BindModeClient>>,
    #[cfg(feature = "clipboard")]
    clipboard: Option<Arc<clipboard::Client>>,
    #[cfg(feature = "inhibit")]
    inhibit: Option<Arc<inhibit::Client>>,
    #[cfg(feature = "keyboard")]
    libinput: HashMap<Box<str>, Arc<libinput::Client>>,
    #[cfg(feature = "keyboard")]
    keyboard_layout: Option<Arc<dyn compositor::KeyboardLayoutClient>>,
    #[cfg(feature = "cairo")]
    lua: Option<Rc<lua::LuaEngine>>,
    #[cfg(feature = "music")]
    music: HashMap<music::ClientType, Arc<dyn music::MusicClient>>,
    #[cfg(feature = "network_manager")]
    network_manager: Option<Arc<networkmanager::Client>>,
    #[cfg(feature = "sys_info")]
    sys_info: Option<Arc<sysinfo::Client>>,
    #[cfg(feature = "tray")]
    tray: Option<Arc<tray::Client>>,
    #[cfg(feature = "brightness")]
    brightness: Option<Arc<brightness::Client>>,
    #[cfg(feature = "battery")]
    upower: Option<Arc<upower::Client>>,
    #[cfg(feature = "volume")]
    volume: Option<Arc<volume::Client>>,
    #[cfg(feature = "bluetooth")]
    bluetooth: Option<Arc<bluetooth::Client>>,
}

pub type ClientResult<T> = color_eyre::Result<Arc<T>>;

impl Clients {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    async fn get<T>(&mut self) -> Result<Arc<T>, T::Error>
    where
        T: Client + 'static,
        <T as Client>::Error: std::error::Error + Send + Sync,
    {
        let key = TypeId::of::<T>();

        let client = self
            .registry
            .remove(&key)
            .ok_or(Error::<T::Error>::Registry)?;

        let client = client
            .downcast::<RegistryEntry<T>>()
            .expect("should be registry entry");

        let client = match *client {
            RegistryEntry::Owned(client) => {
                let client = if client.is_started() {
                    client
                } else {
                    client.
                        start().await.map_err(Error::<T::Error>::Start)?
                };

                Arc::new(client)
            }
            RegistryEntry::Shared(client) => client,
        };

        self.registry
            .insert(key, Box::new(RegistryEntry::Shared(client.clone())));

        Ok(client)
    }

    /// Adds a new client into the registry.
    /// The registered client should be in the stopped state.
    fn register<T>(&mut self, cl: T)
    where
        T: Client + Any,
    {
        self.registry
            .insert(TypeId::of::<T>(), Box::new(RegistryEntry::Owned(cl)));
    }

    pub fn wayland(&mut self) -> Arc<wayland::Client> {
        self.wayland
            .get_or_insert_with(|| Arc::new(wayland::Client::new()))
            .clone()
    }

    pub fn outputs(&mut self) -> Arc<outputs::Client> {
        self.outputs
            .get_or_insert_with(|| Arc::new(outputs::Client::new()))
            .clone()
    }

    #[cfg(feature = "clipboard")]
    pub fn clipboard(&mut self) -> Arc<clipboard::Client> {
        let wayland = self.wayland();

        self.clipboard
            .get_or_insert_with(|| Arc::new(clipboard::Client::new(wayland)))
            .clone()
    }

    #[cfg(feature = "inhibit")]
    pub fn inhibit(&mut self) -> Arc<inhibit::Client> {
        self.inhibit
            .get_or_insert_with(|| Arc::new(inhibit::Client::new()))
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

    #[cfg(feature = "keyboard")]
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

    #[cfg(feature = "bindmode")]
    pub fn bindmode(&mut self) -> ClientResult<dyn compositor::BindModeClient> {
        let client = if let Some(client) = &self.bindmode {
            client.clone()
        } else {
            let client = compositor::Compositor::create_bindmode_client(self)?;
            self.bindmode.replace(client.clone());
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
    pub fn libinput(&mut self, seat: &str) -> Arc<libinput::Client> {
        if let Some(client) = self.libinput.get(seat) {
            client.clone()
        } else {
            let client = libinput::Client::init(seat.to_string());
            self.libinput.insert(seat.into(), client.clone());
            client
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
        if !self.registry.contains_key(&TypeId::of::<swaync::Client>()) {
            self.register(swaync::Client::new());
        }

        let client = await_sync(async move { self.get::<swaync::Client>().await });
        client.map_err(Into::into)
    }

    #[cfg(feature = "sys_info")]
    pub fn sys_info(&mut self) -> Arc<sysinfo::Client> {
        self.sys_info
            .get_or_insert_with(|| {
                let client = Arc::new(sysinfo::Client::new());

                #[cfg(any(feature = "ipc", feature = "cairo"))]
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
            self.tray.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "brightness")]
    pub fn brightness(&mut self) -> ClientResult<brightness::Client> {
        let client = if let Some(client) = &self.brightness {
            client.clone()
        } else {
            let client = await_sync(async { brightness::Client::new().await })?;

            #[cfg(feature = "ipc")]
            Ironbar::variable_manager().register_namespace("brightness", client.clone());

            self.brightness.replace(client.clone());
            client
        };

        Ok(client)
    }

    #[cfg(feature = "battery")]
    pub fn upower(&mut self) -> ClientResult<upower::Client> {
        let client = if let Some(client) = &self.upower {
            client.clone()
        } else {
            let client = await_sync(async { upower::Client::new().await })?;

            #[cfg(any(feature = "ipc", feature = "cairo"))]
            Ironbar::variable_manager().register_namespace("upower", client.clone());

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

    #[cfg(feature = "bluetooth")]
    pub fn bluetooth(&mut self) -> ClientResult<bluetooth::Client> {
        let client = if let Some(client) = &self.bluetooth {
            client.clone()
        } else {
            let client = await_sync(async { bluetooth::Client::new().await })?;
            let client = Arc::new(client);
            self.bluetooth.replace(client.clone());
            client
        };

        Ok(client)
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
