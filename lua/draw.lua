function(id, ptr)
    local cr = __lgi_core.record.new(cairo.Context, ptr)
    _G['__draw_' .. id](cr)
end