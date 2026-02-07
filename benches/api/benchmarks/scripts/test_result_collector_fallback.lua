#!/usr/bin/env lua
-- Test: result_collector の http_status フォールバックを検証

-- このテストはマニュアル確認用
-- 期待: http_status_total == 0 の場合、M.status_counts にフォールバックする

print("[TEST] result_collector should fallback to M.status_counts when http_status_total == 0")
print("")
print("Manual verification required:")
print("1. Check benches/api/benchmarks/scripts/result_collector.lua L461-469")
print("2. Verify that http_status_total == 0 時に M.status_counts にフォールバックする")
print("")
print("Expected code (after L469):")
print("  elseif next(M.status_counts) then")
print("      -- error_tracker aggregated is empty, fallback to thread-local status_counts")
print("      M.results.http_status = {}")
print("      for status, count in pairs(M.status_counts) do")
print("          if tonumber(status) and type(count) == 'number' then")
print("              M.results.http_status[tostring(status)] = count")
print("          end")
print("      end")
print("  end")
print("")
print("❌ FAIL: Manual check pending - fallback not yet added")
os.exit(1)
