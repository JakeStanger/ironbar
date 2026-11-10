-- ironbar is the global UserData object set in rust-land
-- proxy it here to add our own methods in lua
local proxy = ironbar
ironbar = setmetatable({}, {
    __index = function(_, key)
        return proxy[key]
    end,
})

print = ironbar.log_info

local Bar = {}
Bar.__index = Bar

--- Creates a new `Bar` wrapper type around a `Window` widget.
function Bar.new(window)
    local self = setmetatable({}, Bar)
    self.window = window
    self.container = window:get_first_child()
    return self
end

--- Gets the 'start' container for this bar.
function Bar:get_start()
    return self.container:get_first_child()
end

--- Gets the 'center' container for this bar.
function Bar:get_center()
    return self:get_start():get_next_sibling()
end

--- Gets the 'end' container for this bar.
function Bar:get_end()
    return self.container:get_last_child()
end

Position = {
    Start = 0,
    Center = 1,
    End = 2
}

--- Gets a module by name,
--- for this bar, in a given location.
---
--- One of `Position.Start`, `Position.Center`, or `Position.End`
--- must be passed.
---
--- If no match is found, `nil` is returned.
function Bar:get_module(name, position)
    local container
    if position == Position.Start then
        container = self:get_start()
    elseif position == Position.Center then
        container = self:get_center()
    elseif position == Position.End then
        container = self:get_end()
    else
        error('invalid position')
    end

    for _, child in ipairs(get_module_children(container)) do
        local module = child:get_first_child()

        if module.name == name then
            return module
        end
    end

    return nil
end

--- Get a table of handles for all bars.
function ironbar.get_bars()
    local bars = {}

    local windows = Gtk.Window.list_toplevels()
    for _, window in ipairs(windows) do
        table.insert(bars, Bar.new(window))
    end

    return bars
end

--- Gets a handle to a bar by name.
---
--- If no match is found, `nil` is returned.
function ironbar.get_bar(name)
    local windows = Gtk.Window.list_toplevels()

    for _, window in ipairs(windows) do
        if window.name == name then
            return Bar.new(window)
        end
    end

    return nil
end

--

--- Gets a table of all direct (non-recursive) children
--- for this widget.
function get_module_children(widget)
    local children = {}

    local child = widget:get_first_child()
    while child != nil do
        table.insert(children, child)
        child = child:get_next_sibling()
    end

    return children
end