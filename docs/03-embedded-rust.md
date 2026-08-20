# 03 — Embedded Rust: `no_std`, `no_main`, and bare metal

On your laptop, an operating system (OS) gives your program a `main()`, a heap, threads,
files, and a way to print. On this microcontroller **there is no OS**. Your firmware *is*
the only software running. This doc explains everything that changes because of that — and
decodes the cryptic first lines of [`src/main.rs`](../src/main.rs).

---

## 1. "Bare metal" — what actually runs

When you power the board, the CPU doesn't magically find your `main`. The Arm Cortex-M
boot sequence is fixed in silicon:

1. The CPU reads the **initial stack pointer** and the **reset vector** from the first two
   words of flash (at address `0x0800_0000`).
2. It jumps to the **reset handler**, which sets up RAM and then calls *your* entry point.

The crate `cortex-m-rt` ("runtime") provides that reset handler and the **vector table**
(the list of addresses the CPU jumps to for reset and each interrupt). Embassy's
`#[embassy_executor::main]` macro plugs your `async fn main` into it. So the real call
chain is:

```
power on → Cortex-M reset vector → cortex-m-rt reset handler → your main → Embassy executor
```

There is no `libc`, no loader, no shell. Just your `.elf` flashed into the chip.

---

## 2. `#![no_std]` — no standard library

Normal Rust programs link **`std`**, the standard library. `std` assumes an OS underneath:
it has a heap allocator, `Vec`/`String`, threads, files, networking, `println!`. None of
that exists here. So the very first line of `main.rs` is:

```rust
#![no_std]
```

This tells the compiler: **do not link `std`.** Instead you get **`core`** — the
OS-independent subset of the standard library that's always available: integers, slices,
`Option`, `Result`, `Iterator`, `match`, traits, etc. Everything in Doc 02 that didn't
need an allocator lives in `core`.

### What you lose (and how this repo copes)

| Missing from `std` | Why | This project's approach |
| ------------------ | --- | ----------------------- |
| Heap (`Box`, `Vec`, `String`) | No allocator by default | Use **fixed-size** data: `[u8; 2]`, stack buffers. No heap at all. |
| Threads | No OS scheduler | **`async` tasks** on the Embassy executor (Doc 04). |
| `println!` | No stdout/console | **`defmt`** logging over RTT (Doc 08). |
| Files, sockets | No filesystem/network | Talk to hardware directly: GPIO, timers, buses. |
| `std::error`, panics-with-unwind | No unwinder | `panic = "abort"` + `panic-probe`. |

