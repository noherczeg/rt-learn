//! rt-learn — safety-oriented Embassy firmware template for the NUCLEO-C562RE.
//!
//! Target: STMicroelectronics STM32C562RE (Arm Cortex-M33F).
//!
//! Architecture:
//!   * [`heartbeat`]   — blinks the on-board user LED (PA5 / LD1) as a liveness beacon.
//!
//! Logging is done exclusively through `defmt` over RTT; panics are printed via
//! `panic-probe` and then abort (see `panic = "abort"` in Cargo.toml).

#![no_std]
#![no_main]
// Zero-tolerance lint gate.
// The crate is fully safe: there are zero `unsafe` blocks, so this gate has no
// exceptions anywhere.
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
    // Default clock tree is sufficient to bring the core up. For production,
    // configure `config.rcc` (HSE/PLL) to match your timing requirements.
    let config = embassy_stm32::Config::default();
    let peripherals = embassy_stm32::init(config);

    info!("rt-learn boot: STM32C562RE / Cortex-M33F");

    // LED heartbeat task.
    let led = Output::new(peripherals.PA5, Level::Low, Speed::Low);
    spawner.spawn(unwrap!(heartbeat(led)));

    info!("heartbeat task spawned");
}

/// Periodic LED toggle proving the executor is alive.
#[embassy_executor::task]
async fn heartbeat(mut led: Output<'static>) {
    loop {
        led.toggle();
        Timer::after(HEARTBEAT_PERIOD).await;
    }
}
