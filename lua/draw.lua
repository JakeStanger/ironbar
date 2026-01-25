function(id, ptr, width, height)
    local cr = __lgi_core.record.new(cairo.Context, ptr)
    _G['__draw_' .. id](cr, width, height)
end