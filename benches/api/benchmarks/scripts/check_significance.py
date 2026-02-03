#!/usr/bin/env python3
"""Check statistical significance using Welch's t-test."""

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


def check_significance(
    before: List[float],
    after: List[float],
    alpha: float = 0.05,
    direction: str = "higher"
) -> Dict[str, Any]:
    """Run Welch's t-test and return significance results.

    Args:
        before: Before samples
        after: After samples
        alpha: Significance level (default: 0.05)
        direction: 'higher' for improvement with increase, 'lower' for improvement with decrease
    """
    try:
        import numpy as np
        from scipy import stats
    except ImportError as e:
        raise ImportError(
            "Required libraries not found. Please install: pip install numpy scipy"
        ) from e

    if len(before) < 3 or len(after) < 3:
        raise ValueError("At least 3 samples are required for both before and after")

    # Validate input samples
    for i, sample in enumerate(before):
        if not math.isfinite(sample):
            raise ValueError(f"Before sample {i} is NaN or Infinity: {sample}")
    for i, sample in enumerate(after):
        if not math.isfinite(sample):
            raise ValueError(f"After sample {i} is NaN or Infinity: {sample}")

    if direction not in ["higher", "lower"]:
        raise ValueError(f"Invalid direction: {direction}. Must be 'higher' or 'lower'")

    t_statistic, p_value = stats.ttest_ind(after, before, equal_var=False)

    before_mean = float(np.mean(before))
    after_mean = float(np.mean(after))

    if before_mean == 0:
        # Cannot calculate relative improvement when before_mean is 0
        # Use absolute difference as a fallback
        improvement = float(after_mean)
    else:
        improvement = float((after_mean - before_mean) / before_mean)

    # Adjust improvement direction based on metric type
    if direction == "lower":
        improvement = -improvement  # Invert for metrics where lower is better

    if abs(improvement) < 0.01:
        improvement_direction = "no_change"
    elif improvement > 0:
        improvement_direction = "improvement"
    else:
        improvement_direction = "regression"

    return {
        "significant": bool(float(p_value) < alpha),
        "p_value": safe_float(p_value),
        "t_statistic": safe_float(t_statistic),
        "before_mean": safe_float(before_mean),
        "after_mean": safe_float(after_mean),
        "improvement": safe_float(improvement),
        "improvement_direction": improvement_direction,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Check significance using Welch's t-test")
    parser.add_argument("--before", type=float, nargs="+", required=False, help="Before samples")
    parser.add_argument("--after", type=float, nargs="+", required=False, help="After samples")
    parser.add_argument("--alpha", type=float, default=0.05, help="Significance level")
    parser.add_argument("--direction", type=str, default="higher", choices=["higher", "lower"],
                        help="Improvement direction: 'higher' for RPS (default), 'lower' for latency")
    parser.add_argument("--test", action="store_true", help="Run unit tests")

    args = parser.parse_args()

    if args.test:
        return run_tests()

    if not args.before or not args.after:
        print("Error: --before and --after are required", file=sys.stderr)
        return 1

    if len(args.before) < 3 or len(args.after) < 3:
        print("Error: At least 3 samples are required for both before and after", file=sys.stderr)
        return 1

    try:
        result = check_significance(args.before, args.after, args.alpha, args.direction)
        print(json.dumps(result, indent=2, allow_nan=False))
        return 0
    except (ValueError, ImportError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


def run_tests() -> int:
    print("Running unit tests...")
    passed = failed = 0

    test_cases = [
        ([100.0, 101.0, 99.0, 100.5, 100.2], [110.0, 111.0, 109.0, 110.5, 110.2], True, "improvement", "Significant improvement"),
        ([100.0, 101.0, 99.0, 100.5, 100.2], [100.5, 101.5, 99.5, 101.0, 100.7], False, None, "No significant change"),
        ([100.0, 101.0, 99.0, 100.5, 100.2], [90.0, 91.0, 89.0, 90.5, 90.2], True, "regression", "Regression detected"),
    ]

    for before, after, expect_sig, expect_dir, description in test_cases:
        try:
            result = check_significance(before, after)
            if result["significant"] == expect_sig:
                if expect_dir is None or result["improvement_direction"] == expect_dir:
                    print(f"✓ {description} (p={result['p_value']:.4f})")
                    passed += 1
                else:
                    print(f"✗ {description}: wrong direction")
                    failed += 1
            else:
                print(f"✗ {description}: wrong significance (p={result['p_value']:.4f})")
                failed += 1
        except Exception as e:
            print(f"✗ {description}: {e}")
            failed += 1

    try:
        check_significance([100.0, 101.0], [110.0, 111.0, 112.0])
        print("✗ Should raise ValueError for insufficient samples")
        failed += 1
    except ValueError:
        print("✓ Raises ValueError for insufficient samples")
        passed += 1

    # Test --direction lower
    latency_before = [100.0, 101.0, 99.0]
    latency_after = [90.0, 91.0, 89.0]
    lower_result = check_significance(latency_before, latency_after, direction="lower")
    if lower_result["improvement_direction"] == "improvement":
        print(f"✓ Direction 'lower' detected improvement ({lower_result['improvement']:.2f})")
        passed += 1
    else:
        print(f"✗ Direction 'lower' should detect improvement")
        failed += 1

    # Test NaN/Infinity rejection
    try:
        check_significance([100.0, float('nan'), 99.0], [110.0, 111.0, 109.0])
        print("✗ NaN input should raise ValueError")
        failed += 1
    except ValueError:
        print("✓ NaN input raises ValueError")
        passed += 1

    # Test before_mean == 0
    zero_before = [0.0, 0.0, 0.0]
    zero_after = [10.0, 11.0, 9.0]
    zero_result = check_significance(zero_before, zero_after)
    if zero_result["before_mean"] == 0.0 and zero_result["improvement"] > 0:
        print(f"✓ Before mean == 0 handled correctly (improvement={zero_result['improvement']:.2f})")
        passed += 1
    else:
        print(f"✗ Before mean == 0 not handled correctly")
        failed += 1

    exact_before = [100.0, 100.0, 100.0]
    exact_after = [110.0, 110.0, 110.0]
    exact_result = check_significance(exact_before, exact_after)

    if exact_result["before_mean"] == 100.0 and exact_result["after_mean"] == 110.0:
        print("✓ Mean values correct")
        passed += 1
    else:
        print("✗ Mean values incorrect")
        failed += 1

    if abs(exact_result["improvement"] - 0.1) < 0.01:
        print(f"✓ Improvement rate correct ({exact_result['improvement']:.2f})")
        passed += 1
    else:
        print(f"✗ Improvement rate incorrect ({exact_result['improvement']:.2f})")
        failed += 1

    no_change_result = check_significance([100.0, 100.0, 100.0], [100.5, 100.5, 100.5])
    if no_change_result["improvement_direction"] == "no_change":
        print("✓ No change detected for small variation")
        passed += 1
    else:
        print("✗ Should detect no change for small variation")
        failed += 1

    try:
        json_str = json.dumps(exact_result)
        parsed = json.loads(json_str)
        if parsed["p_value"] == exact_result["p_value"]:
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
