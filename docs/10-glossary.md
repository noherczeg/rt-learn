# 10 — Glossary

Every acronym and term used in this repo and these docs, defined in plain language. The
doc number in **See** points to where it's explained in context.

---

## A–C

- **ABI (Application Binary Interface)** — the low-level contract for how functions pass
  arguments and how data is laid out. The `eabihf` in the target triple is Arm's embedded
  ABI with hardware float. *See Doc 03.*
- **`async` / `await`** — Rust keywords for cooperative concurrency. An `async fn` returns a
  *future*; `.await` pauses until it's ready, letting other tasks run. *See Doc 04.*
- **ASIL** — Automotive Safety Integrity Level (A–D), the risk classification in ISO 26262.
  *See Doc 01.*
- **Bare metal** — running with no operating system; your firmware is the only software.
  *See Doc 03.*
- **`bind_interrupts!`** — Embassy macro that wires hardware interrupt vectors to the HAL's
  handlers. Emits `unsafe`, so it's fenced in a local module when used. *See Doc 04.*
- **Borrow checker** — the compiler pass that enforces "many readers XOR one writer,"
  guaranteeing no data races. *See Doc 02.*
- **`bss` / `data`** — linker sections for zero-initialized / initialized global variables
  (in RAM). *See Doc 03.*
- **Crate** — a Rust library or binary package. *See Doc 07.*
- **`core`** — the OS-independent subset of Rust's standard library, available under
  `no_std`. *See Doc 03.*
- **Cortex-M / M33** — Arm's microcontroller CPU core family; the M33 is the Armv8-M core in
  this chip. *See Doc 05.*
- **Critical section** — briefly disabling interrupts to touch shared state atomically.
  *See Doc 07.*
- **Cross-compilation** — building on one CPU (your laptop) for a different CPU (the MCU).
  *See Doc 03.*

## D–H

- **`defmt`** — "deferred formatting," an ultra-efficient logging framework that sends
  interned IDs, not strings. *See Doc 07.*
- **`defmt-rtt`** — the transport that carries `defmt` frames off-chip over RTT. *See Doc 07.*
- **`deny` / `warn` / `allow`** — lint levels: hard error / warning / permitted. *See Doc 09.*
- **DMA (Direct Memory Access)** — hardware that moves data without CPU copying. *See Doc 05.*
- **ECU (Electronic Control Unit)** — a small computer/node in a vehicle. *See Doc 01.*
- **ECC (Error-Correcting Code)** — memory that detects/corrects bit errors. *See Doc 05.*
- **ELF** — the executable file format the build produces; holds code + layout + debug info.
  *See Doc 08.*
- **Executor** — the async scheduler that polls futures and sleeps the core when idle.
  *See Doc 04.*
- **EXTI** — STM32's external interrupt/event controller (e.g. for button GPIOs). *See Doc 07.*
- **`flip-link`** — linker wrapper that places the stack so overflow faults instead of
  corrupting data. *See Doc 03/08.*
- **FPU (Floating-Point Unit)** — hardware for float math; the "F" in Cortex-M33**F**.
  *See Doc 05.*
- **Future** — a value representing pausable/resumable work; what an `async fn` returns.
  *See Doc 04.*
- **GPIO (General-Purpose Input/Output)** — configurable digital pins. *See Doc 05.*
- **HAL (Hardware Abstraction Layer)** — safe, ergonomic drivers over raw registers; here
  `embassy-stm32`. *See Doc 04/05.*
- **Heap** — dynamically allocated memory. **Not used** in this firmware (no allocator).
  *See Doc 03.*

## I–P

- **Interrupt** — a hardware signal that makes the CPU jump to a handler; Embassy's wake
  source. *See Doc 04/05.*
- **ISO 26262** — road-vehicle functional-safety standard. *See Doc 01.*
- **Lifetime / `'static`** — the compiler's tracking of how long a reference is valid;
  `'static` = whole program. *See Doc 02.*
- **Lint** — an automated code-quality check (Clippy/compiler). *See Doc 09.*
- **LTO (Link-Time Optimization)** — optimizing across the whole program at link time.
  *See Doc 07.*
