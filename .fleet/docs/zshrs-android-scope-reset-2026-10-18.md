# Scope reset — 2026-10-18

At Yazid's request, retain only the core patch tool and Android/dependency patch
series on top of main. Eight superseded changes were abandoned with jj. Earlier
artifacts, reports, and implementations remain recoverable through the oplog
(pre-abandon operation: ad01b3409b71).

Removed workspace/output management, Cargo update/check/test/build orchestration,
state snapshots, smoke commands, extra integration test, and failed recorder PATH
probe. The runner now applies patches in this checkout; automatic retirement uses
unpatched main's dependency manifests and lockfile. Retirement is version-based,
not a claim of build or runtime validation. Upstream source and tests are unchanged.

Inspect: jj diff --from main. Undo: jj op log and jj op restore to the desired
operation. No garbage collection or pushes were performed. Existing workspace
registrations remain, with empty changes based on main; their directories were
not removed or refreshed. .fleet/.gitignore is retained solely to prevent those
existing generated directories/logs from becoming tracked.

Validation: cargo check --offline --manifest-path
zshrs-android/runner/Cargo.toml --all-targets passed. The runner was not applied,
and no zshrs build or test execution was started.

Open: rand/ureq major upgrades conflict with unchanged upstream test APIs;
recorder still needs an Android default-PATH fix. First next step is resolve that
dependency/test policy before resuming native compilation.

## Minimal upstream integration follow-up

Restored only tests/android_overlay_contract.rs from 69f015c3. It adds structural
coverage to the existing upstream integration suite without changing any existing
test. Expanded zshrs-android/README.md with the existing CI prerequisites, future
validation/build commands, and confirmed compilation blockers. No runner build
orchestration, workflow, Cargo configuration, dependency alias policy, or failed
recorder PATH probe was introduced. Inspect with jj diff --from main; remove this
follow-up with jj restore from the preceding operation for these two paths.

No Cargo commands were executed in this follow-up, as explicitly requested.
Validation is limited to source/patch inspection and formatting of the restored
test. First next step remains the rand/ureq test-dependency decision and a properly
specified native recorder PATH fix before any future compilation.

## Authorized test API migrations

Yazid authorized API-only test changes and then explicitly chose rand 0.8 for
tests. rand.patch now adds a dev-only rand08 alias and changes only the two
startup parity imports, retaining its generator, methods, ranges, seeds and body.
ureq.patch carries the HTTP test API migration. AST comparison found all 21 test
cases and 81 assertion/payload macros unchanged. JSON decoding still streams
through serde_json, avoiding a new body-size limit. HTTP error-body tests retain
the response and assert the same status/content; transport errors still fail.
The additive contract test now allows only those two named test paths in their
matching upgrade patches. This narrowly replaces the earlier no-test-edits rule.

Recorder recommendation only: use this target's paths.h _PATH_DEFPATH, which is
/data/data/com.termux/files/usr/bin on this Termux device, in an Android-only
recorder branch; keep confstr elsewhere. No recorder code or build machinery was
added. No Cargo commands were run. Compile/runtime verification remains pending.

## HTTP snapshot read compatibility

Yazid authorized restoring upstream's HTTP snapshot size-boundary behavior.
The ureq upgrade patch now retains `std::io::Read` and reads through
`body_mut().as_reader().take(MAX_REMOTE_BYTES).read_to_end(&mut buf)`.
This preserves upstream's capped-buffer result at/above the 1 GiB limit,
instead of introducing ureq's limit-exceeded error. Read errors still propagate.
The preceding review identified the cause as replacing `Read::take` with
ureq's enforcing body limit during the API migration.

Inspect `zshrs-android/patches/upgrades/ureq.patch`'s `daemon/snapshot.rs`
section. To undo only this change, restore its previous enforcing-limit hunk
and import-removal hunk; no runtime source is patched by this edit.
Validation is source/patch inspection only; no Cargo or Rust commands are
authorized in this session. Full compilation and runtime checks remain open.
Re-entry: retain the upstream capped-read chain in future HTTP API migrations;
resume the previously recorded native-build blockers when execution is authorized.
