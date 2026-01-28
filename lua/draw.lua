function(draw_function, ptr, width, height)
    local cr = __lgi_core.record.new(cairo.Context, ptr)
    draw_function(cr, width, height)
end