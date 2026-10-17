# Overlay compatibility decisions

The series contains 23 candidate patches; 17 are enabled and six are held.
These decisions follow Yazid's 2026-10-18 review: preserve upstream behavior,
retain clear improvements explicitly, and decide engine replacements individually.
The upstream source and lockfile are not patched in this checkout.

`hold` in `series.toml` means neither apply nor retire that candidate. The runner
prints the reason. Before changing source, it checks that held patches still
match their upstream input. An already-upgraded or incompatible checkout stops;
the runner does not silently leave a held upgrade installed or undo user changes.
Use a clean upstream checkout when changing this policy. Candidate patch files
remain available for review; removing a hold is a new compatibility decision.

## All 23 decisions

| Patch | Decision and resulting behavior |
| --- | --- |
| libloading | Enabled. Preserve normal Unix loader flags/ownership and restore the two old embedded-NUL filename diagnostics at the plugin boundary. |
| fancy-regex | **Held by user decision.** Keep 0.18 matching behavior until shell parity is demonstrated. |
| nix | Enabled. Borrow the descriptor for `fcntl`; preserve the syscall and descriptor ownership. |
| rusqlite | **Held under the user's dependent-function rule.** Keep 0.32 and upstream bundled SQLite. Malformed stored TEXT changes the error variant and externally reported diagnostic in the upgrade. |
| dirs | Enabled. Inspected `home_dir()` implementation is unchanged on the supported Unix paths; Android still needs nonempty HOME. |
| rand | **Held by user decision.** Keep 0.8 fork recovery, sampling algorithms and seeded replay. The candidate's rand08 alias/test migration is also inactive. |
| metrics-exporter-prometheus | Enabled improvement. Render scrapes on a blocking worker rather than the asynchronous listener thread. Text exposition and configured listener remain; internal storage also changes. No measured throughput claim. |
| sha2 | Enabled. Preserve digest bytes and lowercase, zero-padded hexadecimal representation. The dependency also contains an AArch64 implementation correction. |
| notify | **Held by user decision.** Keep notify 6, avoiding new OPEN events and associated wakeups on pure reads. |
| notify-debouncer-mini | **Held with notify.** Keep 0.4 and existing coalescing/event source. |
| crossterm | Enabled with its `libc` feature to preserve the Unix terminal backend. Other dependency internals, including Mio, still change; full terminal equivalence is unproven. |
| zstd | Enabled with `copy_decode` into empty vectors at all three decoding sites, preserving incremental allocation instead of trusting frame-size metadata for a new reservation. The inspected C codec version remains 1.5.7. |
| ureq | **Corrected upgrade accepted by user.** Preserve request/body policy below; accept changed error reporting and failed redirected-upload reporting. |
| base64 | Enabled improvement to malformed-input diagnostics: byte value/character and decoded bits give more precise failure information. Valid STANDARD encoding/decoding remains unchanged in the inspected paths. |
| ed25519-dalek | Enabled security improvement: constant-time compressed-point comparison during strict verification. No key/signature-format incompatibility found; cross-version interoperability has not been executed. |
| printpdf | **Held by user decision.** Keep 0.7 rendering and document structure. |
| cron | **0.17 fixes accepted by user.** Correct next-occurrence searches across years and weekdays; retain richer invalid-expression diagnostics. |
| axum | Enabled with compatibility adaptation. `Result<Json<_>, JsonRejection>` followed by `.ok()` restores upstream's treatment of all optional-body extraction failures as absent arguments. Route syntax adapts to 0.8. |
| tower | Enabled. No direct tower calls were found; the old axum already used tower 0.5 transitively. |
| Android native-tool-path | Enabled platform improvement. Fuzzer finds the matching sibling shell, retaining explicit override precedence. Recorder uses Termux's clean default PATH on Android; other platforms retain `confstr`. |
| Android libc-bindings | Enabled binding/re-export compatibility. Android's no-op utmp API still supplies no login records; declarations cannot supply missing OS functionality. |
| Android interface-counters | Enabled Android platform selection. Use existing sysfs reads; their existing read-error-to-zero behavior is retained. |
| Android random-source | Enabled import repair. Android retains its existing `/dev/urandom` implementation and error handling; no RNG algorithm is replaced by this patch. |

## HTTP compatibility and accepted differences

