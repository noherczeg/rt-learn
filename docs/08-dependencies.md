# 08 — Dependencies: every crate, and why

This doc walks [`Cargo.toml`](../Cargo.toml) line by line: what each **crate** (Rust
package/library) is, why this project needs it, how it works, and which **feature flags**
are enabled and why. If you've read Docs 03–06 the *roles* will be familiar; here we make
them concrete.

---

## 0. Cargo vocabulary (30-second primer)

- **Crate** — a Rust library or binary. Dependencies are crates.
- **`Cargo.toml`** — the manifest: declares dependencies, features, and build profiles.
- **`Cargo.lock`** — the *exact* resolved versions of every crate (direct and transitive).
  Committed here so builds are reproducible (Doc 10).
- **Feature flags** — optional, named bits of a crate you switch on. E.g. `embassy-stm32`'s
  `stm32c562re` feature selects the chip. Features keep crates small: you compile only what
  you use.
- **Transitive dependency** — a crate pulled in by one of your dependencies, not by you
  directly (e.g. `bare-metal` via `cortex-m`).

---

## 1. The dependency graph, grouped by job

```mermaid
flowchart TD
    APP[rt-learn firmware] --> CM[cortex-m]
    APP --> CMRT[cortex-m-rt]
    APP --> DEFMT[defmt]
    APP --> DRTT[defmt-rtt]
    APP --> PP[panic-probe]
    APP --> EX[embassy-executor]
    APP --> TIME[embassy-time]
    APP --> STM[embassy-stm32]
    STM --> METAPAC[stm32-metapac\n(generated PAC)]
    STM --> CM
    EX --> CM
    subgraph CORE[Cortex-M core support]
      CM
      CMRT
    end
    subgraph LOG[Logging & panic]
      DEFMT
      DRTT
      PP
    end
    subgraph EMB[Embassy async + HAL]
      EX
      TIME
      STM
    end
```

Four jobs: **core support**, **logging/panic**, **Embassy async+HAL**, and (implicitly) the
**generated chip PAC** that Embassy pulls in.

---

## 2. Cortex-M core support

### `cortex-m = { version = "0.7.7", features = ["inline-asm", "critical-section-single-core"] }`

Low-level access to the Cortex-M **core** itself (not the STM32 peripherals): the NVIC,
system control block, `WFE`/`WFI` sleep instructions, and **critical sections** (briefly
disabling interrupts to touch shared state atomically). Everything Cortex-M in the stack
sits on this.

- **`inline-asm`** — use inline assembly for the core intrinsics (smaller/faster; fine on
  modern toolchains).
- **`critical-section-single-core`** — provides a critical-section implementation valid on
  a **single-core** chip (this one). The `critical-section` crate is a standard interface
  many embedded crates use to guard shared data; something must supply the actual
  implementation, and on a single core "disable interrupts briefly" is correct and cheap.

### `cortex-m-rt = "0.7.5"`

The Cortex-M **runtime** (Doc 03): the reset handler, the **vector table**, and the master
linker script **`link.x`** that consumes our `memory.x`. It's what turns a bare chip into
"something that can call your entry point." Embassy's `#[main]` builds on top of it.

> These two also pull in the transitive crate `bare-metal`, which is flagged as
> unmaintained (RUSTSEC-2026-0110). There's no fix until the whole cortex-m 0.7 line
> migrates off it, so `deny.toml` documents and ignores that specific advisory (Doc 10).

---

## 3. Logging and panic handling

Recall from Doc 03 that there's no `println!` and no default panic behavior on bare metal.
Three crates fill the gap. All are part of the **Knurling** project by Ferrous Systems.

### `defmt = "0.3.8"`

