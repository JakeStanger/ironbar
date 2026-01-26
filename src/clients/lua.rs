use mlua::{Error, IntoLua, Lua, UserData, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{ops::Deref, sync::Arc};
use tracing::{debug, error, info, warn};

use crate::Ironbar;
use crate::ironvar::{Namespace, VariableManager};

/// Wrapper around Lua instance
/// to create a singleton and handle initialization.
#[derive(Debug)]
pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    pub fn new(config_dir: &Path) -> Self {
        let lua = unsafe { Lua::unsafe_new() };

        if let Err(err) = lua
            .globals()
            .set("ironbar", IronbarUserData::new(config_dir))
        {
            error!("{err:?}");
        }

        let user_init = config_dir.join("init.lua");
        if user_init.exists() {
            debug!("loading user init script");

            if let Err(err) = lua.load(user_init).exec() {
                error!("{err:?}");
            }
        }

        debug!("loading internal init script");
        if let Err(err) = lua.load(include_str!("../../lua/init.lua")).exec() {
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
}

impl IronbarUserData {
    fn new(config_dir: &Path) -> Self {
        IronbarUserData {
            variable_manager: Ironbar::variable_manager().clone(),
            config_dir: config_dir.to_string_lossy().into(),
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
            None => Err(Error::RuntimeError(format!("Variable not found: {}", key))),
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
            Err(err) => return Err(Error::RuntimeError(format!("SystemTime: {}", err))),
        };
        let table = lua.create_table()?;
        table.set("secs", now.as_secs_f64())?;
        table.set("subsec_millis", now.subsec_millis())?;
        table.set("subsec_micros", now.subsec_micros())?;

        Ok(Value::Table(table))
    }
}

impl UserData for IronbarUserData {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("config_dir", |lua, this| {
            lua.create_string(&this.config_dir)
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("log_debug", |_, _, message: String| {
            debug!(message);
            Ok(())
        });
        methods.add_method("log_info", |_, _, message: String| {
            info!(message);
            Ok(())
        });
        methods.add_method("log_warn", |_, _, message: String| {
            warn!(message);
            Ok(())
        });
        methods.add_method("log_error", |_, _, message: String| {
            error!(message);
            Ok(())
        });
        methods.add_method("unixtime", |lua, _, ()| Self::unixtime(lua));
        methods.add_method("var_get", |lua, this, key| this.var_get(lua, key));
        methods.add_method("var_list", |lua, this, namespace| {
            this.var_list(lua, namespace)
        });
    }
}
