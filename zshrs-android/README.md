# Android patches

`runner/` is a Rust binary that applies enabled entries in `series.toml` to this checkout.
Android compatibility patches and dependency upgrades are kept in separate directories.
The 23 candidates currently comprise 17 enabled patches and six explicit holds.
See [compatibility decisions](COMPATIBILITY.md) for every patch and accepted improvement.

From the repository root, run:

```sh
cargo run --release --manifest-path zshrs-android/runner/Cargo.toml -- apply
```

`build.sh` is a shortcut for that command; it only runs the patch tool.
The runner creates no workspaces or output directories and runs no builds, checks,
tests, or smoke commands. Cargo uses its existing configuration for its own artifacts.
Tool paths are configured in `tools.toml`.

The checkout must descend from `main`. Patches apply with zero fuzz; reverse dry runs
recognize patches already applied. Failures stop processing; partial changes are
visible in jj. Upgrade patches are automatically removed when unpatched `main`'s
manifest and lockfile supply their target versions. Android patches never retire
by dependency version. Changes and deletions can be recovered through jj's oplog.
This version comparison is not proof of runtime compatibility.

Held entries are neither applied nor retired, and their reasons are printed.
Before any source changes, a forward dry run checks each held candidate still
matches upstream input. A previously applied held upgrade or incompatible source
stops processing; use a clean upstream checkout or review the hold. The runner
does not undo existing modifications automatically.

Standard CI uses `apply --ci` in disposable checkouts. This explicit mode applies
enabled entries without jj ancestry checks or automatic retirement. Held-patch
preflight and zero-fuzz patch checks still run. Normal local `apply` retains its
main-ancestry and retirement behavior.

## Upstream validation and native builds

The patch tool only prepares source. Run the existing upstream tests on the
patched checkout; a successful test of unpatched main does not validate the overlay.
No second build runner or output directory is needed.

`tests/android_overlay_contract.rs` is an additive, dependency-free integration
test discovered by the existing upstream test command and CI. It checks real candidate
patches for valid unified-diff structure and rejects changes to upstream test paths
except the explicitly authorized rand/ureq API migrations.
It does not prove that patches apply, preserve inline tests, or compile on Android.
The standard workflow now adds overlay checks and Ubuntu workspace tests alongside
the baseline jobs, plus an aarch64 Android compile/link job at API 29. Overlay tests
build workspace binaries first and enable `recorder,zd,parity-fuzz,prometheus`.
The shared CI action applies the overlay, resolves its graph and archives Cargo.lock;
subsequent checks are locked. No Android runtime execution is claimed.
Test API edits remain in patch data rather than applied directly to main.

Before testing, follow the reference-shell and source setup in
[CI](../.github/workflows/ci.yml): the pinned zsh oracle, release zsh oracle, upstream
C source for port-purity checks, and reference shells for parity tests. Missing
oracles can cause skips; an unprepared local test run is not equivalent to CI.
`MAINTAINERS.md` still mentions a gated ztst job, but current CI explicitly removed
that job pending the PTY harness. Do not claim that gate ran.

The following commands are reference instructions, not automatic runner actions.
After applying dependency upgrades, resolve the lockfile before using the upstream
locked validation convention:

```sh
cargo metadata --format-version 1 > /dev/null
cargo check --workspace --all-targets --locked --features recorder,zd,parity-fuzz,prometheus
cargo build --workspace --bins --locked --features recorder,zd,parity-fuzz,prometheus
cargo test --workspace --locked --no-fail-fast --features recorder,zd,parity-fuzz,prometheus
```

The full workspace native build, with the release tools and requested fuzz driver,
is separate from validation:

```sh
cargo build --release --workspace --locked --features recorder,zd,parity-fuzz
```

`recorder` enables the recorder and daemon; `zd` enables the daemon client;
`parity-fuzz` enables the development fuzz driver. Defaults remain enabled.
`--workspace` includes the daemon and runtime workspace members. Cargo's configured
host, target directory, jobs, and compiler settings remain authoritative. This
native build is not a tagged release: upstream release packaging uses its `dist`
profile and separately lists shipped binaries in `release.yml`; the fuzz driver is
not part of those release archives. Upstream has no Android release job.

## Remaining validation

- Rand remains upstream 0.8 for both runtime and tests; its candidate API migration
  is held. The enabled ureq patch migrates the existing HTTP tests and adds a
  malformed/mistyped optional-JSON regression test. No tests are excluded.
- Recorder's missing Android `confstr` call is addressed in `native-tool-path.patch`
  with a target-specific default PATH. It has not been compiled locally.
- The earlier full release build stopped before completion. The new CI gates and
  native runtime behavior remain unexecuted; no successful Android build is claimed.

## Recorder PATH for this deployment

The Android-only recorder branch uses `/data/data/com.termux/files/usr/bin`, the
installed Termux `<paths.h>` `_PATH_DEFPATH`. It preserves environment scrubbing;
it never inherits the launching shell's expanded PATH. Non-Android platforms keep
their `confstr` branch. Other Android installations need an explicit deployment
setting before this overlay is suitable for them. No header-probe framework or
compiler dependency is added, and the ported shell's DEFAULT_PATH is unchanged.
