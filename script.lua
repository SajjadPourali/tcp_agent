function data2str(data)
    local str = ""
    for k, v in pairs(data) do
        str = str .. string.char(v)
    end
    return str
end

function str2data(str)
    local table = {}
    for i = 1, #str do
        table[i] = string.byte(str:sub(i, i))
    end
    return table
end

while (true) do
    local buffer = recive(2)
    if (buffer["status"] == outgoing) then
        -- send(outgoing,str2data(string.upper(data2str(buffer["data"]))))
        send(outgoing, buffer["data"])
    elseif (buffer["status"] == incomming) then
        -- send(incomming,str2data(string.upper(data2str(buffer["data"]))))
        send(incomming, buffer["data"])
    else
        break
    end
end
