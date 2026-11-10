use crate::clients::{Client, Clients};
use crate::spawn;
use mlua::{Error, Function, MetaMethod, UserData, UserDataMethods, Value};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use tracing::error;

/// Lua handle to the client registry, exposed as `ironbar.clients`.
pub struct LuaClients(pub Arc<Mutex<Clients>>);

/// Generates the name -> accessor dispatch for singleton clients.
macro_rules! lua_clients {
    ($lua:expr, $clients:expr, $name:expr, { $($(#[$meta:meta])* $key:literal => $method:ident),* $(,)? }) => {{
        // avoid unused warnings when no client features are enabled
        let _ = (&$lua, &$clients);
        match $name {
            $(
                $(#[$meta])*
                $key => {
                    let client = crate::lock!($clients).$method();
                    mlua::IntoLua::into_lua(LuaClient(client.map_err(Error::external)?), $lua)
                }
            )*
            _ => Ok(Value::Nil),
        }
    }};
}

impl UserData for LuaClients {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |lua, this, name: String| {
            // keyed clients are exposed as functions taking the key,
            // so they are accessed with `.` like singleton clients.
            #[cfg(feature = "keyboard")]
            if name == "libinput" {
                let clients = this.0.clone();
                let func = lua.create_function(move |lua, seat: String| {
                    let client = crate::lock!(clients).libinput(&seat);
                    mlua::IntoLua::into_lua(LuaClient(client.map_err(Error::external)?), lua)
                })?;
                return Ok(Value::Function(func));
            }

            lua_clients!(lua, this.0, name.as_str(), {
                #[cfg(feature = "notifications")]
                "notifications" => notifications,
                #[cfg(feature = "battery")]
                "upower" => upower,
                #[cfg(feature = "power_profiles")]
                "power_profiles" => power_profiles,
                #[cfg(feature = "bluetooth")]
                "bluetooth" => bluetooth,
                #[cfg(feature = "sys_info")]
                "sysinfo" => sys_info,
            })
        });
    }
}

/// Lua wrapper around a client,
/// exposing its state and event stream.
pub struct LuaClient<C: Client>(pub Arc<C>);

impl<C: Client> Clone for LuaClient<C> {
    fn clone(&self) -> Self {
        LuaClient(self.0.clone())
    }
}

impl<C> UserData for LuaClient<C>
where
    C: Client + 'static,
    <C as Client>::State: UserData + Send + Sync + 'static,
    <C as Client>::Event: UserData + Send + Sync + Clone + 'static,
    <C as Client>::Error: 'static,
{
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("state", async |_, client, _: ()| {
            client.0.state().await.map_err(Error::external)
        });

        methods.add_method("subscribe", |_, client, func: Function| {
            let mut rx = client.0.subscribe();

            let handle = spawn(async move {
                while let Ok(ev) = rx.recv().await {
                    if let Err(err) = func.call_async::<()>(ev).await {
                        error!("Error calling Lua function: {err}");
                    }
                }
            });

            Ok(LuaSubscription(handle))
        });

        C::add_lua_methods(methods);
    }
}

/// Handle to a client subscription created from Lua.
#[allow(dead_code)] // unused if no registry client features are enabled
struct LuaSubscription(JoinHandle<()>);

impl UserData for LuaSubscription {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("unsubscribe", |_, this, ()| {
            this.0.abort();
            Ok(())
        });
    }
}

/// Implements [`mlua::IntoLua`] for each type,
/// converting to a Lua string using its [`Debug`] impl.
macro_rules! into_lua_debug {
    ($($ty:ty),* $(,)?) => {
        $(
            impl mlua::IntoLua for $ty {
                fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
                    mlua::IntoLua::into_lua(format!("{self:?}"), lua)
                }
            }
        )*
    };
}

pub(crate) use into_lua_debug;