**`defmt`** ("deferred formatting") is a super-efficient logging framework for tiny devices.
Per the [defmt book](https://defmt.ferrous-systems.com/): instead of formatting `"255"` on
the MCU and sending a string, it **interns** format strings at compile time and sends only
tiny **indices + raw bytes**; your *laptop* reconstructs the text. This means logging costs
the MCU almost nothing (RAM, flash, and CPU). It gives you `info!`, `warn!`, `error!`,
`debug!`, `trace!` and the `{:?}` formatting you see throughout the code.

### `defmt-rtt = "0.4.1"`

The **transport** that carries `defmt` frames off the chip using SEGGER's **RTT** (Real-Time
Transfer) — a mechanism that shuttles bytes through the debug (SWD) connection via a small
RAM ring buffer, with essentially no timing impact. `probe-rs` reads the other end. The
line `use defmt_rtt as _;` in `main.rs` links it for its side effects (registering the
global logger). RTT is the natural choice with `probe-rs` because support is built in.

### `panic-probe = { version = "0.3.2", features = ["print-defmt"] }`

Supplies the required `#[panic_handler]` (Doc 03). On panic it prints the panic info and
halts so a debugger can inspect the state.

- **`print-defmt`** — route the panic message through `defmt` (so it appears in your normal
  log stream) rather than a separate mechanism. `use panic_probe as _;` links it.

Together these three are why Doc 09 can show you readable logs like `CAN TX: id=0x100 seq=3`
streaming to your terminal over a single USB cable.

---

## 4. Embassy: async executor, time, and the HAL

All three Embassy crates are pulled from **git at an exact commit**, not crates.io. This is
the single most important thing to understand about this manifest — §5 explains why.

### `embassy-executor = "0.10"` (git) — features `platform-cortex-m`, `executor-thread`, `defmt`

The async scheduler and the `#[embassy_executor::main]` / `#[task]` macros (Doc 04).

- **`platform-cortex-m`** — the Cortex-M platform implementation (uses `WFE`/`SEV`, NVIC).
  Renamed from `arch-cortex-m` in 0.10; without it, `#[main]` literally doesn't exist and
  you get "could not find `main`" (Embassy FAQ).
- **`executor-thread`** — the thread-mode executor (the normal single-priority one that
  sleeps the core when idle).
- **`defmt`** — let the executor emit `defmt` logs.

### `embassy-time = "0.5"` (git) — features `defmt`, `defmt-timestamp-uptime`, `tick-hz-32_768`

Time primitives: `Timer`, `Duration`, `Instant`. Backed by a hardware timer chosen by the
HAL (see `time-driver-any` below).

- **`defmt`** — logging integration.
- **`defmt-timestamp-uptime`** — prefix each log line with the device uptime (handy for
  seeing task interleaving).
- **`tick-hz-32_768`** — run the time base at 32,768 Hz (Doc 05): low power, ~30 µs
  resolution, a classic choice. The HAL's time driver must support the chosen rate.

### `embassy-stm32 = "0.6"` (git) — features `defmt`, `stm32c562re`, `time-driver-any`, `exti`, `unstable-pac`

The **HAL** (Doc 04/05): safe drivers for GPIO (`Output`), FDCAN (`CanConfigurator`,
`CanTx`, `CanRx`, `Frame`), clocks (RCC), interrupts (`bind_interrupts!`), and more.

- **`stm32c562re`** — **selects the chip.** This flag makes the HAL pull the right register
  definitions from `stm32-metapac` and enable the correct peripheral versions (Doc 05). It
  **must match** the target and the `probe-rs --chip`. This exact feature is missing from
  crates.io `embassy-stm32` 0.6.0 — the reason for the git dependency (§5).
- **`time-driver-any`** — let the HAL pick any suitable hardware timer to drive
  `embassy-time`. Without a time driver you'd get `undefined symbol: _embassy_time_now`.
- **`exti`** — enable the **EXTI** (external interrupt/event) controller support, for
  interrupt-driven GPIO (e.g. a button via `ExtiInput` — a natural next exercise).
- **`unstable-pac`** — expose the raw PAC for functionality not yet wrapped by the HAL.
  "Unstable" = its API may change; acceptable for a pinned template.
- **`defmt`** — logging integration across the HAL.

`embassy-stm32` transitively pulls **`stm32-metapac`**, the auto-generated PAC (Doc 05),
which itself comes from the `stm32-data-generated` git repo — another source `deny.toml`
must trust.

---

## 5. Why Embassy comes from git (pinned by `rev`)

The manifest pins all three Embassy crates to one commit:

