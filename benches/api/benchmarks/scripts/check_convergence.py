#!/usr/bin/env python3
"""Check convergence using t-distribution confidence interval."""

import sys
import argparse
import math
from typing import List, Tuple


def calculate_ci_width_ratio(samples: List[float]) -> Tuple[float, float, float]:
    """Calculate 95% CI and return (mean, ci_width, ci_width_ratio)."""
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

    mean = np.mean(samples)
    std_err = stats.sem(samples)

    # If std_err == 0, samples are identical → converged
    if std_err == 0:
        return float(mean), 0.0, 0.0

    confidence = 0.95
    df = len(samples) - 1
    t_critical = stats.t.ppf((1 + confidence) / 2, df)

    ci_width = 2 * t_critical * std_err

    # When mean is near zero, use absolute CI width instead of ratio
    # This handles cases like error rates where mean can be 0
    epsilon = 1e-6
    if abs(mean) < epsilon:
        ci_width_ratio = ci_width
    else:
        ci_width_ratio = ci_width / abs(mean)

    return float(mean), float(ci_width), float(ci_width_ratio)


def check_convergence(samples: List[float], threshold: float = 0.1) -> bool:
    """Check if CI width / mean < threshold.

    When mean is near zero (|mean| < 1e-6), uses absolute CI width instead of ratio.
    This handles cases like error rates where mean can be 0.
    """
    _, _, ci_width_ratio = calculate_ci_width_ratio(samples)
    return ci_width_ratio < threshold


def main() -> int:
    parser = argparse.ArgumentParser(description="Check convergence using t-distribution CI")
    parser.add_argument("--threshold", type=float, default=0.1,
                        help="CI width / mean threshold (uses absolute CI width when mean is near zero)")
    parser.add_argument("--test", action="store_true", help="Run unit tests")
    parser.add_argument("samples", type=float, nargs="*", help="Sample values")

    args = parser.parse_args()

    if args.test:
        return run_tests()

    if len(args.samples) < 3:
        print("Error: At least 3 samples are required", file=sys.stderr)
        return 1

    try:
        converged = check_convergence(args.samples, args.threshold)
        print("converged" if converged else "not_converged")
        return 0
    except (ValueError, ImportError) as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


def run_tests() -> int:
    print("Running unit tests...")
    passed = failed = 0

    tests = [
        (lambda: check_convergence([100.0, 101.0, 99.5, 100.5, 100.2], 0.1), True, "Small variation converges"),
        (lambda: check_convergence([100.0, 150.0, 80.0], 0.1), False, "Large variation does not converge"),
    ]

    for test_func, expected, description in tests:
        try:
            result = test_func()
            if result == expected:
                print(f"✓ {description}")
                passed += 1
            else:
                print(f"✗ {description}")
                failed += 1
        except Exception as e:
            print(f"✗ {description}: {e}")
            failed += 1

    # Test insufficient samples error
    try:
        check_convergence([100.0, 101.0], 0.1)
        print("✗ Insufficient samples raises ValueError")
        failed += 1
    except ValueError:
        print("✓ Insufficient samples raises ValueError")
        passed += 1

    # Test identical samples → converged (std_err == 0)
    try:
        result = check_convergence([100.0, 100.0, 100.0], 0.1)
        if result:
            print("✓ Identical samples converge (std_err == 0)")
            passed += 1
        else:
            print("✗ Identical samples should converge")
            failed += 1
    except Exception as e:
        print(f"✗ Identical samples: {e}")
        failed += 1

    # Test NaN/Infinity rejection
    try:
        check_convergence([100.0, float('nan'), 102.0], 0.1)
        print("✗ NaN input should raise ValueError")
        failed += 1
    except ValueError:
        print("✓ NaN input raises ValueError")
        passed += 1

    # Test mean == 0 case
    try:
        mean, _, ratio = calculate_ci_width_ratio([0.0, 0.0, 0.0])
        if mean == 0.0 and ratio == 0.0:
            print("✓ Mean == 0 handled correctly")
            passed += 1
        else:
            print("✗ Mean == 0 not handled correctly")
            failed += 1
    except Exception as e:
        print(f"✗ Mean == 0: {e}")
        failed += 1

    # Test negative mean case
    try:
        mean, _, ratio = calculate_ci_width_ratio([-100.0, -102.0, -98.0])
        if mean < 0 and ratio >= 0:
            print(f"✓ Negative mean handled correctly (mean={mean:.2f}, ratio={ratio:.4f})")
            passed += 1
        else:
            print(f"✗ Negative mean not handled correctly (ratio={ratio})")
            failed += 1
    except Exception as e:
        print(f"✗ Negative mean: {e}")
        failed += 1

    samples = [100.0, 102.0, 98.0, 101.0, 99.0]
    mean, _, ci_width_ratio = calculate_ci_width_ratio(samples)
    if 99.5 <= mean <= 100.5:
        print(f"✓ Mean calculation correct ({mean:.2f})")
        passed += 1
    else:
        print(f"✗ Mean calculation incorrect ({mean:.2f})")
        failed += 1

    if ci_width_ratio > 0:
        print(f"✓ CI width ratio calculated ({ci_width_ratio:.4f})")
        passed += 1
    else:
        print(f"✗ CI width ratio invalid ({ci_width_ratio:.4f})")
        failed += 1

    print(f"\n{passed} passed, {failed} failed")
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
