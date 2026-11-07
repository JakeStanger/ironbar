use super::{BatteryLevel, BatteryState, BatteryType};
use crate::clients::lua::into_lua_debug;

into_lua_debug!(BatteryLevel, BatteryState, BatteryType);
