# Quality Gates Specification

## Purpose

The project SHALL enforce a "production-grade, zero-warning" bar: the firmware crate
forbids `unsafe`, compiles clean under strict Clippy lints, is formatted consistently,
and passes a supply-chain audit. CI SHALL run the whole gate on every push and PR so the
`master` branch stays green and builds stay reproducible.

## Architecture

- **`src/main.rs` lint attributes**: crate-wide gate — `#![deny(unsafe_code)]`,
  `#![deny(clippy::all)]`, `#![warn(clippy::pedantic)]`, `#![warn(clippy::nursery)]`,
  `#![warn(clippy::cargo)]`. Two documented, justified allows scope out lints that do not
  apply to this executor/dependency model: `clippy::multiple_crate_versions` (transitive
  Embassy/PAC/HAL duplicates, still surfaced by `cargo deny`) and `clippy::future_not_send`
  (the single-core cooperative executor's task futures are intentionally `!Send`).
- **`rustfmt.toml`**: formatting rules (100-column width, spaces), enforced by
  `cargo fmt --all --check`.
- **`clippy.toml`**: tunes Clippy so its strictest lints do not false-positive on the
  project's domain words.
- **`deny.toml`**: `cargo-deny` supply-chain policy — permissive licenses only, security
  advisories checked, and only trusted sources (crates.io plus the pinned Embassy git repos).
- **`.github/workflows/ci.yml`**: runs the gate on every push/PR to `master` in order —
  **format → lint → build → license & advisory check**.

## Requirements

### Requirement: Firmware crate SHALL contain zero unsafe code

The crate SHALL forbid `unsafe` with no exceptions, enforced at compile time.

#### Scenario: unsafe code fails the build

- **GIVEN** `src/main.rs` declares `#![deny(unsafe_code)]`
- **WHEN** any `unsafe` block or function is introduced in the crate
- **THEN** the build fails

### Requirement: Code SHALL pass strict linting with zero warnings

The crate SHALL compile clean under the strict Clippy lint set, with only the two
documented, justified allows.

#### Scenario: clippy runs clean as an error gate

- **GIVEN** the crate-wide Clippy lint attributes are in effect
- **WHEN** `cargo clippy --all-targets -- -D warnings` is run
- **THEN** it completes with no warnings or errors

#### Scenario: Justified allows are documented

- **GIVEN** `clippy::multiple_crate_versions` and `clippy::future_not_send` are allowed
- **WHEN** the source is reviewed
- **THEN** each allow carries an inline comment explaining why it is safe for this model

### Requirement: Code SHALL be consistently formatted

The code SHALL conform to `rustfmt.toml` and be verifiable in check mode.

#### Scenario: Formatting check passes

- **GIVEN** the formatting rules in `rustfmt.toml`
- **WHEN** `cargo fmt --all --check` is run
- **THEN** it reports no formatting differences

### Requirement: Dependencies SHALL pass a supply-chain audit

`cargo-deny` SHALL enforce the license, advisory, ban, and source policy in `deny.toml`.

#### Scenario: cargo deny passes under policy

- **GIVEN** the policy in `deny.toml`
- **WHEN** `cargo deny check` is run
- **THEN** only permissive-licensed crates from trusted sources are present
- **AND** no unresolved security advisories are reported

### Requirement: CI SHALL run the full gate on every push and PR

CI SHALL execute format, lint, build, and supply-chain checks on every push/PR to
`master`, keeping the branch green.

#### Scenario: CI enforces the gate in order

- **GIVEN** `.github/workflows/ci.yml` is configured
- **WHEN** a push or pull request targets `master`
- **THEN** the pipeline runs format → lint → build → license & advisory check
- **AND** any failing stage fails the pipeline
