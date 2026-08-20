# Build and Flash Specification

## Purpose

The project SHALL build byte-for-byte reproducibly for the STM32C562RE and flash to the
NUCLEO-C562RE with zero manual host setup beyond installing the documented tools. This
covers the pinned toolchain, the git-pinned Embassy dependency, the cross-compilation
target wiring, the linker memory map, the stack-overflow-protecting linker, and the
vendored probe-rs chip description.

## Architecture

- **`rust-toolchain.toml`**: pins the exact Rust stable version, components, and the MCU
  target `thumbv8m.main-none-eabihf`, so every machine and CI runner uses the same
  compiler. Recent enough to parse Embassy `main`'s edition-2024 manifests.
- **`Cargo.toml`**: consumes `embassy-executor`, `embassy-time`, and `embassy-stm32` from
  git pinned to an exact `rev` (not a moving branch), with the `stm32c562re` chip feature.
  Profiles use `codegen-units = 1`, LTO, `panic = "abort"`, and `overflow-checks = true`;
  release optimizes for size (`opt-level = "s"`).
- **`Cargo.lock`**: committed to lock the full dependency graph for reproducible builds.
- **`.cargo/config.toml`**: sets the default build target (so no `--target` flag is
  needed), wires in `flip-link` and the linker scripts, and defines the `cargo run` runner
  as `probe-rs run --chip STM32C562RET6 --chip-description-path chipdb/STM32C5_Series.yaml`.
- **`memory.x`**: the linker memory map — flash `512 KB @ 0x0800_0000`, RAM
  `128 KB @ 0x2000_0000` — straight from the STM32C562RE datasheet.
- **`build.rs`**: copies `memory.x` where the linker finds it and re-runs when it changes.
- **`chipdb/STM32C5_Series.yaml`**: a vendored probe-rs target description (flash algorithm
  + memory map) for the STM32C5 series, since probe-rs `0.32`'s built-in database stops at
  STM32C0. Sourced from ST's CMSIS DFP `STMicroelectronics.stm32c5xx_dfp` v2.1.0.

## Requirements

### Requirement: Project SHALL build reproducibly for the MCU target without a target flag

The project SHALL compile for `thumbv8m.main-none-eabihf` using only `cargo build`, with
the target, toolchain, and dependency graph fully pinned.

#### Scenario: Debug and release builds succeed with pinned inputs

- **GIVEN** a clean checkout with the pinned `rust-toolchain.toml` and committed `Cargo.lock`
- **WHEN** `cargo build` or `cargo build --release` is run
- **THEN** the firmware compiles for `thumbv8m.main-none-eabihf` without passing `--target`
- **AND** the resolved dependency graph matches the committed `Cargo.lock`

#### Scenario: Embassy is pinned to an exact commit

- **GIVEN** `Cargo.toml` references Embassy crates from git
- **WHEN** dependencies are resolved
- **THEN** each Embassy crate is pinned to an exact `rev` SHA, never a floating branch

### Requirement: Firmware SHALL flash and log with zero chip-database setup

`cargo run` SHALL flash the board and stream defmt logs using the vendored chip
description, requiring no CMSIS-pack download or `target-gen` step.

#### Scenario: cargo run flashes via the bundled chip description

- **GIVEN** `probe-rs` and `flip-link` are installed and the board is connected via ST-LINK
- **WHEN** `cargo run --release` is executed
- **THEN** the runner invokes `probe-rs run --chip STM32C562RET6 --chip-description-path chipdb/STM32C5_Series.yaml`
- **AND** the firmware is flashed and defmt logs stream to the terminal

### Requirement: Linker SHALL use the correct memory map and guard against stack overflow

The build SHALL place code and data per the STM32C562RE memory map and SHALL turn a stack
overflow into a hardware fault rather than silent corruption.

#### Scenario: Memory map is applied from memory.x

- **GIVEN** `memory.x` declares flash `512 KB @ 0x0800_0000` and RAM `128 KB @ 0x2000_0000`
- **WHEN** the linker runs
- **THEN** `build.rs` has made `memory.x` available and the regions are honored

#### Scenario: flip-link protects the stack

- **GIVEN** `.cargo/config.toml` wires `flip-link` as the linker
- **WHEN** the firmware's stack overflows at runtime
- **THEN** a hardware fault is triggered instead of silent memory corruption
