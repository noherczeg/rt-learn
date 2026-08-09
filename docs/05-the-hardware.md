# 05 — The hardware: Arm Cortex-M33 and the STM32C562RE

Software choices only make sense once you know the silicon they run on. This doc explains
the board, the chip, the CPU core, and the peripheral concepts the firmware touches — GPIO,
clocks, timers, interrupts, and FDCAN.

> ⚠️ **Verify before you flash.** The STM32C5 series is very new. Pin assumptions in this
> repo (LED on `PA5`, FDCAN1 on `PB8`/`PB9`) follow common Nucleo-64 conventions but
> **must be confirmed** against the NUCLEO-C562RE schematic and your wiring. See §7.

---

## 1. The board vs. the chip

- **NUCLEO-C562RE** — the *development board*. It's a PCB from ST with the MCU, a USB
  connector, headers to reach the pins, a user LED and button, and — importantly — an
  **on-board ST-LINK debugger**. The ST-LINK is a second small chip that lets your laptop
  flash firmware and stream logs over one USB cable, with no external probe needed. This is
  what [`probe-rs`](09-toolchain-build-flash.md) talks to.
- **STM32C562RE** — the *microcontroller* on that board: the actual CPU + memory +
  peripherals your code runs on. This is the thing `memory.x` and the `embassy-stm32`
  `stm32c562re` feature describe.

The Nucleo-64 form factor (64-pin package, Arduino + ST Morpho headers) is ST's standard
"here's a chip on a convenient board" product.

---

## 2. The STM32C562RE at a glance

