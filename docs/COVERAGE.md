# Rust coverage snapshot

**89.3% line coverage — 1,182 / 1,324 unique instrumented source lines.**

This is a checked-in snapshot, not a live coverage service or a security guarantee.
The README badge links here so its scope and age stay visible.

| Measurement | Value |
|---|---|
| Source commit | [`d5744fdca7d8`](https://github.com/rainmana/md-to-pdf/commit/d5744fdca7d856a2571a8a6ec68d5af1e03a0c0d) |
| Measured (UTC) | 2026-10-25 |
| Platform | aarch64-apple-darwin |
| Rust toolchain | rustc 1.96.0 (ac68faa20 2026-05-25) |
| Coverage tool | cargo-llvm-cov 0.8.4 / LLVM LCOV per-line records |
| Test command | `cargo llvm-cov --locked --all-targets --json` |

The denominator includes every instrumented Rust file under `src/`, including
inline unit-test modules in those files. Standalone `tests/` files execute but
are excluded from the denominator, as are dependencies, the archived TypeScript
implementation, and generated/vendor files outside `src/`. Files that LLVM does
not report are listed below; they do not silently count as covered. Coverage
depends on the host platform and compiler, and is not branch coverage. Stable
Rust doctests are not instrumented by this command. LLVM's JSON and LCOV file
summary counts must agree. This badge counts unique `DA` line records from LLVM's
LCOV export; LLVM's own aggregate summaries can count more lines than its explicit
per-line records, so their aggregate percentage can differ. The raw exports are
retained for inspection. The generator unions line records after resolving
paths: `md2pdf` includes `main.rs`, so its separate compilation is merged with
`md-to-pdf` instead of counting the same source lines twice. A line hit by either
binary counts once.

| Source file | Covered lines | Instrumented lines | Coverage |
|---|---:|---:|---:|
| `src/config.rs` | 189 | 202 | 93.6% |
| `src/engine.rs` | 127 | 142 | 89.4% |
| `src/lib.rs` | 32 | 32 | 100.0% |
| `src/main.rs` | 246 | 331 | 74.3% |
| `src/markdown.rs` | 588 | 617 | 95.3% |
| **Total** | **1,182** | **1,324** | **89.3%** |

Source files absent from LLVM's report: `src/bin/md2pdf.rs`

## Refresh locally

Use the repository's pinned Rust toolchain and install the coverage tooling:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked --version 0.8.4
mkdir -p .analysis/coverage
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo llvm-cov --locked --all-targets --json \
  --output-path .analysis/coverage/coverage.json
cargo llvm-cov report --lcov --output-path .analysis/coverage/coverage.lcov
python3 scripts/coverage-badge.py .analysis/coverage/coverage.json \
  --lcov .analysis/coverage/coverage.lcov \
  --source-commit "$(git rev-parse HEAD)" --date "$(date -u +%F)" \
  --platform "$(rustc -vV | sed -n 's/^host: //p')" \
  --toolchain "$(rustc --version)"
git diff -- assets/badges/coverage.svg docs/COVERAGE.md
```

The generator uses only Python's standard library. It takes LLVM's measured
line records rather than counting source text or selecting successful functions.
Review the two generated files before committing a new snapshot. Use `--offline`
with `cargo llvm-cov` when all locked Cargo dependencies are already cached.

## Continuous reports

The [Coverage workflow](../.github/workflows/coverage.yml) runs on pushes, pull
requests, and manual dispatch. It uploads the raw LLVM JSON and LCOV, generated SVG, and
report, and adds the report to the GitHub job summary. It has read-only repository
permissions and does not upload to an external coverage service, publish Pages,
or commit changes. Its latest artifacts can be newer than this checked-in badge;
refreshing the snapshot remains an explicit reviewed documentation change.
