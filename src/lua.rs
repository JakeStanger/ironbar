use mlua::{Error, IntoLua, Lua, UserData, UserDataFields, UserDataMethods, Value};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{ops::Deref, sync::Arc};
use tracing::{debug, error, info, warn};

use crate::clients::Clients;
use crate::clients::lua::LuaClients;
use crate::ironvar::{Namespace, VariableManager, WritableNamespace};
use crate::{Ironbar, await_sync, spawn};

/// Wrapper around Lua instance
/// to create a singleton and handle initialisation.
#[derive(Debug)]
pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    pub fn new(config_dir: &Path, clients: Arc<Mutex<Clients>>) -> Self {
        // SAFETY: Use `unsafe_new` to allow loading of C modules.
        let lua = unsafe { Lua::unsafe_new() };

        if let Err(err) = lua
            .globals()
            .set("ironbar", IronbarUserData::new(config_dir, clients))
        {
            error!("{err:?}");
        }

        let user_init = config_dir.join("init.lua");
        if user_init.exists() {
            debug!("loading user init script");

            if let Err(err) = await_sync(async { lua.load(user_init).exec_async().await }) {
                error!("{err:?}");
            }
        }

        debug!("loading internal init script");
        if let Err(err) = lua.load(include_str!("../lua/init.lua")).exec() {
            error!("{err:?}");
        }

        Self { lua }
    }
}

impl Deref for LuaEngine {
    type Target = Lua;

    fn deref(&self) -> &Self::Target {
        &self.lua
    }
}

struct IronbarUserData {
    variable_manager: Arc<VariableManager>,
    config_dir: String,
    clients: Arc<Mutex<Clients>>,
}

impl IronbarUserData {
    fn new(config_dir: &Path, clients: Arc<Mutex<Clients>>) -> Self {
        IronbarUserData {
            variable_manager: Ironbar::variable_manager().clone(),
            config_dir: config_dir.to_string_lossy().into(),
            clients,
        }
    }

    fn var_get(&self, lua: &Lua, mut key: String) -> Result<Value, Error> {
        let mut ns: Arc<dyn Namespace + Sync + Send> = self.variable_manager.clone();

        if key.contains('.') {
            for part in key.split('.') {
                ns = if let Some(ns) = ns.get_namespace(part) {
                    ns.clone()
                } else {
                    key = part.into();
                    break;
                };
            }
        }

        match ns.get(&key) {
            Some(value) => Self::to_value(lua, value),
            None => Err(Error::RuntimeError(format!("Variable not found: {key}"))),
        }
    }

    fn var_list(&self, lua: &Lua, namespace: Option<String>) -> Result<Value, Error> {
        let mut ns: Arc<dyn Namespace + Sync + Send> = self.variable_manager.clone();

        if let Some(namespace) = namespace {
            for part in namespace.split('.') {
                ns = match ns.get_namespace(part) {
                    Some(ns) => ns.clone(),
                    None => {
                        return Err(Error::RuntimeError(format!("Namespace not found: {part}")));
                    }
                };
            }
        }

        let table = lua.create_table()?;

        for (key, value) in ns.get_all() {
            table.set(key, Self::to_value(lua, value)?)?;
        }

        Ok(Value::Table(table))
    }

    fn var_set(&self, key: String, value: Value) -> Result<Value, Error> {
        match self.variable_manager.set(&key, value.to_string()?) {
            Ok(_) => Ok(Value::Nil),
            Err(err) => Err(Error::RuntimeError(format!(
                "Failed to set variable: {err}"
            ))),
        }
    }

    fn to_value(lua: &Lua, value: String) -> Result<Value, Error> {
        if let Ok(i) = value.parse::<i64>() {
            i.into_lua(lua)
        } else if let Ok(f) = value.parse::<f64>() {
            f.into_lua(lua)
        } else {
            value.into_lua(lua)
        }
    }

    fn unixtime(lua: &Lua) -> Result<Value, Error> {
        let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(now) => now,
            Err(err) => return Err(Error::RuntimeError(format!("SystemTime: {err}"))),
        };
        let table = lua.create_table()?;
        table.set("secs", now.as_secs_f64())?;
        table.set("subsec_millis", now.subsec_millis())?;
        table.set("subsec_micros", now.subsec_micros())?;

        Ok(Value::Table(table))
    }
}

impl UserData for IronbarUserData {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("config_dir", |lua, this| {
            lua.create_string(&this.config_dir)
        });

        fields.add_field_method_get("clients", |lua, this| {
            lua.create_userdata(LuaClients(this.clients.clone()))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("log_debug", |_, message: String| {
            debug!(message);
            Ok(())
        });
        methods.add_function("log_info", |_, message: String| {
            info!(message);
            Ok(())
        });
        methods.add_function("log_warn", |_, message: String| {
            warn!(message);
            Ok(())
        });
        methods.add_function("log_error", |_, message: String| {
            error!(message);
            Ok(())
        });
        methods.add_function("unixtime", |lua, ()| Self::unixtime(lua));
        methods.add_method("var_get", |lua, this, key| this.var_get(lua, key));
        methods.add_method("var_list", |lua, this, namespace| {
            this.var_list(lua, namespace)
        });
        methods.add_method("var_set", |_, this, (key, value)| this.var_set(key, value));

        methods.add_function("spawn", |lua, func: mlua::Function| {
            let thread = lua.create_thread(func)?;
            let fut = thread.into_async::<()>(())?;

            spawn(async move {
                if let Err(err) = fut.await {
                    error!("lua task failed: {err}");
                }
            });

            Ok(())
        });

        methods.add_async_function("sleep", |_, ms: f64| async move {
            tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
            Ok(())
        });
    }
}
