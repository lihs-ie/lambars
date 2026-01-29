-- Transfer benchmark script for wrk
-- Tests the /accounts/{id}/transfer endpoint
--
-- Usage:
--   wrk -t4 -c100 -d30s -s transfer.lua http://localhost:8081 -- <from_account_id> <to_account_id>
--
-- Arguments:
--   from_account_id: Source account ID with sufficient balance (required)
--   to_account_id: Destination account ID (required)

local from_account_id = nil
local to_account_id = nil
local counter = 0

function init(args)
    if args[1] then
        from_account_id = args[1]
    end
    if args[2] then
        to_account_id = args[2]
    end

    if not from_account_id or not to_account_id then
        io.write("Error: both from_account_id and to_account_id are required\n")
        io.write("Usage: wrk ... -s transfer.lua http://localhost:8081 -- <from_id> <to_id>\n")
        os.exit(1)
    end
end

function request()
    counter = counter + 1
    local path = string.format("/accounts/%s/transfer", from_account_id)

    -- Transfer small amount to avoid insufficient funds
    local idempotency_key = string.format("bench-transfer-%d-%d", os.time(), counter)
    local body = string.format('{"to_account_id": "%s", "amount": {"amount": "1", "currency": "JPY"}, "idempotency_key": "%s"}', to_account_id, idempotency_key)

    return wrk.format("POST", path, {
        ["Content-Type"] = "application/json"
    }, body)
end

function done(summary, latency, requests)
    io.write("------------------------------\n")
    io.write("Transfer Benchmark Results\n")
    io.write("------------------------------\n")
    io.write(string.format("From Account: %s\n", from_account_id))
    io.write(string.format("To Account: %s\n", to_account_id))
    io.write(string.format("Requests/sec: %.2f\n", summary.requests / summary.duration * 1000000))
    io.write(string.format("Transfer/sec: %.2f KB\n", summary.bytes / summary.duration * 1000000 / 1024))
    io.write(string.format("Avg Latency:  %.2f ms\n", latency.mean / 1000))
    io.write(string.format("Max Latency:  %.2f ms\n", latency.max / 1000))
    io.write(string.format("Latency Stdev: %.2f ms\n", latency.stdev / 1000))
    io.write(string.format("Total Requests: %d\n", summary.requests))
    io.write(string.format("Socket Errors: connect=%d, read=%d, write=%d, timeout=%d\n",
        summary.errors.connect, summary.errors.read, summary.errors.write, summary.errors.timeout))

    io.write("\nLatency Distribution:\n")
    for _, p in pairs({50, 75, 90, 99}) do
        n = latency:percentile(p)
        io.write(string.format("  %d%%: %.2f ms\n", p, n / 1000))
    end
end
