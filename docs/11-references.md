# 11 — References (official docs)

The authoritative sources behind these docs. When embedded Rust moves and a detail here
drifts, **these win**. Grouped by topic; each is the primary/official source.

> Reproducibility note: this project pins exact versions (`rust-toolchain.toml` +
> `Cargo.lock` + Embassy git `rev`). "latest" links below may describe newer APIs than this
> repo uses — cross-check against the pinned versions in [`Cargo.toml`](../Cargo.toml).

---

## The Rust language

- **The Rust Programming Language ("The Book")** — https://doc.rust-lang.org/book/
- **Rust by Example** — https://doc.rust-lang.org/rust-by-example/
- **`core` API reference** (the `no_std` subset) — https://doc.rust-lang.org/core/
- **`std` API reference** — https://doc.rust-lang.org/std/
- **Async Rust chapter** — https://doc.rust-lang.org/book/ch17-00-async-await.html
- **The Rust Reference** (language spec-ish) — https://doc.rust-lang.org/reference/
- **The Cargo Book** (manifest, features, profiles, config) — https://doc.rust-lang.org/cargo/
- **The rustup Book** (toolchains) — https://rust-lang.github.io/rustup/

## Embedded Rust (bare metal)

- **The Embedded Rust Book** — https://docs.rust-embedded.org/book/
- **The Embedonomicon** (build a `no_std` binary from scratch) — https://docs.rust-embedded.org/embedonomicon/
- **`cortex-m`** — https://docs.rs/cortex-m/
- **`cortex-m-rt`** (runtime, reset, `link.x`) — https://docs.rs/cortex-m-rt/
- **`critical-section`** — https://docs.rs/critical-section/
- **Awesome Embedded Rust** (ecosystem index) — https://github.com/rust-embedded/awesome-embedded-rust

## Embassy (async framework + HAL)

- **Embassy Book** — https://embassy.dev/book/
- **Embassy API docs (all crates)** — https://docs.embassy.dev/
- **`embassy-executor`** — https://docs.rs/embassy-executor/
- **`embassy-time`** — https://docs.rs/embassy-time/
- **`embassy-stm32`** — https://docs.embassy.dev/embassy-stm32/
- **`embassy-sync`** (channels/mutexes) — https://docs.rs/embassy-sync/
- **Embassy source & examples** — https://github.com/embassy-rs/embassy
- **`stm32-data` / metapac** (how per-chip support is generated) — https://github.com/embassy-rs/stm32-data

## Logging, panic, flashing (Knurling / probe-rs)

- **defmt Book** — https://defmt.ferrous-systems.com/
- **`defmt`** — https://docs.rs/defmt/
- **`defmt-rtt`** — https://docs.rs/defmt-rtt/
- **`panic-probe`** — https://docs.rs/panic-probe/
- **probe-rs documentation** — https://probe.rs/docs/
- **flip-link** (stack-overflow protection) — https://github.com/knurling-rs/flip-link
- **Knurling project** — https://knurling.ferrous-systems.com/

## Quality gates & supply chain

- **Clippy** (lints + `clippy.toml`) — https://doc.rust-lang.org/clippy/
- **rustfmt** (formatting options) — https://rust-lang.github.io/rustfmt/
- **cargo-deny Book** — https://embarkstudios.github.io/cargo-deny/
- **RustSec advisory database** — https://rustsec.org/
- **`Cargo.lock` vs `Cargo.toml` / reproducibility** — https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html

## The hardware (Arm + ST)

- **Arm Cortex-M33** — https://developer.arm.com/Processors/Cortex-M33
- **Armv8-M architecture** — https://developer.arm.com/documentation/ddi0553/latest/
- **STM32C5 series (STM32C562RE)** — https://www.st.com/en/microcontrollers-microprocessors/stm32c5-series.html
- **NUCLEO boards (ST evaluation tools)** — https://www.st.com/en/evaluation-tools/
  (find the NUCLEO-C562RE page for the schematic, user manual, and pinout — the source of
  truth for the pin assignments to verify in Doc 05)

## Automotive functional safety

- **ISO 26262** (road-vehicle functional safety) — ISO standard (paywalled); overviews
  widely available.
- **Ferrocene** (qualified Rust compiler for ISO 26262 / IEC 61508) — https://ferrous-systems.com/ferrocene/

---

*If you add a dependency or change a version, update [07-dependencies.md](07-dependencies.md)
and, if the behavior changes, the relevant topic doc — and re-verify against the source
above.*
