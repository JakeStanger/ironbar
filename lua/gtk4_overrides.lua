-- GTK4-compatible replacements for lgi's built-in `Gtk` and `Gdk` overrides.
--
-- lgi 0.9.x ships overrides that target GTK3 only
-- and so raise errors when loading the 4.0 namespaces.
-- lgi loads overrides via `require`, so registering these in `package.preload`
-- takes precedence over the stock files.

local function version_check(ns)
    if ns._version ~= "4.0" then
        return ("ironbar uses GTK4; %s %s cannot be loaded"):format(ns._name, ns._version)
    end
end

package.preload["lgi.override.Gdk"] = function()
    local lgi = require("lgi")
    local Gdk = lgi.Gdk

    local err = version_check(Gdk)
    if err then
        return err
    end

    -- Easier-to-use Gdk.RGBA.parse() override.
    if Gdk.RGBA then
        local parse = Gdk.RGBA.parse
        function Gdk.RGBA.parse(arg1, arg2)
            if Gdk.RGBA:is_type_of(arg1) then
                -- Standard member method.
                return parse(arg1, arg2)
            else
                -- Static constructor variant.
                local rgba = Gdk.RGBA()
                return parse(rgba, arg1) and rgba or nil
            end
        end
    end
end

package.preload["lgi.override.Gtk"] = function()
    local lgi = require("lgi")
    local Gtk = lgi.Gtk
    local Gdk = lgi.Gdk

    local err = version_check(Gtk)
    if err then
        return err
    end

    -- GTK is already initialised by the running application,
    -- so unlike the stock override, `Gtk.init_check()` is not called here.

    Gtk.Allocation = Gdk.Rectangle

    Gtk.Widget._attribute = {
        width = {
            get = Gtk.Widget.get_width,
            set = function(self, width)
                self.width_request = width
            end,
        },
        height = {
            get = Gtk.Widget.get_height,
            set = function(self, height)
                self.height_request = height
            end,
        },
    }
end