- **`memory.x`** — the linker's map of flash/RAM addresses and sizes. *See Doc 03/05.*
- **MMIO (Memory-Mapped I/O)** — controlling hardware by reading/writing special addresses
  (registers). *See Doc 05.*
- **Move** — ownership transfer; the source variable becomes invalid. *See Doc 02.*
- **MCU (Microcontroller Unit)** — a single chip with CPU + memory + peripherals. *See Doc 01.*
- **`no_std` / `no_main`** — crate attributes: no standard library / no ordinary entry point.
  *See Doc 03.*
- **NVIC (Nested Vectored Interrupt Controller)** — Cortex-M hardware that dispatches
  interrupts. *See Doc 05.*
- **`Option<T>`** — "a value or nothing"; Rust's null-free absence type. *See Doc 02.*
- **Ownership** — Rust's rule that every value has one owner, freed when the owner ends.
  *See Doc 02.*
- **PAC (Peripheral Access Crate)** — auto-generated typed register access; here
  `stm32-metapac`. *See Doc 04/05.*
- **`panic = "abort"`** — on panic, stop immediately (no stack unwinding). *See Doc 03.*
- **`panic-probe`** — supplies the `#[panic_handler]` that logs then halts. *See Doc 07.*
- **PLL (Phase-Locked Loop)** — clock circuit that multiplies a frequency up (e.g. to
  144 MHz). *See Doc 05.*
- **`probe-rs`** — tool that flashes firmware and streams logs via a debug probe. *See Doc 08.*

## Q–Z

- **RCC (Reset and Clock Control)** — the STM32 peripheral that generates/distributes clocks.
  *See Doc 05.*
- **Register** — a hardware control/status location accessed via MMIO. *See Doc 05.*
- **`Result<T, E>`** — Rust's success-or-error return type; no exceptions. *See Doc 02.*
- **RTIC / RTOS** — real-time frameworks; Embassy is an async alternative to a traditional
  RTOS. *See Doc 04.*
- **RTT (Real-Time Transfer)** — SEGGER protocol moving bytes over the debug link via a RAM
  buffer; carries `defmt`. *See Doc 07/08.*
- **RustSec** — the community vulnerability database `cargo-deny` audits against. *See Doc 09.*
- **rustfmt / Clippy** — the official formatter / linter. *See Doc 09.*
- **Slice (`&[u8]`)** — a borrowed view into a run of elements. *See Doc 02.*
- **Spawner** — Embassy handle used to launch tasks. *See Doc 04.*
- **Stack** — memory for function calls and locals; grows downward, protected by flip-link.
  *See Doc 03.*
- **`std`** — the full standard library (needs an OS); unavailable here. *See Doc 03.*
- **SWD (Serial Wire Debug)** — Arm's 2-pin debug protocol used by the ST-LINK. *See Doc 08.*
- **ST-LINK** — the on-board debugger chip on the Nucleo board. *See Doc 05/08.*
- **Target triple** — string naming the build destination, e.g.
  `thumbv8m.main-none-eabihf`. *See Doc 03.*
- **Task** — an `async fn` marked `#[embassy_executor::task]`, scheduled by the executor.
  *See Doc 04.*
- **Thumb (thumbv8m)** — the compact Cortex-M instruction encoding. *See Doc 03/05.*
- **Trait** — a Rust interface: a set of methods a type promises to implement. *See Doc 02.*
- **TrustZone-M** — Armv8-M security extension (secure/non-secure worlds). *See Doc 05.*
- **`unsafe`** — Rust keyword disabling some compile-time checks; forbidden crate-wide here
  with zero exceptions (the crate has no `unsafe`). *See Doc 09.*
- **`unwrap()` / `unwrap!`** — extract a value or panic; avoided in run loops, used only at
  boot. *See Doc 02.*
- **Vector table** — the array of addresses the CPU jumps to for reset and each interrupt.
  *See Doc 03/05.*
- **`WFE` / `WFI`** — Cortex-M "wait for event/interrupt" sleep instructions the executor
  uses when idle. *See Doc 04/05.*
