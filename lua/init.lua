-- Workaround for lgi incompatibility with Lua 5.5:
-- for-loop variables are read-only since Lua 5.5, but lgi's
-- component.lua reassigns the `en` loop variable. Patch the
-- module before loading it through the normal searcher.
do
    -- Find lgi.component on the module path
    local file = package.searchpath('lgi.component', package.path)
    if file then
        local f = assert(io.open(file, 'rb'))
        local src = assert(f:read('a'))
        f:close()

        -- Only fix the assignment inside the `for en, idx in pairs(index)`
        -- loop, anchored on the preceding line which only occurs there.
        -- The `while`-loop assignment (line ~60) must stay intact.
        src = src:gsub(
            '(val = xvalue%(children%[idx%]%)\n%s-)en = not xform_name_reverse and en or xform_name_reverse%(en%)',
            '%1local en = not xform_name_reverse and en or xform_name_reverse(en)'
        )

        package.preload['lgi.component'] = load(src, '@' .. file)
    end
end

local lgi = require('lgi')
cairo = lgi.cairo

__lgi_core = require('lgi.core')