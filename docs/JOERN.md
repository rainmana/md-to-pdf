# Original TypeScript Joern index

The original source at Git commit `8fe27113574f77377d44dda284d8e782588ada90`
was indexed locally on 2026-10-23, before the Rust rewrite. No Joern installation
or download was needed. The local executable is `/opt/homebrew/bin/joern`, from
Homebrew package `4.0.530`; its runtime banner reports `HEAD+20260501-1846`.
The indexed source is now preserved under `legacy/typescript/src/`; graph file
names retain their paths relative to the original `src/` input directory.

Two graphs cover all 20 TypeScript/JavaScript files originally under `src/`:

| Graph | Files | Nodes | Methods | Internal methods | Calls | Type declarations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `.analysis/joern/cpg.bin` | 15 | 6,397 | 185 | 66 | 1,258 | 142 |
| `.analysis/joern/fixtures-cpg.bin` | 5 | 4,074 | 103 | 56 | 802 | 68 |

Both graphs were reopened successfully and queried with the JSSRC language and
`base,controlflow,typerel,callgraph,dataflowOss` overlays. Method counts include
Joern's synthetic and external methods; node counts describe each graph after
import, including its import/type-recovery passes.

The JSSRC frontend skips test/spec files by default. Five byte-identical copies
were therefore staged under neutral filenames for the supplemental graph:

| Supplemental filename | Original path |
| --- | --- |
| `api.ts` | `src/test/api.spec.ts` |
| `cli.ts` | `src/test/cli.spec.ts` |
| `library.ts` | `src/test/lib.spec.ts` |
| `api-types.ts` | `src/test/api.type-spec.ts` |
| `mathjax-config.js` | `src/test/mathjax/mathjax-config.js` |

The supplemental graph preserves syntax and local relationships but does not
preserve the original relative import layout. It is separate from the production
graph; cross-graph call resolution is not implied.

## Reproduce

From the rewritten repository root, with Joern already installed (in a checkout
of the original commit, replace `legacy/typescript/src` with `src`):

```sh
mkdir -p .analysis/joern/fixtures-input
joern-parse legacy/typescript/src --language jssrc --output .analysis/joern/cpg.bin
cp legacy/typescript/src/test/api.spec.ts .analysis/joern/fixtures-input/api.ts
cp legacy/typescript/src/test/cli.spec.ts .analysis/joern/fixtures-input/cli.ts
cp legacy/typescript/src/test/lib.spec.ts .analysis/joern/fixtures-input/library.ts
cp legacy/typescript/src/test/api.type-spec.ts .analysis/joern/fixtures-input/api-types.ts
cp legacy/typescript/src/test/mathjax/mathjax-config.js .analysis/joern/fixtures-input/mathjax-config.js
joern-parse .analysis/joern/fixtures-input --language jssrc \
  --output .analysis/joern/fixtures-cpg.bin
cd .analysis/joern
joern cpg.bin
```

Example queries at the Joern prompt (repeat after opening `fixtures-cpg.bin`):

```scala
cpg.metaData.language.l
cpg.metaData.overlays.l
cpg.all.size
cpg.file.name.l.sorted
cpg.method.size
cpg.method.isExternal(false).size
cpg.call.size
cpg.typeDecl.size
```

The ignored local directory also contains `verify.sc`, `verify.log`,
`fixtures-verify.log`, parser logs, `source-sha256.txt`, and
`supplemental-source-map.json`. Verification can be repeated from that directory:

```sh
joern --nocolors --script verify.sc \
  --param cpgFile="$PWD/cpg.bin"
joern --nocolors --script verify.sc \
  --param cpgFile="$PWD/fixtures-cpg.bin"
```

## Evidence and limitations

The production parser and graph loader emitted nonfatal type-recovery warnings
for `new URL(input).protocol` in original `src/lib/is-http-url.ts:8`:
`Unhandled field structure` and `Unable to assign identifier '_tmp_0'`.
Parsing and graph import still completed successfully. The supplemental parser
and loader emitted no warnings. These are static source indexes, not dependency
indexes or proof of runtime behavior.

SHA-256 of the graph files immediately after parsing:

```text
5bcbb0cd1e1cfffc4fdc49d14ae7f98752f1266e6bef00eec64663d8af4a518b  cpg.bin
334a9507e2358e6a6342fe789bd5235d4a28e826e18b2fd269143a01a483d8b1  fixtures-cpg.bin
```

Graph serialization can vary across runs or Joern versions, so these hashes
identify the local artifacts rather than asserting reproducible graph bytes.
