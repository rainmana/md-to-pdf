# Rust coverage

[![Hosted Rust line coverage](https://raw.githubusercontent.com/rainmana/md-to-pdf/coverage-data/coverage.svg)](https://github.com/rainmana/md-to-pdf/blob/coverage-data/COVERAGE.md)

The README badge follows the **latest successfully published coverage run on
`master`**. It is measured by GitHub Actions on Linux with the repository's
pinned Rust toolchain. There is no manually maintained percentage in the README.

- [Current measured report](https://github.com/rainmana/md-to-pdf/blob/coverage-data/COVERAGE.md)
- [Source commit and workflow-run metadata](https://github.com/rainmana/md-to-pdf/blob/coverage-data/metadata.json)
- [Coverage workflow and run history](https://github.com/rainmana/md-to-pdf/actions/workflows/coverage.yml)

GitHub's image/CDN caches can briefly delay the displayed image. The report and
metadata identify the source commit and workflow run that produced the published
result. If a newer run fails, the badge keeps the last successful published
measurement; check the workflow history for current failures.

## What is measured

The percentage counts unique instrumented Rust source lines under `src/`,
including inline unit-test modules. Standalone integration-test source files,
dependencies, archived TypeScript, and generated/vendor files outside `src/` are
excluded from the denominator. The integration tests still execute during the
measurement. This is line coverage, not branch coverage or a security guarantee.
Stable Rust doctests are not instrumented by this command.

`cargo-llvm-cov` exports JSON and LCOV from the same instrumented test run. The
report generator validates their file-summary counts, then unions canonical
per-line records. `md2pdf` includes `main.rs`; counting that shared source once
prevents the two command names from duplicating the denominator. A line covered
by either binary counts once. LLVM's aggregate summaries can count more lines
than its explicit LCOV line records, so their aggregate percentage can differ.
Files absent from LLVM's report are listed explicitly in the generated report.

Coverage varies with the platform and compiler. The old local macOS measurement
is retained in Git history; it is no longer the README's badge source.

## How publication works

The [workflow](../.github/workflows/coverage.yml) measures coverage on pushes,
pull requests, and manual dispatch. Its measurement job has read-only repository
permissions and uploads raw JSON, LCOV, the SVG badge, and a Markdown report.
The report also appears in the GitHub job summary.

After a successful push or manual run on `master`, a separate publisher gets
`contents: write` permission. It publishes only `coverage.svg`, `COVERAGE.md`,
and `metadata.json` to the dedicated `coverage-data` branch. Pull requests and
other source branches cannot update that branch. Publication verifies that the
measured commit still matches remote `master`, and an older workflow run cannot
replace a newer result for the same commit. Updates preserve branch history and
use ordinary Git pushes.

This uses the workflow's temporary `GITHUB_TOKEN`. It needs no external coverage
service, personal access token, GitHub Pages site, or bot commits on `master`.
The token's generated branch pushes do not recursively start another workflow.

## Refresh the hosted result

A successful push to `master` refreshes the badge automatically. To remeasure
without changing source, select **Run workflow** on the
[Coverage workflow](https://github.com/rainmana/md-to-pdf/actions/workflows/coverage.yml)
with `master` selected, or run:

```sh
gh workflow run coverage.yml --repo rainmana/md-to-pdf --ref master
```

## Measure locally

Local measurements write only ignored files, so they cannot replace the hosted
badge or this documentation. With the pinned toolchain selected:

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
```

The generated `.analysis/coverage/coverage.svg` and `COVERAGE.md` record that
measurement's platform and compiler. Use `--offline` with `cargo llvm-cov` when
all locked Cargo dependencies are cached. The publication safeguard tests use
local temporary Git repositories and can be run without network access:

```sh
python3 scripts/test-publish-coverage.py
```
