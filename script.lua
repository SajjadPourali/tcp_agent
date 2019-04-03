while (true) do
    local buffer = recive(2048)
    -- print(buffer["no"])
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
