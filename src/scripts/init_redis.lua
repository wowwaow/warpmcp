-- Redis initialization script for MCP knowledge store
-- This script initializes the required Redis modules and creates the search index

-- Check if required modules are loaded
local function check_module(name)
    local modules = redis.call('MODULE', 'LIST')
    for _, module in ipairs(modules) do
        if module[2] == name then
            return true
        end
    end
    return false
end

-- Ensure RediSearch module is loaded
if not check_module('search') then
    return {
        err = "RediSearch module not loaded. Please install using: MODULE INSTALL redisearch"
    }
end

-- Ensure RedisJSON module is loaded
if not check_module('ReJSON') then
    return {
        err = "RedisJSON module not loaded. Please install using: MODULE INSTALL rejson"
    }
end

-- Drop existing index if it exists
redis.call('FT.DROPINDEX', 'knowledge-idx', 'DD')

-- Create search index with proper schema
return redis.call(
    'FT.CREATE', 'knowledge-idx',
    'ON', 'JSON',
    'PREFIX', '1', 'knowledge:',
    'LANGUAGE', 'english',
    'LANGUAGE_FIELD', 'language',
    'SCORE', '_score',
    'SCORE_FIELD', '@score',
    'SCHEMA',
    -- Text fields with weights and features
    '$.content', 'AS', 'content', 'TEXT', 'WEIGHT', '2.0', 'PHONETIC', 'dm:en',
    '$.key', 'AS', 'key', 'TEXT', 'WEIGHT', '1.5',
    '$.tags.*', 'AS', 'tags', 'TAG', 'SORTABLE',
    '$.category', 'AS', 'category', 'TAG', 'SORTABLE',
    '$.agent_id', 'AS', 'agent_id', 'TAG', 'SORTABLE',
    '$.created_at', 'AS', 'created_at', 'NUMERIC', 'SORTABLE',
    '$.access_count', 'AS', 'access_count', 'NUMERIC', 'SORTABLE',
    '$.metadata.*', 'AS', 'metadata', 'TEXT', 'WEIGHT', '1.0'
)
