> [!WARNING]
> The Lua API is still in development 
> and subject to frequent breaking changes.

Ironbar includes a Lua API which can be used to extend functionality 
and write custom scripts that run entirely in-process.

The engine is started automatically with Ironbar.
If an `init.lua` file exists in the config directory, 
it will be loaded and executed automatically.

The full lua `stdlib` is available, and you can load in additional system packages as desired.

Ironbar's own API can be accessed via the `ironbar` global.

## API

### `ironbar:config_dir`

Absolute path to the configuration directory. 

This can be used for relative file imports, e.g.:

```lua
 local_module = dofile(ironbar.config_dir .. "local_mod.lua")`
 ```

### `ironbar:log_*(msg)`

Writes a string message to Ironbar's main logger.

The following are available:

- `ironbar:log_debug(msg)`
- `ironbar:log_info(msg)`
- `ironbar:log_warn(msg)`
- `ironbar:log_error(msg)`

### `ironbar:unixtime()`

Returns high-resolution unixtime (stdlib only offers second-resolution). 

Will return a table:

- `secs`: Seconds since unix-epoch with fractions
- `subsec_millis`: Sub-second milliseconds as integer
- `subsec_micros`: Sub-second microseconds as integer

### `ironbar:var_get(key)`

Gets an ironvar. 
This supports namespaced variables using a `.` separator.

```lua
memory_free = ironbar:var_get("sysinfo.memory_free")
```

### `ironbar:var_list(namespace)`

Get all variables of a namespace as table (non-recursive).

```lua
memory_free = ironbar:var_list("sysinfo")["memory_free"]
```

### `ironbar:var_set(key, value)`

Sets an ironvar. 
This does **not** support writing into namespaces.

```lua
ironbar:var_set("my_variable", "hello world")
```

### `ironbar:spawn(cb)`

Spawns a coroutine and returns its handle.
The coroutine is spawned onto a Tokio task internally.

> [!Tip]
> Do not use the Lua coroutine API directly, as tasks will not run.
> Use `ironban:spawn` instead to ensure they are placed on the runtime.

```lua
ironbar:spawn(function()
    -- do something asynchronously
end)
```

### `ironbar:sleep(ms)`

Pauses the coroutine for the specified number of milliseconds.

> [!WARNING]
> Do not call this method outside a coroutine spawned by `ironbar:spawn`.

```lua
ironbar:spawn(function()
    -- snip
    ironbar:sleep(1000)
    -- snip
end)
```

The method returns a handle with two methods:

- `<handle>:abort()` - Aborts the task.
- `<handle>:is_finished()` - Gets if this task has completed.

### `ironbar.clients`

Provides access to ironbar's system clients.

Each client is lazy-loaded on first access. 
Their instances are shared with the main bar modules,
although the associated modules do not need to be present to use the client.

Currently supported clients:

- `ironbar.clients.notifications`
- `ironbar.clients.upower`
- `ironbar.clients.power_profiles`
- `ironbar.clients.bluetooth`
- `ironbar.clients.libinput(seat)`

> [!NOTE]
> The full client set is currently in the process of being refactored,
> so support will steadily grow in the near future.

Each client has two methods associated with it:

#### `<client>:state()`

Returns the current state. This is async, so must be called inside a spawned task ([`ironbar:spawn`](#ironbarspawncb)).

#### `<client>:subscribe(fn)`

Calls `fn(event)` on each client event. 

```lua
local kb = ironbar.clients.libinput("seat0")

kb:subscribe(function(ev)
	ironbar.log_info("key: " .. ev.key .. " | state: " .. tostring(ev.state))
end)
```

The method returns a handle with an `unsubscribe()` method,
which can be used to stop receiving events.