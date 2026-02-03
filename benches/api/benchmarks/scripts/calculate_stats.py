#!/usr/bin/env python3
"""Calculate statistics from sample values."""

import sys
import json
import argparse
import math
from typing import List, Dict, Any, Optional


def safe_float(value: float) -> Optional[float]:
    """Convert to float or None if NaN/Infinity."""
    if not math.isfinite(value):
        return None
    return float(value)


def calculate_stats(samples: List[float]) -> Dict[str, Any]:
    """Calculate statistics and return as dict with 95% CI."""
    try:
        import numpy as np
        from scipy import stats
    except ImportError as e:
        raise ImportError(
            "Required libraries not found. Please install: pip install numpy scipy"
        ) from e

    if len(samples) < 3:
        raise ValueError("At least 3 samples are required")

    # Validate input samples
    for i, sample in enumerate(samples):
        if not math.isfinite(sample):
            raise ValueError(f"Sample {i} is NaN or Infinity: {sample}")

    mean = float(np.mean(samples))
    stddev = float(np.std(samples, ddof=1))
    stderr = float(stats.sem(samples))
    min_val = float(np.min(samples))
    max_val = float(np.max(samples))
    n_samples = len(samples)

    confidence = 0.95
    df = n_samples - 1
    t_critical = stats.t.ppf((1 + confidence) / 2, df)
    margin_of_error = t_critical * stderr

    ci_lower = mean - margin_of_error
    ci_upper = mean + margin_of_error
    ci_width = 2 * margin_of_error

    # When mean is near zero, use absolute CI width instead of ratio
    # This handles cases like error rates where mean can be 0
    epsilon = 1e-6
    if abs(mean) < epsilon:
        ci_width_ratio = float(ci_width)
    else:
        ci_width_ratio = float(ci_width / abs(mean))

    # Validate output values
    return {
        "mean": safe_float(mean),
        "stddev": safe_float(stddev),
        "stderr": safe_float(stderr),
        "min": safe_float(min_val),
        "max": safe_float(max_val),
        "samples": n_samples,
        "confidence_interval_95": [safe_float(ci_lower), safe_float(ci_upper)],
        "ci_width_ratio": safe_float(ci_width_ratio),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Calculate statistics from sample values")
    parser.add_argument("--test", action="store_true", help="Run unit tests")
    parser.add_argument("samples", type=float, nargs="*", help="Sample values")

    args = parser.parse_args()

    if args.test:
        return run_tests()

    if len(args.samples) < 3:
        print("Error: At least 3 samples are required", file=sys.stderr)
        return 1

    try:
        stats_result = calculate_stats(args.samples)
        print(json.dumps(stats_result, indent=2, allow_nan=False))
        return 0
    except (ValueError, ImportError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


def run_tests() -> int:
    print("Running unit tests...")
    passed = failed = 0

    samples = [100.0, 102.0, 98.0, 101.0, 99.0]
    result = calculate_stats(samples)

    checks = [
        (99.5 <= result["mean"] <= 100.5, f"Mean correct ({result['mean']:.2f})"),
        (1.0 <= result["stddev"] <= 2.0, f"Stddev reasonable ({result['stddev']:.2f})"),
        (result["min"] == 98.0 and result["max"] == 102.0, "Min/Max correct"),
        (result["samples"] == 5, "Sample count correct"),
    ]

    for condition, description in checks:
        if condition:
            print(f"✓ {description}")
            passed += 1
        else:
            print(f"✗ {description}")
            failed += 1

    identical_samples = [100.0] * 5
    identical_result = calculate_stats(identical_samples)

    if identical_result["stddev"] == 0.0:
        print("✓ Stddev is 0 for identical samples")
        passed += 1
    else:
        print("✗ Stddev should be 0 for identical samples")
        failed += 1

    if identical_result["ci_width_ratio"] == 0.0:
        print("✓ CI width ratio is 0 for identical samples")
        passed += 1
    else:
        print("✗ CI width ratio should be 0 for identical samples")
        failed += 1

    try:
        calculate_stats([100.0, 101.0])
        print("✗ Should raise ValueError for insufficient samples")
        failed += 1
    except ValueError:
        print("✓ Raises ValueError for insufficient samples")
        passed += 1

    # Test NaN/Infinity rejection
    try:
        calculate_stats([100.0, float('inf'), 102.0])
        print("✗ Infinity input should raise ValueError")
        failed += 1
    except ValueError:
        print("✓ Infinity input raises ValueError")
        passed += 1

    ci_samples = [95.0, 100.0, 105.0]
    ci_result = calculate_stats(ci_samples)
    ci_lower, ci_upper = ci_result["confidence_interval_95"]
    if ci_lower < ci_result["mean"] < ci_upper:
        print(f"✓ CI bounds correct ({ci_lower:.2f}, {ci_upper:.2f})")
        passed += 1
    else:
        print(f"✗ CI bounds incorrect ({ci_lower:.2f}, {ci_upper:.2f})")
        failed += 1

    import numpy as np
    expected_stderr = np.std(samples, ddof=1) / np.sqrt(len(samples))
    if abs(result["stderr"] - expected_stderr) < 0.01:
        print(f"✓ Stderr correct ({result['stderr']:.4f})")
        passed += 1
    else:
        print(f"✗ Stderr incorrect ({result['stderr']:.4f}, expected {expected_stderr:.4f})")
        failed += 1

    try:
        json_str = json.dumps(result)
        parsed = json.loads(json_str)
        if parsed["mean"] == result["mean"]:
            print("✓ JSON serialization works")
            passed += 1
        else:
            print("✗ JSON serialization failed")
            failed += 1
    except Exception as e:
        print(f"✗ JSON serialization: {e}")
        failed += 1

    print(f"\n{passed} passed, {failed} failed")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
