#!/usr/bin/env lua
-- Test: common.lua の create_done_handler が finalize_benchmark を呼ぶことを検証

-- このテストはマニュアル確認用
-- 期待: create_done_handler() が返す関数は print_summary() と finalize_benchmark() の両方を呼ぶ

print("[TEST] common.create_done_handler should call finalize_benchmark")
print("")
print("Manual verification required:")
print("1. Check benches/api/benchmarks/scripts/common.lua L254-258")
print("2. Verify that create_done_handler calls M.finalize_benchmark(summary, latency, requests)")
print("")
print("Expected code:")
print("  function M.create_done_handler(script_name)")
print("      return function(summary, latency, requests)")
print("          M.print_summary(script_name, summary)")
print("          M.finalize_benchmark(summary, latency, requests)")
print("      end")
print("  end")
print("")
print("❌ FAIL: Manual check pending - finalize_benchmark not yet added")
os.exit(1)
