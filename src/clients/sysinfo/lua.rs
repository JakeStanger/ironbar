use super::{Client, Function, Prefix, RefreshType, TokenType, Value, ValueSet};
use crate::clients::lua::LuaClient;
use mlua::{Error, Lua, Result, Table, UserDataMethods};
use std::str::FromStr;

/// Registers a method per token,
/// which refreshes its data returns the current value.
pub(super) fn add_methods<M: UserDataMethods<LuaClient<Client>>>(methods: &mut M) {
    macro_rules! getters {
        ($($name:ident => $token:ident),* $(,)?) => {$(
            methods.add_method(stringify!($name), |_, client, ()| {
                let data = client.0.data().map_err(Error::external)?;
                data.refresh(TokenType::$token.refresh_type());
                Ok(data.$name())
            });
        )*};
    }

    getters!(
        cpu_frequency => CpuFrequency,
        cpu_percent => CpuPercent,
        memory_free => MemoryFree,
        memory_available => MemoryAvailable,
        memory_total => MemoryTotal,
        memory_used => MemoryUsed,
        memory_percent => MemoryPercent,
        swap_free => SwapFree,
        swap_total => SwapTotal,
        swap_used => SwapUsed,
        swap_percent => SwapPercent,
        gpu_percent => GpuPercent,
        gpu_memory_free => GpuMemoryFree,
        gpu_memory_total => GpuMemoryTotal,
        gpu_memory_used => GpuMemoryUsed,
        gpu_memory_percent => GpuMemoryPercent,
        temp_c => TempC,
        temp_f => TempF,
        disk_free => DiskFree,
        disk_total => DiskTotal,
        disk_used => DiskUsed,
        disk_percent => DiskPercent,
        disk_read => DiskRead,
        disk_write => DiskWrite,
        net_down => NetDown,
        net_up => NetUp,
        load_average_1 => LoadAverage1,
        load_average_5 => LoadAverage5,
        load_average_15 => LoadAverage15,
    );

    methods.add_method("uptime", |_, _, ()| Ok(Client::uptime()));
}

/// Parses an optional unit prefix such as `"M"` or `"Gi"`,
/// defaulting to no prefix.
fn parse_prefix(prefix: Option<String>) -> Result<Prefix> {
    prefix.map_or(Ok(Prefix::None), |prefix| {
        Prefix::from_str(&prefix).map_err(|err| Error::runtime(err.to_string()))
    })
}

#[mlua::userdata_impl]
impl RefreshType {
    /// The kind of data that was refreshed.
    #[lua(getter, name = "kind", infallible)]
    fn lua_kind(&self) -> String {
        format!("{self:?}")
    }

    #[lua(meta, name = "__tostring", infallible)]
    fn lua_tostring(&self) -> String {
        self.lua_kind()
    }
}

#[mlua::userdata_impl]
impl Value {
    /// The value in its base unit.
    #[lua(getter, name = "value", infallible)]
    fn lua_value(&self) -> f64 {
        self.get(Prefix::None)
    }

    /// Gets the value scaled to the given unit prefix.
    #[lua(name = "get")]
    fn lua_get(&self, prefix: Option<String>) -> Result<f64> {
        Ok(self.get(parse_prefix(prefix)?))
    }
}

#[mlua::userdata_impl]
impl ValueSet {
    /// Table of each named value in its base unit.
    #[lua(getter, name = "values")]
    fn lua_values(&self, lua: &Lua) -> Result<Table> {
        lua.create_table_from(
            self.values
                .iter()
                .map(|(name, value)| (name.to_string(), value.get(Prefix::None))),
        )
    }

