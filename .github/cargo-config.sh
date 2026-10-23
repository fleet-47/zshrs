#!/bin/sh
# Writes the release cargo config to ./.cargo/config.toml. CPU flags equal
# `target-cpu=native` on the device; the kernel does not expose SVE, hence -sve,-sve2.
set -eu
mkdir -p .cargo
cat > .cargo/config.toml <<'TOML'
[build]
incremental = false
rustflags = ["-C", "target-cpu=cortex-x3", "-C", "target-feature=+aes,+bf16,+bti,+crc,+dit,+dotprod,+dpb,+dpb2,+fcma,+fhm,+flagm,+fp16,+frintts,+i8mm,+jsconv,+lor,+lse,+mte,+neon,+paca,+pacg,+pan,+pmuv3,+ras,+rcpc,+rcpc2,+rdm,+sb,+sha2,+sha3,+sm4,+spe,+ssbs,+vh,-sve,-sve2"]

[install]

[term]
color = "always"

[profile.bench]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
lto = "thin"
opt-level = 3
overflow-checks = false
rpath = false
split-debuginfo = "off"
strip = "symbols"

[profile.bench.build-override]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.bench.package."*"]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.dev]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
lto = "thin"
opt-level = 3
overflow-checks = false
panic = "abort"
rpath = false
split-debuginfo = "off"
strip = "symbols"

[profile.dev.build-override]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.dev.package."*"]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.release]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
lto = "thin"
opt-level = 3
overflow-checks = false
panic = "abort"
rpath = false
split-debuginfo = "off"
strip = "symbols"

[profile.release.build-override]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.release.package."*"]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.test]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
lto = "thin"
opt-level = 3
overflow-checks = false
rpath = false
split-debuginfo = "off"
strip = "symbols"

[profile.test.build-override]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"

[profile.test.package."*"]
codegen-units = 1
debug = "none"
debug-assertions = false
incremental = false
opt-level = 3
overflow-checks = false
split-debuginfo = "off"
strip = "symbols"
TOML