Facts (from ST's datasheet, the source for [`memory.x`](../memory.x)):

| Property | Value | Where it shows up in the repo |
| -------- | ----- | ----------------------------- |
| CPU core | Arm **Cortex-M33F**, up to **144 MHz** | target `thumbv8m.main-none-eabihf` |
| Architecture | **Armv8-M Mainline** with FPU + TrustZone-M | the `thumbv8m.main` in the triple |
| Flash | **512 KB** @ `0x0800_0000` | `FLASH` region in `memory.x` |
| SRAM | **128 KB** @ `0x2000_0000` (part ECC-backed) | `RAM` region in `memory.x` |
| CAN | **1× FDCAN** controller | `can_fd.rs`, `FDCAN1` |
| Debug | On-board **ST-LINK** (SWD) | `probe-rs run --chip STM32C562RE` |

**STM32** is ST's huge family of Cortex-M microcontrollers. The **C5** line is a newer,
mainstream/efficiency-oriented series. "Brand new" is the operative phrase: the crates.io
release of `embassy-stm32` predates it, which is *the* reason this project pulls Embassy
from git (Doc 08).

### ECC RAM

Some of the SRAM is **ECC** (Error-Correcting Code) — extra bits let the hardware detect
and correct single-bit memory upsets (e.g. from electrical noise or radiation). That's a
reliability feature you'll appreciate in automotive contexts; you don't manage it in code,
but it's part of why this chip suits the domain.

---

## 3. The Cortex-M33 core

**Arm Cortex-M** is a family of 32-bit CPU cores designed for microcontrollers:
deterministic, low-power, interrupt-driven. The **M33** is a mid/high-end member of the
Armv8-M generation. What matters for this project:

- **Thumb instruction set.** Cortex-M runs the compact "Thumb" (Thumb-2) encoding — hence
  `thumbv8m` in the target triple (Doc 03).
- **FPU (the "F").** The M33**F** has a hardware floating-point unit. The `hf` in `eabihf`
  says "use hardware float instructions/ABI." (This firmware does little float work, but
  the ABI must match the silicon.)
- **NVIC** — the **Nested Vectored Interrupt Controller**. This is the hardware that
  receives interrupt signals from peripherals, prioritizes them, and dispatches the CPU to
  the right handler via the **vector table**. Embassy's whole wake mechanism (Doc 04) rides
  on the NVIC; `bind_interrupts!` populates the vector-table slots (`FDCAN1_IT0`, etc.).
- **Sleep instructions** — `WFI`/`WFE` ("wait for interrupt/event") let the core halt until
  something happens. The Embassy thread-mode executor uses these to sleep when idle.
- **TrustZone-M** — Armv8-M security extension partitioning secure/non-secure worlds. Not
  used by this template, but it's why the architecture is "Mainline" v8-M.

Cortex-M is the workhorse of embedded Rust: `cortex-m` and `cortex-m-rt` (Doc 08) provide
the core-level primitives (the reset handler, critical sections, register access).

---

## 4. Peripherals: how software touches the physical world

Around the CPU sit **peripherals** — dedicated hardware blocks for specific jobs. Software
controls them through **registers**: special memory addresses where each bit configures or
reads hardware. Writing to a GPIO register can literally set a pin's voltage; reading one
tells you a pin's state. This is **memory-mapped I/O (MMIO)**.

Raw register poking is unsafe and chip-specific, so (recall Doc 04) Rust wraps it in layers.
`embassy_stm32::init()` hands you a `Peripherals` struct owning each one exactly once, and
the HAL gives safe types on top.

### GPIO — the LED

**GPIO** = General-Purpose Input/Output: pins you can drive high/low (output) or read
(input). The heartbeat uses one as output:

```rust
let led = Output::new(peripherals.PA5, Level::Low, Speed::Low);
```

- `peripherals.PA5` — pin 5 of port A, *moved* in (so nothing else can use it).
- `Level::Low` — initial state (LED off, assuming active-high wiring).
- `Speed::Low` — the pin's slew rate / drive strength. An LED doesn't need to switch fast,
  so `Low` minimizes noise and power. (Fast buses like SPI would use a higher speed.)

`led.toggle()` flips it; under the hood the HAL writes the GPIO output register. Pin names
follow ST convention: `PA5` = **P**ort **A**, pin **5**; `PB8` = Port B, pin 8.

### Clocks and the RCC

Every digital block needs a **clock** — a square wave that paces its logic. The **RCC**
(Reset and Clock Control) peripheral generates and distributes clocks from oscillators and
**PLLs** (Phase-Locked Loops, which multiply a low frequency up to, say, 144 MHz). Clock
setup is *critical* for buses like CAN: the bit timing is derived from the peripheral's
clock, so a wrong clock = a wrong baud rate = no communication.

This template uses defaults:

```rust
let config = embassy_stm32::Config::default();
let peripherals = embassy_stm32::init(config);
```

The default clock tree is enough to boot and clock FDCAN for the demo. The code comments
correctly flag that **production** firmware should configure `config.rcc` (HSE/PLL and the
`mux.fdcansel` clock source) to match the *exact* bit timing your bus needs. Getting RCC
right is one of the most common "it compiles and flashes but doesn't work" issues (the
Embassy FAQ devotes a section to it).

### Timers and `embassy-time`

Delays like `Timer::after(HEARTBEAT_PERIOD).await` are backed by a **hardware timer**. The
`embassy-stm32` feature `time-driver-any` tells the HAL "pick a suitable hardware timer to
drive `embassy-time`." The tick rate is set by `embassy-time`'s `tick-hz-32_768` feature —
32,768 Hz is a classic choice (it's 2¹⁵, the frequency of watch crystals) giving ~30 µs
resolution at very low power. Without a time driver enabled you'd get the classic
`undefined symbol: _embassy_time_now` linker error (Embassy FAQ).

### Interrupts (recap from Doc 04)

Peripherals signal events by raising interrupts into the NVIC. FDCAN has two lines,
`FDCAN1_IT0` and `FDCAN1_IT1`; both are bound to Embassy handlers so received frames and
transmit completions wake the right `async` task. This is the bridge between "hardware did
something" and "resume my `.await`."

### DMA (context)

**DMA** (Direct Memory Access) is a controller that moves data between memory and
peripherals *without* the CPU copying each byte. Embassy leans on DMA heavily for high-rate
peripherals (its book calls DMA a "first choice"). CAN is a notable exception that doesn't
need DMA for framing, so this template doesn't configure DMA — but it's a core concept for
UART/SPI/ADC work you'll meet later.

---

## 5. The memory map, revisited

```
0x0800_0000 ┌───────────────────────────┐
            │ FLASH  (512 KB)           │  code (.text), constants (.rodata),
            │                           │  vector table, initial values of .data
0x0808_0000 └───────────────────────────┘
                     … gap …
0x2000_0000 ┌───────────────────────────┐
            │ SRAM   (128 KB)           │  .data, .bss (variables), the stack,
            │                           │  Embassy task storage
0x2002_0000 └───────────────────────────┘
```

`memory.x` encodes exactly these numbers; `cortex-m-rt`'s `link.x` and `flip-link` arrange
your program within them (Doc 03). Code runs directly from flash; variables and the stack
live in SRAM. With `flip-link`, the stack is placed so an overflow faults immediately
instead of corrupting your statics.

---

## 6. How chip support is generated (the neat part)

You might wonder how one crate, `embassy-stm32`, supports hundreds of STM32 chips. Per the
[Embassy book's "developers" section](https://embassy.dev/book/): ST publishes machine-
readable descriptions of every chip's peripherals; the `stm32-data` project parses them and
generates `stm32-metapac`, a **PAC** with typed register definitions for each chip.
`embassy-stm32` then uses Cargo feature flags (like `stm32c562re`) plus auto-derived
`cfg` flags to include exactly the right peripheral-version implementations. This is why:

- Selecting the wrong or missing chip feature breaks the build (the C5 wasn't in crates.io).
- The generated PAC comes from the `stm32-data-generated` git repo, which `deny.toml` must
  therefore trust as a source (Doc 08/10).

---

## 7. Pins used by this firmware — verify these

| Signal | Pin | Assumed because | Verify against |
| ------ | --- | --------------- | -------------- |
| User LED | `PA5` | Nucleo-64 convention (green LD user LED) | NUCLEO-C562RE schematic |
| FDCAN1 RX | `PB8` | Classic FDCAN1 mapping | schematic + CAN transceiver wiring |
| FDCAN1 TX | `PB9` | Classic FDCAN1 mapping | schematic + CAN transceiver wiring |

If your board differs, change the pins in [`src/main.rs`](../src/main.rs) and
[`src/can_fd.rs`](../src/can_fd.rs). Also note: CAN needs a **transceiver** (a chip that
converts the MCU's logic-level TX/RX into the differential CAN-H/CAN-L bus voltages) — the
MCU pins alone can't drive a real bus. More in [06-can-and-canfd.md](06-can-and-canfd.md).

**Next:** [06-can-and-canfd.md](06-can-and-canfd.md) — the automotive bus in depth.

---

### Where to go deeper (official)

- [STM32C562RE product page & datasheet (ST)](https://www.st.com/en/microcontrollers-microprocessors/stm32c5-series.html) — the source of truth for memory, pins, clocks.
- [NUCLEO-C562RE board page (ST)](https://www.st.com/en/evaluation-tools/) — schematic, user manual, pinout.
- [Arm Cortex-M33 (Arm)](https://developer.arm.com/Processors/Cortex-M33) — core reference.
- [`embassy-stm32` API docs](https://docs.embassy.dev/embassy-stm32/) — GPIO, RCC, CAN, timers.
- [Embassy book → "Understanding metapac"](https://embassy.dev/book/) — how per-chip support is generated.
