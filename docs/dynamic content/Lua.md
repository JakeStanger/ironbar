> [!WARNING]
> The Lua API is still in development 
> and subject to frequent breaking changes.

Ironbar includes a Lua API which can be used to extend functionality 
and write custom scripts that run entirely in-process.

The engine is started automatically with Ironbar.
If an `init.lua` file exists in the config directory, 
it will be loaded and executed automatically on startup.

The full lua `stdlib` is available, and you can load in additional system packages as desired.

Ironbar's own API can be accessed via the `ironbar` global.

> [!TIP]
> There is a mix of functions and methods throughout the API.
> Pay close attention to whether a `:` or `.` is used.

## GTK

> [!NOTE]
> GTK bindings are provided by [LGI](https://github.com/lgi-devs/lgi),
> which must be installed for LuaJIT.
> See [compiling](compiling) for information.

> [!WARNING]
> Access to the GTK API provides absolutely no handrails, no guarantees, 
> and no mechanisms that will prevent you from breaking Ironbar.
> Have fun and get creative, 
> but be aware there is limited support available for any issues you may encounter.

All of GTK, Glib and Cario are available via the respective `Gtk`, `Glib` and `cairo` globals.
These are direct bindings to the C API, 
meaning they provide **full access** to their APIs and the current Ironbar widget tree,
allowing for very powerful low-level manipulation.

```lua
for _, window in ipairs(Gtk.Window.list_toplevels()) do
    ironbar.log_info(window.name)
end
```

A higher-level API is also provided, which is documented below.

## API

### `ironbar.config_dir`

Absolute path to the configuration directory. 

This can be used for relative file imports, e.g.:

```lua
 local_module = dofile(ironbar.config_dir .. "local_mod.lua")`
 ```

### `ironbar.log_*(msg)`

Writes a string message to Ironbar's main logger.

The following are available:

- `ironbar.log_debug(msg)`
- `ironbar.log_info(msg)`
- `ironbar.log_warn(msg)`
- `ironbar.log_error(msg)`

> [!TIP]
> The built in `print` function is aliased to `ironbar.log_info`.

### `ironbar.unixtime()`

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

### `ironbar.spawn(cb)`

Spawns a coroutine and returns its handle.
The coroutine is spawned onto a Tokio task internally.

> [!WARNING]
> Do not use the Lua coroutine API directly, as tasks will not run.
> Use `ironban:spawn` instead to ensure they are placed on the runtime.

```lua
ironbar.spawn(function()
    -- do something asynchronously
end)
```

### `ironbar.sleep(ms)`

Pauses the coroutine for the specified number of milliseconds.

> [!WARNING]
> This is an async function. 
> Do not call this method outside a coroutine spawned by [`ironbar:spawn`](#ironbarspawncb).

```lua
ironbar.spawn(function()
    -- snip
    ironbar.sleep(1000)
    -- snip
end)
```

The method returns a handle with two methods:

- `<handle>:abort()` - Aborts the task.
- `<handle>:is_finished()` - Gets if this task has completed.

### `ironbar:get_bars()`

Get a table of handles for all bars.

```lua
local bars = ironbar.get_bars()
```

### `ironbar:get_bar(name)`

Gets a handle to s bar by name.

If no match is found, `nil` is returned.

```lua
local main_bar = ironbar.get_bar("DP-1")
```

### `<Bar>:get_start()`

Gets the 'start' container (`Gtk.Box`) for this bar.

> [!NOTE]
> The container's children will be a series of `Gtk.Releaver` widgets,
> which wrap each module's root widget.

```lua
local start_widgets = my_bar:get_start()
```

### `<Bar>:get_center()`

Gets the 'center' container (`Gtk.Box`) for this bar.

> [!NOTE]
> The container's children will be a series of `Gtk.Releaver` widgets,
> which wrap each module's root widget.

```lua
local center_widgets = my_bar:get_center()
```

### `<Bar>:get_end()`

Gets the 'end' container (`Gtk.Box`) for this bar.

> [!NOTE]
> The container's children will be a series of `Gtk.Releaver` widgets,
> which wrap each module's root widget.

```lua
local end_widgets = my_bar:get_end()
```

### `<Bar>:get_module(name, position)`

Gets a module by name, for this bar, in a given location.

One of `Position.Start`, `Position.Center`, or `Position.End`
must be passed.

If a match is found, the root widget is returned.
If no match is found, `nil` is returned.

```lua
local my_module = my_bar:get_module("my_module", Position.Center)
my_module:set_visible(false)
```

### `get_module_children()`

Gets a table of all direct (non-recursive) children for this widget.

```lua
local children = get_module_children(my_widget)

for _, child in ipairs(children) do
    -- do something with child
end
```

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
- `ironbar.clients.sysinfo`

> [!NOTE]
> The full client set is currently in the process of being refactored,
> so support will steadily grow in the near future.

Each client has two methods associated with it:

#### `<client>:state()`

Returns the current state. 

> [!WARNING]
> This is an async function.
> Do not call this method outside a coroutine spawned by [`ironbar:spawn`](#ironbarspawncb).

#### `<client>:subscribe(fn)`

Calls `fn(event)` on each client event. 
The callback is automatically spawned onto an async task.

```lua
local kb = ironbar.clients.libinput("seat0")

kb:subscribe(function(ev)
	ironbar.log_info("key: " .. ev.key .. " | state: " .. tostring(ev.state))
end)
```

The method returns a handle with an `unsubscribe()` method,
which can be used to stop receiving events.

Client-specific documentation can be found on the relevant module page.