    /// Applies a function (`sum`, `min`, `max`, `mean` or a value name) to the set,
    /// scaled to the given unit prefix.
    #[lua(name = "get")]
    fn lua_get(&self, function: String, prefix: Option<String>) -> Result<f64> {
        let function = Function::from_str(&function)
            .map_err(|()| Error::runtime("function must not be empty"))?;

        Ok(self.apply(&function, parse_prefix(prefix)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_approx(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < f64::EPSILON,
            "expected {expected}, got {actual}"
        );
    }

    fn lua_with_set() -> Result<Lua> {
        let lua = Lua::new();

        let set: ValueSet = [
            ("a".into(), Value::new(1000.0)),
            ("b".into(), Value::new(3000.0)),
        ]
        .into_iter()
        .collect();

        lua.globals().set("set", set)?;
        Ok(lua)
    }

    fn lua_with_value() -> Result<Lua> {
        let lua = Lua::new();
        lua.globals()
            .set("val", Value::new_with_prefix(2.0, Prefix::Mega))?;
        Ok(lua)
    }

    async fn lua_with_client() -> Result<Lua> {
        let client = crate::clients::Client::start(Client::new())
            .await
            .map_err(Error::external)?;

        let lua = Lua::new();
        lua.globals()
            .set("sysinfo", LuaClient(std::sync::Arc::new(client)))?;
        Ok(lua)
    }

    #[test]
    fn value_set_get_function() -> Result<()> {
        let lua = lua_with_set()?;
        assert_approx(lua.load("return set:get('sum')").eval()?, 4000.0);
        Ok(())
    }

    #[test]
    fn value_set_get_function_with_prefix() -> Result<()> {
        let lua = lua_with_set()?;
        assert_approx(lua.load("return set:get('max', 'k')").eval()?, 3.0);
        Ok(())
    }

    #[test]
    fn value_set_get_named() -> Result<()> {
        let lua = lua_with_set()?;
        assert_approx(lua.load("return set:get('b')").eval()?, 3000.0);
        Ok(())
    }

    #[test]
    fn value_set_values_field() -> Result<()> {
        let lua = lua_with_set()?;
        assert_approx(lua.load("return set.values.a").eval()?, 1000.0);
        Ok(())
    }

    #[test]
    fn value_set_get_empty_errors() -> Result<()> {
        let lua = lua_with_set()?;
        assert!(lua.load("return set:get('')").eval::<f64>().is_err());
        Ok(())
    }

    #[test]
    fn value_value_field() -> Result<()> {
        let lua = lua_with_value()?;
        assert_approx(lua.load("return val.value").eval()?, 2_000_000.0);
        Ok(())
    }

    #[test]
    fn value_get_with_prefix() -> Result<()> {
        let lua = lua_with_value()?;
        assert_approx(lua.load("return val:get('k')").eval()?, 2000.0);
        Ok(())
    }

    #[test]
    fn value_get_invalid_prefix_errors() -> Result<()> {
        let lua = lua_with_value()?;
        assert!(lua.load("return val:get('nope')").eval::<f64>().is_err());
        Ok(())
    }

    #[test]
    fn refresh_type_tostring() -> Result<()> {
        let lua = Lua::new();
        lua.globals().set("kind", RefreshType::Network)?;

        let kind: String = lua.load("return tostring(kind)").eval()?;
        assert_eq!(kind, "Network");
        Ok(())
    }

    #[tokio::test]
    async fn client_memory_percent() -> Result<()> {
        let lua = lua_with_client().await?;
        let memory_percent: f64 = lua.load("return sysinfo:memory_percent().value").eval()?;
        assert!((0.0..=100.0).contains(&memory_percent));
        Ok(())
    }

    #[tokio::test]
    async fn client_cpu_percent() -> Result<()> {
        let lua = lua_with_client().await?;
        let cpu_mean: f64 = lua
            .load("return sysinfo:cpu_percent():get('mean')")
            .eval()?;
        assert!(cpu_mean >= 0.0);
        Ok(())
    }

    #[tokio::test]
    async fn client_load_average() -> Result<()> {
        let lua = lua_with_client().await?;
        let load: f64 = lua.load("return sysinfo:load_average_1().value").eval()?;
        assert!(load >= 0.0);
        Ok(())
    }

    #[tokio::test]
    async fn client_uptime() -> Result<()> {
        let lua = lua_with_client().await?;
        assert!(lua.load("return sysinfo:uptime()").eval::<String>().is_ok());
        Ok(())
    }
}