Production clients and migrated HTTP tests explicitly disable environment proxy
discovery, restore a 30-second connect timeout, and preserve explicit overall
timeouts. Upstream ureq 2's configured five-attempt policy followed at most four
redirects; ureq 3 is configured with four followed redirects to match that boundary.
The default User-Agent remains `ureq/2.12.1` as a compatibility wire value; the
implementation is ureq 3. Explicit request headers can still override it.

Text reads retain the inclusive 10 MiB byte limit and lossy UTF-8 decoding
regardless of MIME type. JSON reads remain streaming. Snapshot reads retain
`Read::take(MAX_REMOTE_BYTES)` and its capped-buffer behavior.

Accepted reporting changes:

- POST/PUT 307/308 redirects that cannot replay the body now report failure,
  rather than presenting the original redirect response as upload success.
  The original server receives the request; the redirected destination receives
  no replayed upload.
- Dependency transport, status and JSON-decoding diagnostics differ. In
  particular, omitting the request URL from HTTP-status errors prevents the
  Gemini URL's API key from being included in that diagnostic. This is a specific
  improvement, not a claim that all diagnostics or application logging redact secrets.

The added daemon HTTP regression test sends malformed JSON and a wrong
Content-Type to `/op/ping`, checking the upstream empty-arguments behavior.

## Held engine differences and future improvements

- **Rand 0.10:** loses automatic post-fork recovery and changes seeded sampling.
  A future migration needs explicit child reseeding, old replay algorithms where
  seeds are a contract, and unbiased sampling. No measured speed benefit justifies
  those changes here.
- **SQLite 3.53 versus 3.46:** query planner, SQL capabilities and fixes change.
  The dependency/query scan covered completion/history/FTS, plugin and compsys
  caches, catalog/import/export, jobs/scheduling, artifacts, transactions and
  maintenance operations. No execution established unchanged results. A concrete
  dependent-function change was sufficient to hold the patch: `op_keys` reads
  strings and exposes conversion errors through IPC. Old invalid-TEXT errors say
  `Conversion error from type Text at index: 0, ...`; new ones say `... at index 0`.
  PluginCache and other String reads are also affected. This is not evidence of
  changed healthy-data results. Future acceptance needs real database copies and
  query/result/error comparisons, including FTS and ordering without tie-breakers.
- **Printpdf 0.12:** unsupported characters become `?` rather than disappearing;
  document structure changes and debug streams compress. Neither version provides
  full Unicode rendering with the current builtin fonts. Proper Unicode font
  embedding is a separate potential improvement.
- **Fancy-regex 0.19:** full Unicode-fold backreference equality, inline flags and
  `\G` fixes are useful correctness improvements, but change edge-case answers.
  Compare against the shell oracle before accepting them.
- **Notify 8/debouncer 0.7:** a wrapper can suppress false downstream change events
  but cannot eliminate the extra kernel OPEN events/wakeups. No concrete benefit
  for this daemon was established; retaining the old pair avoids both costs.

Cron's retained correction is concrete: searching from October for a specified
June event in the following year now finds it, instead of incorrectly carrying
the October month bound into that year. The daemon uses UTC; DST fixes are not
claimed as benefits on this path.

## Validation boundary

Only `rg`, `sem`, `ast-grep` inspection and file-edit tools were permitted locally.
Static validation checked all 23 unified diffs and exact source contexts for the
17 enabled patches in series order, in memory. It did not execute the patch
utility, compile, resolve dependencies, run tests or execute SQL. Independent
review inspected the changes; some newer registry sources were unavailable on
the later review pass, limiting independent dependency revalidation.

Standard CI now applies the enabled overlay in disposable checkouts, resolves
dependencies without a broad `cargo update`, archives each resolved lockfile,
and runs locked checks/tests. The Android job compiles and links the aarch64
workspace against the runner's NDK at API 29, including optional binaries and
Prometheus. It does not run those binaries on Android or prove Termux runtime
compatibility. Device runtime tests, terminal behavior and signature
interoperability remain open. The CI changes have not been executed here.

Major dependency upgrades and a newly resolved transitive graph cannot honestly
be certified as identical from source inspection alone. The guarantees here are
the explicit preserved policies and selected holds; remaining validation is a gate,
not an assumed success.
