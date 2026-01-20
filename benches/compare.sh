#!/usr/bin/env bash
#
# CU Benchmark Comparison Script
#
# Compares benchmark results between two runs (e.g., before/after optimization).
#
# Usage:
#   ./compare.sh baseline.log optimized.log
#   ./compare.sh                  # Runs benchmark and saves to results/
#
# Output format:
#   Benchmark Name | Before (ns) | After (ns) | Improvement (%)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

run_benchmarks() {
    local output_file="$1"
    echo "Running benchmarks..."
    cargo test --package mcpsol-core --test cu_benchmarks -- --nocapture 2>&1 | tee "$output_file"
    echo "Results saved to: $output_file"
}

parse_benchmark() {
    local file="$1"
    local name="$2"
    grep "BENCHMARK: $name " "$file" | sed 's/.*per_op_ns=\([0-9]*\).*/\1/' | head -1
}

compare_results() {
    local before="$1"
    local after="$2"

    echo ""
    echo "============================================================"
    echo "CU BENCHMARK COMPARISON"
    echo "============================================================"
    echo ""
    printf "%-30s | %12s | %12s | %12s\n" "Benchmark" "Before (ns)" "After (ns)" "Change"
    echo "-------------------------------+--------------+--------------+-------------"

    for bench in "compact_minimal" "compact_typical" "compact_complex" \
                 "paginated_direct_typical" "paginated_cached_typical" \
                 "paginated_direct_complex" "paginated_cached_complex" \
                 "cache_init_typical" "cache_init_complex" \
                 "estimate_schema_size" "estimate_single_tool"; do

        before_ns=$(parse_benchmark "$before" "$bench" 2>/dev/null || echo "N/A")
        after_ns=$(parse_benchmark "$after" "$bench" 2>/dev/null || echo "N/A")

        if [[ "$before_ns" != "N/A" && "$after_ns" != "N/A" && "$before_ns" != "" && "$after_ns" != "" ]]; then
            if [[ "$before_ns" -gt 0 ]]; then
                diff=$((before_ns - after_ns))
                pct=$(echo "scale=1; $diff * 100 / $before_ns" | bc 2>/dev/null || echo "0")

                if (( diff > 0 )); then
                    color=$GREEN
                    sign="+"
                elif (( diff < 0 )); then
                    color=$RED
                    sign=""
                else
                    color=$NC
                    sign=""
                fi
                printf "%-30s | %12s | %12s | ${color}%s%.1f%%${NC}\n" "$bench" "$before_ns" "$after_ns" "$sign" "$pct"
            else
                printf "%-30s | %12s | %12s | %12s\n" "$bench" "$before_ns" "$after_ns" "N/A"
            fi
        else
            printf "%-30s | %12s | %12s | %12s\n" "$bench" "${before_ns:-N/A}" "${after_ns:-N/A}" "-"
        fi
    done

    echo ""
}

main() {
    mkdir -p "$RESULTS_DIR"

    if [[ $# -eq 2 ]]; then
        # Compare two provided files
        compare_results "$1" "$2"
    elif [[ $# -eq 1 ]]; then
        # Run benchmark and compare with provided baseline
        timestamp=$(date +%Y%m%d_%H%M%S)
        current_log="${RESULTS_DIR}/benchmark_${timestamp}.log"
        run_benchmarks "$current_log"
        compare_results "$1" "$current_log"
    else
        # Run benchmark and save
        timestamp=$(date +%Y%m%d_%H%M%S)
        current_log="${RESULTS_DIR}/benchmark_${timestamp}.log"
        run_benchmarks "$current_log"
        echo ""
        echo "To compare with a baseline, run:"
        echo "  $0 baseline.log $current_log"
    fi
}

main "$@"
