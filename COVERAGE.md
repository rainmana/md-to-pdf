# Rust coverage report

**88.9% line coverage — 1,177 / 1,324 unique instrumented source lines.**

This is a measured result from an instrumented test run, not a security guarantee.
The README badge follows the latest published successful coverage run on `master`.

| Measurement | Value |
|---|---|
| Source commit | [`47a8b1cf31bb`](https://github.com/rainmana/md-to-pdf/commit/47a8b1cf31bbc3063081609dcb46717d148bdfef) |
| Measured (UTC) | 2026-10-25 |
| Platform | x86_64-unknown-linux-gnu |
| Rust toolchain | rustc 1.98.1 (48a229cea 2026-09-01) |
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
| `src/main.rs` | 241 | 331 | 72.8% |
| `src/markdown.rs` | 588 | 617 | 95.3% |
| **Total** | **1,177** | **1,324** | **88.9%** |

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
# Outputs: .analysis/coverage/coverage.svg and .analysis/coverage/COVERAGE.md
```

The generator uses only Python's standard library. It takes LLVM's measured
line records rather than counting source text or selecting successful functions.
Local outputs stay in the ignored `.analysis/coverage` directory. Use `--offline`
with `cargo llvm-cov` when all locked Cargo dependencies are already cached.

## Continuous reports

The [Coverage workflow](https://github.com/rainmana/md-to-pdf/actions/workflows/coverage.yml) runs on pushes, pull
requests, and manual dispatch. It uploads the raw LLVM JSON and LCOV, generated SVG, and
report, and adds the report to the GitHub job summary. Measurement has read-only
repository permissions. After a successful push or manual run on `master`, a
separate publisher updates only the badge, report, and run metadata on the
`coverage-data` branch. Pull requests and other branches cannot publish.

The README reads that hosted badge automatically. The
[published report](https://github.com/rainmana/md-to-pdf/blob/coverage-data/COVERAGE.md)
and [run metadata](https://github.com/rainmana/md-to-pdf/blob/coverage-data/metadata.json)
identify the measured source commit and workflow run. GitHub's image/CDN caches
can briefly delay the displayed image. Publication uses the workflow's temporary
GitHub token, without a separate coverage-service account or personal token.
