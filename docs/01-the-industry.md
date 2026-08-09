# 01 — The industry: automotive & safety-critical embedded

Before any code, understand the *world* this firmware lives in. The choices in this
repo (strict linting, pinned toolchains, no heap, no panics in the hot path) only make
sense once you know what "embedded" and "automotive" demand.

---

## 1. What is "embedded"?

An **embedded system** is a computer built into a larger device to do one dedicated job:
the engine controller in a car, the flight computer in a drone, the controller in a
washing machine or a pacemaker. Unlike your laptop, it usually has:

- **No operating system** (or a tiny real-time one) — your code *is* the whole system.
- **Tiny resources** — kilobytes of RAM, not gigabytes. This chip has **128 KB of RAM
  and 512 KB of flash**. Your browser tab uses more than that.
- **No screen, keyboard, or files** — I/O is pins, buses, sensors, and actuators.
- **Hard real-world deadlines** — if a brake controller is 50 ms late, that's not a lag
  spike, it's a crash.
- **Runs forever** — firmware boots when power is applied and must not leak memory,
  deadlock, or crash for *years*.

The chip at the center is a **microcontroller (MCU)**: a single package containing a CPU
core, RAM, flash storage, and **peripherals** (hardware blocks for GPIO, timers,
communication buses, etc.). See [05-the-hardware.md](05-the-hardware.md).

---

## 2. What makes *automotive* special

Cars are **safety-critical, harsh-environment, mass-produced, long-lived** systems.
That combination drives everything:

| Constraint | Consequence for firmware |
| ---------- | ------------------------ |
| A bug can kill someone | Correctness > cleverness. Undefined behavior is unacceptable. |
| −40 °C to +125 °C, vibration, electrical noise | Robust hardware handling; the bus (CAN) is designed for noise immunity. |
| Millions of units, cost-sensitive | Small, cheap MCUs → every byte of RAM/flash matters. |
| 15+ year service life, no easy patching | Determinism and reproducibility; you must be able to rebuild the *exact* firmware years later. |
| Many ECUs must cooperate in real time | A shared, prioritized, robust communication bus: **CAN / CAN FD** (see [06-can-and-canfd.md](06-can-and-canfd.md)). |

A modern car contains dozens to **~100+ ECUs** (Electronic Control Units) — small
computers for the engine, brakes, doors, lights, infotainment — all talking over CAN and
newer buses. This project simulates a single ECU: a heartbeat plus a CAN FD talker.

### Functional safety and ISO 26262

The automotive industry has a formal safety standard, **ISO 26262** ("Road vehicles —
Functional safety"). It defines **ASIL** levels (A→D, D being most critical) and demands
things like: freedom from interference between components, absence of undefined behavior,
traceability, and rigorous testing. You don't need to master it, but notice how this
repo's habits *mirror* those demands:

- **No undefined behavior** → Rust's safety guarantees + `#![deny(unsafe_code)]`.
- **Determinism / reproducibility** → pinned toolchain + committed `Cargo.lock`.
- **No silent failures** → no `unwrap()` in run loops; errors are logged, not fatal.
- **Freedom from interference** → tasks own their peripherals; no shared mutable global state.
- **Supply-chain integrity** → `cargo-deny` gates licenses, advisories, and sources.

These are explained in detail in [10-quality-gates.md](10-quality-gates.md).

---

## 3. Why Rust for this?

Historically, embedded firmware is written in **C** (and some C++). C is fast and close
to the metal, but it is famously easy to write memory bugs in: buffer overflows,
use-after-free, data races, null-pointer dereferences. In a safety-critical system those
are catastrophic. Studies from Microsoft and Google have repeatedly found that **~70% of
serious security bugs are memory-safety bugs** — exactly the class C makes easy.

**Rust** gives you C-level performance and control **without a garbage collector**, while
the compiler *proves* — at compile time — that whole categories of those bugs cannot
happen:

- **No use-after-free / double-free** — the ownership system tracks when values die.
- **No data races** — the borrow checker forbids aliased mutable access across threads.
- **No buffer overflows** — slices are bounds-checked (and the checks are cheap).
- **No null dereference** — there is no `null`; absence is modeled with `Option`.

Crucially, it does this with **zero runtime cost** and **no runtime/GC**, so it fits on a
128 KB MCU. That is the whole pitch: *the safety of a high-level language with the
footprint of C.* The next doc, [02-rust-for-beginners.md](02-rust-for-beginners.md),
shows how the language actually delivers that.

Rust is being adopted in automotive and safety contexts specifically for these reasons;
there is even ongoing work on qualified/certified Rust toolchains (e.g. Ferrocene, a
qualified Rust compiler for ISO 26262 / IEC 61508). This project isn't certified, but it
teaches the *mindset* that leads there.

---

## 4. Where this project sits

`rt-learn` is a **learning template**, not a shipping product. It deliberately adopts
industry practices so that the habits you build here transfer directly to real work:

- Real MCU, real CAN FD, real cross-compilation and flashing — not a simulator.
- The same tooling professionals use: `probe-rs`, `defmt`, `cargo-deny`, pinned toolchains.
- A "zero-warning" bar so you learn to write code that *passes review*, not just compiles.

> **Takeaway:** Everything strict about this repo is strict *on purpose*. It's teaching
> you the discipline that safety-critical embedded work requires, using the smallest
> possible real example.

**Next:** [02-rust-for-beginners.md](02-rust-for-beginners.md) — the language itself.
