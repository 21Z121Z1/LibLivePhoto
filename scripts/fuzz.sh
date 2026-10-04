#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
seconds="${1:-60}"
log_root="${2:-evidence/local/fuzz}"
mkdir -p "$log_root"
cargo run --quiet -p liblivephoto --example fuzz_seeds
for target in asset_input container_headers remux; do
  cargo +nightly fuzz run "$target" --sanitizer address -- \
    -max_total_time="$seconds" -max_len=1048576 -rss_limit_mb=2048 -timeout=5 \
    -print_final_stats=1 2>&1 | tee "$log_root/$target.log"
done
