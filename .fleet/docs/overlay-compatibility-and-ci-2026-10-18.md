# Overlay compatibility and standard CI — 2026-10-18

## Agreed scope and decisions

Yazid requested correcting all 23 patches to preserve upstream behavior, except
clear improvements described explicitly, then added overlay coverage to standard
CI. Engine choices were made individually:

- Keep upstream rand 0.8, fancy-regex 0.18, printpdf 0.7 and notify 6/debouncer 0.4.
- Keep SQLite only if dependent queries/functions are unchanged. The static audit
  found an externally visible invalid-TEXT error change in daemon `op_keys` and
  cache readers, so hold rusqlite 0.40 under that rule. Healthy-data query results
  were not executed or proven different.
- Keep corrected ureq 3, accepting reporting differences and failed-upload-redirect
  reporting. Its status diagnostics also omit Gemini's API-key-bearing URL.
- Keep cron 0.17's next-occurrence fixes and richer parse diagnostics.

Six candidates are held; 17 remain enabled. Holds preserve candidate files and
are explicit/logged, never silent fallback. No original source files have been
patched in the current checkout.

## What changed and why

- `series.toml` records six holds. The runner skips their application/retirement,
  validates nonempty reasons and checks their upstream input with forward dry runs
  before any source edits. It stops on an already-applied/incompatible held patch.
- Runner `apply --ci` explicitly prepares disposable CI checkouts without jj or
  retirement. Normal local ancestry/retirement rules remain.
- `ureq.patch` restores proxy, timeout, redirect-boundary, User-Agent, capped read
  and text-decoding policies. `axum.patch` restores optional-JSON extraction behavior.
  The HTTP migration adds a real-daemon regression case for malformed JSON and a
  wrong Content-Type, preserving the existing tests.
- `libloading.patch` restores Unix invalid-filename diagnostics; `crossterm.patch`
  opts into the old libc backend. `zstd.patch` retains incremental decoding allocation.
- `native-tool-path.patch` adds the recorder's Android-only default PATH using
  the installed Termux header's `/data/data/com.termux/files/usr/bin`; no inherited
  PATH or header-probe build framework. The non-Android branch is unchanged.
- `.github/workflows/ci.yml` adds baseline/overlay checks, Ubuntu overlay tests and
  an aarch64 Android NDK API29 compile/link gate. Optional features are
  `recorder,zd,parity-fuzz,prometheus`. Existing baseline test coverage remains.
  `.github/actions/android-overlay/action.yml` applies the series, minimally resolves
  dependencies and archives each job's Cargo.lock; later checks use `--locked`.
- `zshrs-android/COMPATIBILITY.md` covers all 23 decisions and exact improvements;
  README now describes the implemented policy/CI rather than superseded plans.

## Inspection and removal

Inspect the named patch files, `series.toml`, runner `main.rs`, workflow/action and
compatibility document. The runtime checkout still contains upstream code.
To undo this turn, restore those named files to their pre-turn versions through jj
and remove the newly added action/compatibility/session-note files as one reviewed
change. Do not merely remove hold lines: that re-enables deliberately rejected
upgrades. No commit, VCS operation, CI dispatch or push was performed here.

## Validation and review

Only rg/sem/ast-grep inspection and file-edit tools were allowed. In-memory
JavaScript analysis of rg output verified all 23 patch structures (316 hunks),
the enabled series' 76 hunks against exact contexts in application order, and
the six held candidates against their current baseline contexts. No patch utility,
Cargo/Rust command, SQL execution, dependency resolution or CI run was executed.

Source review and an independent gpt-6-astra frontier review found no additional
substantive defects after fixes. Independent dependency revalidation was limited
because some newer cached dependency sources read earlier were unavailable later.
Compilation and runtime equivalence remain unproven.

Reflection: validating patches individually missed a cross-patch context dependency
(ureq expected the pre-upgrade zstd manifest line). Full-series in-memory validation
caught it; trim unrelated manifest context and validate in series order. A misplaced
Zstd hunk introduced during editing was also corrected before final validation.
Use this complete-series check for future overlay maintenance.

## Re-entry

Completed: agreed engine policy, targeted compatibility edits and standard CI wiring.
Open: actual CI compile/test/link results, Termux device runtime/terminal behavior,
signature interoperability, and any transitive dependency differences in the resolved
graph. First next step is inspect the ordinary CI results after these changes are
committed/pushed by an authorized actor; no green build is claimed. Held engine
upgrades remain separate future decisions, not unfinished approved migrations.
