-- Redis diagnostic script
local function debug(msg)
    redis.log(redis.LOG_WARNING, msg)
end

local function check_module(name)
    local modules = redis.call('MODULE', 'LIST')
    for _, module in ipairs(modules) do
        if module[2] == name then
            return true
        end
    end
    return false
end

local function check_index(name)
    local ok, res = pcall(redis.call, 'FT._LIST')
    if not ok then
        return {err = "Failed to list indices: " .. res}
    end
    for _, index in ipairs(res) do
        if index == name then
            return true
        end
    end
    return false
end

local function get_index_info(name)
    local ok, res = pcall(redis.call, 'FT.INFO', name)
    if not ok then
        return {err = "Failed to get index info: " .. res}
    end
    return res
end

-- Main diagnostic flow
local diagnostics = {}

-- Check Redis version
diagnostics.redis_version = redis.call('INFO', 'server')['redis_version']

-- Check modules
diagnostics.has_search = check_module('search')
diagnostics.has_json = check_module('ReJSON')

-- Check index
diagnostics.index_exists = check_index('knowledge-idx')
if diagnostics.index_exists then
    diagnostics.index_info = get_index_info('knowledge-idx')
end

-- Return diagnostic info
return cjson.encode(diagnostics)
