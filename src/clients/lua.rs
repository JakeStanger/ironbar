use mlua::Lua;
use std::ops::Deref;
use std::path::Path;
use tracing::{debug, error};

/// Wrapper around Lua instance
/// to create a singleton and handle initialization.
#[derive(Debug)]
pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    pub fn new(config_dir: &Path) -> Self {
        let lua = unsafe { Lua::unsafe_new() };

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
