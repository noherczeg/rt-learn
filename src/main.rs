//! rt-learn — safety-oriented Embassy firmware template for the NUCLEO-C562RE.
//!
//! Target: STMicroelectronics STM32C562RE (Arm Cortex-M33F, FDCAN).
//!
//! Architecture:
//!   * [`heartbeat`]   — Task 1: blinks the on-board user LED as a liveness beacon.
//!   * [`can_fd`]      — Task 2: initializes FDCAN and runs async TX/RX tasks.
//!
//! Logging is done exclusively through `defmt` over RTT; panics are printed via
//! `panic-probe` and then abort (see `panic = "abort"` in Cargo.toml).

#![no_std]
#![no_main]
// Zero-tolerance lint gate. `deny(unsafe_code)` is crate-wide; the single
// unavoidable `unsafe` (the interrupt-vector bindings emitted by
// `bind_interrupts!`) is locally re-permitted in `can_fd.rs`.
#![deny(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
// Transitive embedded dependencies (Embassy + PAC + HAL) legitimately pull
// duplicate crate versions we cannot control; keep this from failing `cargo clippy
// -- -D warnings`. Duplicates are still surfaced by `cargo deny check`.
#![allow(clippy::multiple_crate_versions)]
// Embassy's thread-mode executor is single-core and cooperatively scheduled; its
// task futures (which capture the `!Send` `Spawner`) are intentionally not `Send`.
// The nursery `future_not_send` lint does not apply to this executor model.
#![allow(clippy::future_not_send)]

mod can_fd;

use defmt::{info, unwrap};
use defmt_rtt as _; // global defmt logger (RTT transport)
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::{Duration, Timer};
use panic_probe as _; // panic handler that prints over defmt then aborts

/// LED heartbeat blink interval.
const HEARTBEAT_PERIOD: Duration = Duration::from_millis(500);

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Default clock tree is sufficient to bring the core up and clock the FDCAN
    // kernel. For production, configure `config.rcc` (HSE/PLL + `mux.fdcansel`)
    // to match the exact bit timing your bus requires.
    let config = embassy_stm32::Config::default();
    let peripherals = embassy_stm32::init(config);

    info!("rt-learn boot: STM32C562RE / Cortex-M33F");

    // Task 1: LED heartbeat.
    let led = Output::new(peripherals.PA5, Level::Low, Speed::Low);
    spawner.spawn(unwrap!(heartbeat(led)));

    // Task 2: FDCAN. Pins PB8 (FDCAN1_RX) / PB9 (FDCAN1_TX) are the classic
    // FDCAN1 mapping routed to the board's CAN FD header/transceiver — adjust to
    // match your wiring.
    let (can_tx, can_rx) = can_fd::init(peripherals.FDCAN1, peripherals.PB8, peripherals.PB9);
    spawner.spawn(unwrap!(can_fd::tx_task(can_tx)));
    spawner.spawn(unwrap!(can_fd::rx_task(can_rx)));

    info!("all tasks spawned");
}

/// Task 1 — periodic LED toggle proving the executor is alive.
#[embassy_executor::task]
async fn heartbeat(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after(HEARTBEAT_PERIOD).await;
    }
}