Working without a heap is a *feature* here, not a hardship: no allocator means no
allocation failures, no fragmentation, and fully predictable memory use — exactly what a
safety-critical system wants. (See the "pass buffers by reference" best practice in the
[Embassy book](https://embassy.dev/book/).)

---

## 3. `#![no_main]` — no ordinary entry point

The second line:

```rust
#![no_main]
```

A normal Rust program has a `main()` with a specific signature that the OS runtime calls.
With no OS, that machinery doesn't apply — the *entry point* is defined by the vector
table instead. `#![no_main]` says "don't expect the standard `main`; the entry point is
declared some other way." Here that "other way" is the
`#[embassy_executor::main]` attribute on our `async fn main`, which expands into the real,
cortex-m-rt-compatible entry point and starts the executor. (Full detail in Doc 04.)

---

## 4. Panics on bare metal

In `std` Rust, a `panic!` unwinds the stack (running cleanup) and typically aborts the
thread. On bare metal there's **nowhere to unwind to** and no console to print to. Two
pieces handle this:

1. **A panic handler.** `#![no_std]` code must provide a function to run on panic. We don't
   write one by hand — the line `use panic_probe as _;` pulls in
   [`panic-probe`](https://crates.io/crates/panic-probe), which installs a handler that
   prints the panic message over `defmt` and then halts the core so a debugger can inspect
   it. (The `as _` means "link this crate for its side effects, I won't name it.")

2. **`panic = "abort"`.** In `Cargo.toml` both build profiles set this. It disables stack
   *unwinding* entirely (which needs code and tables we don't want on a tiny chip) — on
   panic the program just stops. Smaller, simpler, deterministic.

This is why Doc 02 stressed *not* using `.unwrap()` in run loops: a panic here isn't a
stack trace on your terminal, it's a **halted microcontroller**.

---

## 5. Cross-compilation and the target triple

You build on your laptop (probably x86-64 or Apple Silicon) but the code must run on an
Arm Cortex-M33. Producing code for a *different* CPU than the one you're building on is
**cross-compilation**. Rust identifies the destination with a **target triple**:

```
thumbv8m.main-none-eabihf
│         │    │    │
│         │    │    └─ hf: hardware floating-point (the M33F has an FPU)
│         │    └────── eabi: the Arm Embedded ABI (calling conventions)
│         └─────────── none: no operating system
└───────────────────── thumbv8m.main: Armv8-M "main" instruction set (Thumb), = Cortex-M33
```

- **`thumbv8m.main`** — the instruction set of the Cortex-M33 (Armv8-M Mainline, Thumb encoding).
- **`none`** — no OS. (Contrast a laptop target like `x86_64-unknown-linux-gnu`.)
- **`eabihf`** — Embedded ABI, **h**ardware **f**loat.

This target is set as the default in [`.cargo/config.toml`](../.cargo/config.toml) so you
can just type `cargo build` and it cross-compiles automatically. It's also listed in
[`rust-toolchain.toml`](../rust-toolchain.toml) so the toolchain installs the needed
pre-compiled `core` for it. More in [08-toolchain-build-flash.md](08-toolchain-build-flash.md).

---

## 6. Memory: flash vs RAM, and the linker

A microcontroller has two main memories, at **fixed addresses** wired into the silicon:

- **Flash** (non-volatile, keeps its contents without power) — your **code** and constants
  live here and run directly from it. On this chip: **512 KB at `0x0800_0000`**.
- **RAM** (volatile, lost on power-off) — your **variables, stack, and buffers** live here.
  On this chip: **128 KB at `0x2000_0000`**.

The **linker** is the tool that decides *where* each piece of your program goes in those
address ranges. It needs to know the map. That's what [`memory.x`](../memory.x) is:

```
MEMORY
{
    FLASH : ORIGIN = 0x08000000, LENGTH = 512K
    RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
```

Those numbers come straight from the STM32C562RE datasheet (see [05-the-hardware.md](05-the-hardware.md)).
`cortex-m-rt` ships a master linker script, `link.x`, that `INCLUDE`s this `memory.x` to
lay out the standard sections (`.text` = code, `.rodata` = constants, `.data`/`.bss` =
variables, `.vector_table`).

### How `memory.x` reaches the linker: `build.rs`

The linker only searches certain directories. [`build.rs`](../build.rs) is a **build
script** — a small Rust program Cargo compiles and runs *before* your firmware. Its whole
job is to copy `memory.x` into Cargo's `OUT_DIR` (which is on the linker's search path) and
tell Cargo to re-run if the memory map changes:

```rust
File::create(out_dir.join("memory.x"))?.write_all(include_bytes!("memory.x"))?;
println!("cargo:rustc-link-search={}", out_dir.display()); // add OUT_DIR to link path
println!("cargo:rerun-if-changed=memory.x");
```

So the flow is: `build.rs` places `memory.x` → `link.x` includes it → the linker positions
your code in flash and your data in RAM.

### `flip-link` — stack-overflow protection

The **stack** (where function calls and local variables live) grows *downward* from the top
of RAM. If it grows too far it can silently overwrite your global variables — a classic,
hard-to-debug embedded failure. [`.cargo/config.toml`](../.cargo/config.toml) uses
`flip-link` as the linker wrapper. It rearranges memory so the stack sits *below* your
static data; a stack overflow then runs off the end of RAM and triggers a **hardware fault**
immediately, instead of corrupting data. Cheap, deterministic safety — very much the
automotive mindset from Doc 01.

The other linker flags there (`-Tlink.x`, `-Tdefmt.x`, `--nmagic`) are explained in
[08-toolchain-build-flash.md](08-toolchain-build-flash.md).

---

## 7. Peripherals and the ownership trick

Beyond the CPU, the MCU has **peripherals** — hardware blocks like GPIO ports, timers, and
serial buses. Software controls them by reading/writing special memory addresses
called **registers** (memory-mapped I/O). Poking raw registers is error-prone, so Rust
embedded uses layers (detailed in Doc 04/05):

- **PAC** (Peripheral Access Crate) — typed, but low-level, register access.
- **HAL** (Hardware Abstraction Layer) — safe, ergonomic APIs (e.g. `Output::new` for a GPIO pin).

The clever part: `embassy_stm32::init()` returns a `Peripherals` struct that **owns every
peripheral exactly once**. When `main` does `Output::new(peripherals.PA5, …)`, pin PA5 is
*moved* out. You literally cannot configure the same pin twice — the borrow checker stops
you at compile time. Hardware resource conflicts become *compile errors*. That's Rust's
ownership system (Doc 02) applied to physical hardware.

---

## 8. Reading the top of `main.rs` with full understanding

```rust
#![no_std]        // no standard library — only `core`
#![no_main]       // entry point comes from the Embassy/cortex-m-rt vector table
#![deny(unsafe_code)]   // (lint gate, Doc 09) — forbid `unsafe` crate-wide

use defmt_rtt as _;     // link the RTT logging transport (side-effect only)
use panic_probe as _;   // link the panic handler (side-effect only)
```

None of this is boilerplate noise anymore: each line is a direct consequence of "there is
no operating system."

**Next:** [04-async-and-embassy.md](04-async-and-embassy.md) — how three tasks share one CPU.

---

### Where to go deeper (official)

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/) — the canonical bare-metal intro.
- [The Embedonomicon](https://docs.rust-embedded.org/embedonomicon/) — building a `no_std` binary from scratch, including the vector table and linking.
- [`cortex-m-rt` docs](https://docs.rs/cortex-m-rt/) — the runtime, reset handler, and `link.x`.
- [`core` API docs](https://doc.rust-lang.org/core/) — everything available without `std`.
