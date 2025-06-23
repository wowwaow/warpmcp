local keys = redis.call('KEYS', 'knowledge:*')
for _, key in ipairs(keys) do
    local data = redis.call('JSON.GET', key)
    local obj = cjson.decode(data)
    if type(obj) == 'table' and #obj == 1 then
        redis.call('JSON.SET', key, '$', cjson.encode(obj[1]))
    end
end
