# 07 — Architecture: why the repo looks like this

Now that you understand the language (Doc 02), bare metal (Doc 03), async/Embassy (Doc 04),
the chip (Doc 05), and CAN (Doc 06), this doc connects it all: **what each file is, why it
exists, and how they interact at build time and run time.**

---

## 1. The guiding principles

Every structural choice serves one of these goals (all rooted in Doc 01):

1. **Safety first** — the compiler and tooling should catch mistakes, not the car.
2. **Determinism / reproducibility** — the same source must always produce the same firmware.
3. **Separation of concerns** — each file/module has one clear job.
4. **No surprises at runtime** — no heap, no panics in hot paths, no silent failures.

---

## 2. The file map

```
rt-learn/
├── .cargo/config.toml       # HOW to build & flash (target, linker, probe-rs runner)
├── .github/workflows/ci.yml # CI gate: fmt → clippy → build → cargo-deny
├── .vscode/                 # editor: rust-analyzer target + clippy on save
├── src/
│   ├── main.rs              # entry point, lint gate, wiring, heartbeat task
│   └── can_fd.rs            # FDCAN driver module: init + tx_task + rx_task
├── build.rs                 # build script: feeds memory.x to the linker
├── memory.x                 # chip memory map (flash/RAM addresses & sizes)
├── Cargo.toml               # dependencies + build profiles + metadata
├── Cargo.lock               # exact resolved dependency graph (committed!)
├── rust-toolchain.toml      # pins the exact compiler + components + target
├── rustfmt.toml             # formatting rules
├── clippy.toml              # linter tuning
├── deny.toml                # supply-chain policy (licenses/advisories/sources)
└── README.md                # human-facing overview
```

Group them by *purpose*:

| Purpose | Files | Covered in |
| ------- | ----- | ---------- |
| **The firmware** | `src/main.rs`, `src/can_fd.rs` | §3–4 here |
| **Build & memory** | `.cargo/config.toml`, `build.rs`, `memory.x`, `Cargo.toml` | Doc 03, Doc 09 |
| **Reproducibility** | `Cargo.lock`, `rust-toolchain.toml` | Doc 09, Doc 10 |
| **Quality gates** | `rustfmt.toml`, `clippy.toml`, `deny.toml`, `ci.yml` | Doc 10 |
| **Editor** | `.vscode/` | Doc 09 |

---

## 3. Why two source files (not one, not ten)

The firmware is split into a **conductor** and a **driver**:

- **`src/main.rs` — the conductor.** It owns the crate-wide rules and the *wiring*. It
  doesn't do much work itself; it initializes hardware and starts the players. Keeping the
  entry point thin makes the system's shape obvious at a glance.
- **`src/can_fd.rs` — the FDCAN driver.** All CAN logic is isolated in one module so it's
  reusable, testable, and the one place `unsafe` is (locally) permitted. If you later add,
  say, a `sensors.rs`, it slots in the same way.

This mirrors how real firmware grows: a small top-level orchestrator plus one module per
subsystem. Two files is the smallest example that still demonstrates the pattern.

### `main.rs` responsibilities, in order

1. **Crate-wide attributes** (Doc 03 + Doc 10): `#![no_std]`, `#![no_main]`, and the strict
   lint gate (`deny(unsafe_code)`, clippy levels, plus the few justified `allow`s). These
   apply to *every* module, including `can_fd.rs`.
2. **`mod can_fd;`** — pulls the driver module into the crate.
3. **`async fn main`** — `embassy_stm32::init(...)`, create the LED, then **spawn** the
   three tasks via the `Spawner` (Doc 04).
4. **`heartbeat` task** — the simplest possible Embassy task; a good first thing to study.

### `can_fd.rs` responsibilities

`mod irqs` + `bind_interrupts!` (the fenced `unsafe`), `init` (configure + split), and the
`tx_task`/`rx_task` async loops — all detailed in Doc 06.

---

## 4. How the pieces interact — two views

### Build-time (how source becomes firmware)

```mermaid
flowchart TD
    TC[rust-toolchain.toml\npins compiler+target] --> CARGO[cargo build]
    LOCK[Cargo.lock\nexact deps] --> CARGO
    TOML[Cargo.toml\ndeps+profiles] --> CARGO
    CARGO --> BUILD[build.rs runs first]
    BUILD -->|copies| MEMX[memory.x → OUT_DIR]
    CONFIG[.cargo/config.toml\ntarget+rustflags] --> LINK
    CARGO --> COMPILE[compile src/*.rs for thumbv8m]
    COMPILE --> LINK[link: flip-link + link.x + defmt.x]
    MEMX --> LINK
    LINK --> ELF[firmware .elf]
```

`Cargo.toml` + `Cargo.lock` + `rust-toolchain.toml` decide *exactly* what compiles;
`build.rs` + `memory.x` + `.cargo/config.toml` decide *how it's laid out and linked*
(Doc 03, Doc 09). Result: a deterministic `.elf`.

### Run-time (how firmware behaves on the chip)

```mermaid
flowchart TD
    RESET([reset]) --> RT[cortex-m-rt reset handler]
    RT --> M["#[embassy_executor::main] → executor + main task"]
    M --> INIT[embassy_stm32::init → clocks + Peripherals]
    INIT --> HB[spawn heartbeat PA5]
    INIT --> CANI["can_fd::init(FDCAN1,PB8,PB9) → split"]
    CANI --> TXt[spawn tx_task CanTx]
    CANI --> RXt[spawn rx_task CanRx]
    subgraph EX[Embassy executor - single core]
      HB --> HB
      TXt --> TXt
      RXt --> RXt
    end
    EX -->|logs| DEFMT[defmt over RTT → your terminal]
```

(Detailed in Doc 04/06.)

---

## 5. Why ownership drives the architecture

A subtle but important point: the structure isn't just style — it's **enforced by Rust's
ownership** (Doc 02/03):

- `embassy_stm32::init` yields a `Peripherals` struct owning every peripheral once.
- `main` *moves* `PA5` into `heartbeat`, and `FDCAN1`/`PB8`/`PB9` into `can_fd::init`.
- `can.split()` produces `CanTx` and `CanRx`, each *moved* into its own task.

So each task **exclusively owns** its hardware. No two tasks can touch the same peripheral,
which means no locks are needed and hardware-conflict bugs become *compile errors*. The
file/task boundaries and the ownership boundaries are the same boundaries. That's the
"freedom from interference" idea from Doc 01, made structural.

---

## 6. Why the config files are first-class citizens

In many hobby projects the "real code" is `main.rs` and everything else is noise. Here the
config files are *the point* — they're what makes the firmware production-grade:

| File | The single question it answers |
| ---- | ------------------------------ |
| `rust-toolchain.toml` | *Which exact compiler builds this?* |
| `Cargo.lock` | *Which exact dependency versions?* |
| `Cargo.toml` (profiles) | *How is it optimized/linked?* |
| `memory.x` | *Where does code/data live on the chip?* |
| `.cargo/config.toml` | *What target, linker, and run command?* |
| `clippy.toml` / `rustfmt.toml` | *What does "clean code" mean here?* |
| `deny.toml` | *Which dependencies/licenses are allowed?* |
| `ci.yml` | *What must pass before code is accepted?* |

Together they answer: *"Can a stranger, years from now, rebuild the byte-identical firmware
and trust it?"* — the reproducibility goal from Doc 01. Each is dissected in Docs 08–10.

**Next:** [08-dependencies.md](08-dependencies.md) — every crate, and why it's there.
