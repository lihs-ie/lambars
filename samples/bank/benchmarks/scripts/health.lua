-- Health check benchmark script for wrk
-- Tests the /health endpoint as a baseline

wrk.method = "GET"
wrk.headers["Content-Type"] = "application/json"

function request()
    return wrk.format("GET", "/health")
end

function done(summary, latency, requests)
    io.write("------------------------------\n")
    io.write("Health Check Benchmark Results\n")
    io.write("------------------------------\n")
    io.write(string.format("Requests/sec: %.2f\n", summary.requests / summary.duration * 1000000))
    io.write(string.format("Transfer/sec: %.2f KB\n", summary.bytes / summary.duration * 1000000 / 1024))
    io.write(string.format("Avg Latency:  %.2f ms\n", latency.mean / 1000))
    io.write(string.format("Max Latency:  %.2f ms\n", latency.max / 1000))
    io.write(string.format("Latency Stdev: %.2f ms\n", latency.stdev / 1000))
    io.write(string.format("Total Requests: %d\n", summary.requests))
    io.write(string.format("Socket Errors: connect=%d, read=%d, write=%d, timeout=%d\n",
        summary.errors.connect, summary.errors.read, summary.errors.write, summary.errors.timeout))

    -- Percentile latencies
    io.write("\nLatency Distribution:\n")
    for _, p in pairs({50, 75, 90, 99}) do
        n = latency:percentile(p)
        io.write(string.format("  %d%%: %.2f ms\n", p, n / 1000))
    end
end