```toml
embassy-stm32 = { version = "0.6", git = "https://github.com/embassy-rs/embassy",
    rev = "5b878d927dbfc0422f1294dc02ed47221f13ac1f", features = [ … "stm32c562re" … ] }
```

The reasoning (also in the manifest comments and repo memory):

1. **The chip is newer than the last release.** crates.io `embassy-stm32` 0.6.0 predates the
   STM32C5 and does **not** expose `stm32c562re`. Chip support lives on Embassy's `main`.
2. **A moving branch would break reproducibility.** So instead of tracking `main`, it pins an
   exact **`rev`** (commit SHA). Combined with the committed `Cargo.lock`, every build —
   yours, a teammate's, CI's, one years from now — is **byte-for-byte identical** (Doc 01/10).
3. **All three Embassy crates share the same `rev`.** Mixing versions (some from git, some
   from crates.io) triggers the infamous `Only one package … may specify the same links
   value` error (Embassy FAQ). One source, one revision — consistent.
4. **Explicit `version = "…"` is kept** alongside the git `rev` so that `cargo-deny`'s
   `wildcards = "deny"` policy passes (no `*` version requirements) (Doc 10).

> **To upgrade Embassy:** bump the `rev` *deliberately* (never a floating branch), update
> `Cargo.lock`, and re-run the whole gate (fmt/clippy/build/deny). Because `main` ships
> edition-2024 manifests, the pinned toolchain must be recent enough to parse them — which
> is exactly why `rust-toolchain.toml` isn't on an ancient version (Doc 09).

---

## 6. Build profiles (also in `Cargo.toml`)

Not dependencies, but part of the same file — they tune the compiler. Both profiles favor
small, predictable firmware:

```toml
[profile.dev]                 [profile.release]
opt-level = 1                 opt-level = "s"     # optimize for size
debug = 2                     debug = 2           # keep debuginfo (not flashed)
lto = "thin"                  lto = "fat"         # link-time optimization
codegen-units = 1             codegen-units = 1   # 1 unit = better opt, reproducible
panic = "abort"               panic = "abort"     # no unwinding on bare metal (Doc 03)
overflow-checks = true        overflow-checks = true  # trap arithmetic overflow
```

Why these matter here:

- **`opt-level`** — even *debug* builds use `opt-level = 1` because fully-unoptimized
  firmware can be too big/slow to behave sanely on a small MCU. Release optimizes for **size**
  (`"s"`) on flash-constrained silicon.
- **`lto` (Link-Time Optimization)** + **`codegen-units = 1`** — let the optimizer see the
  whole program at once → smaller, faster, and *deterministic* output. (The Embassy FAQ
  recommends exactly this for size.)
- **`debug = 2`** — full debug info is kept for on-target debugging and `defmt`; it lives in
  the `.elf` on your PC and is **not** flashed to the chip, so it costs no device flash.
- **`panic = "abort"`** — no stack unwinder on bare metal (Doc 03).
- **`overflow-checks = true`** — arithmetic overflow **panics** instead of silently wrapping,
  even in release. This is a *safety-over-speed* choice (Doc 01) and is precisely why
  `tx_task` uses `wrapping_add` for its intentional 255→0 counter (Doc 06): "wrapping here
  is deliberate," everywhere else an accidental overflow is caught.

**Next:** [09-toolchain-build-flash.md](09-toolchain-build-flash.md) — building & running it.

---

### Where to go deeper (official)

- [`cortex-m`](https://docs.rs/cortex-m/) · [`cortex-m-rt`](https://docs.rs/cortex-m-rt/)
- [defmt book](https://defmt.ferrous-systems.com/) · [`defmt-rtt`](https://docs.rs/defmt-rtt/) · [`panic-probe`](https://docs.rs/panic-probe/)
- [Embassy book](https://embassy.dev/book/) · [`embassy-executor`](https://docs.rs/embassy-executor/) · [`embassy-time`](https://docs.rs/embassy-time/) · [`embassy-stm32`](https://docs.embassy.dev/embassy-stm32/)
- [The Cargo Book — dependencies, features, profiles](https://doc.rust-lang.org/cargo/)
