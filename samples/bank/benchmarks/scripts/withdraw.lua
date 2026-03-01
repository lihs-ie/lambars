-- Withdraw benchmark script for wrk
-- Tests the /accounts/{id}/withdraw endpoint
--
-- Usage:
--   wrk -t4 -c100 -d30s -s withdraw.lua http://localhost:8081 -- <account_id> [eff]
--
-- Arguments:
--   account_id: Pre-created account ID with sufficient balance (required)
--   eff: Use "eff" for eff_async! version endpoint (optional)

local account_id = nil
local use_eff = false
local counter = 0

function init(args)
    if args[1] then
        account_id = args[1]
    end
    if args[2] == "eff" then
        use_eff = true
    end

    if not account_id then
        io.write("Error: account_id is required\n")
        io.write("Usage: wrk ... -s withdraw.lua http://localhost:8081 -- <account_id> [eff]\n")
        os.exit(1)
    end
end

function request()
    counter = counter + 1
    local path = use_eff
        and string.format("/accounts/%s/withdraw-eff", account_id)
        or string.format("/accounts/%s/withdraw", account_id)

    -- Withdraw small amount to avoid insufficient funds
    local idempotency_key = string.format("bench-withdraw-%d-%d", os.time(), counter)
    local body = string.format('{"amount": {"amount": "1", "currency": "JPY"}, "idempotency_key": "%s"}', idempotency_key)

    return wrk.format("POST", path, {
        ["Content-Type"] = "application/json"
    }, body)
end

function done(summary, latency, requests)
    local endpoint = use_eff and "withdraw-eff" or "withdraw"
    io.write("------------------------------\n")
    io.write(string.format("Withdraw Benchmark Results (%s)\n", endpoint))
    io.write("------------------------------\n")
    io.write(string.format("Account ID: %s\n", account_id))
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
