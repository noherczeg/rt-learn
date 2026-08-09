//! Modular FDCAN (CAN FD) driver wrapper and async tasks for the STM32C562RE.
//!
//! Responsibilities:
//!   * bind the FDCAN interrupt lines to the HAL handlers,
//!   * configure nominal + data bit timing and an accept-all RX filter,
//!   * bring the peripheral into normal operating mode, and
//!   * expose two Embassy tasks that transmit and receive frames concurrently.
//!
//! All fallible operations are handled explicitly: there are no `unwrap()` calls
//! or raw panics in the run loops — malformed frames and bus errors are logged
//! and the task keeps running.

use defmt::{info, warn};
use embassy_stm32::can::filter::{StandardFilter, StandardFilterSlot};
use embassy_stm32::can::frame::Frame;
use embassy_stm32::can::{CanConfigurator, CanRx, CanTx, OperatingMode};
use embassy_stm32::peripherals::{FDCAN1, PB8, PB9};
use embassy_stm32::Peri;
use embassy_time::{Duration, Timer};

/// Nominal (arbitration phase) bit rate in bits per second.
const NOMINAL_BITRATE: u32 = 500_000;
/// Data-phase bit rate for the FD payload in bits per second.
const DATA_BITRATE: u32 = 2_000_000;
/// Standard CAN identifier used for the outgoing heartbeat frame.
const HEARTBEAT_ID: u16 = 0x100;
/// Interval between transmitted heartbeat frames.
const TX_PERIOD: Duration = Duration::from_millis(1_000);

// SAFETY GATE: `bind_interrupts!` emits the interrupt-vector bindings, which are
// inherently `unsafe`. This local `#[allow(unsafe_code)]` is the *only* exception
// to the crate-wide `#![deny(unsafe_code)]`; no application logic runs here.
#[allow(unsafe_code)]
mod irqs {
    use embassy_stm32::can;
    use embassy_stm32::peripherals::FDCAN1;

    embassy_stm32::bind_interrupts!(pub struct Irqs {
        FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
        FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
    });
}

/// Initialize FDCAN1 and split it into independent TX and RX halves.
///
/// `rx_pin`/`tx_pin` must be routed to the board's CAN FD transceiver. Returns the
/// transmit and receive endpoints, ready to be handed to [`tx_task`]/[`rx_task`].
#[must_use]
pub fn init(
    instance: Peri<'static, FDCAN1>,
    rx_pin: Peri<'static, PB8>,
    tx_pin: Peri<'static, PB9>,
) -> (CanTx<'static>, CanRx<'static>) {
    let mut configurator = CanConfigurator::new(instance, rx_pin, tx_pin, irqs::Irqs);

    // Accept every standard-ID frame into RX FIFO 0. Tighten this to explicit
    // acceptance filters for a real bus to reject irrelevant traffic in hardware.
    configurator.properties().set_standard_filter(
        StandardFilterSlot::_0,
        StandardFilter::accept_all_into_fifo0(),
    );

    configurator.set_bitrate(NOMINAL_BITRATE);
    // `false` disables transmitter delay compensation; enable it for high data
    // bit rates once the transceiver loop delay is characterized.
    configurator.set_fd_data_bitrate(DATA_BITRATE, false);

    let can = configurator.start(OperatingMode::NormalOperationMode);
    let (tx, rx, _properties) = can.split();
    (tx, rx)
}

/// Async transmit task: emits a heartbeat FD frame once per [`TX_PERIOD`].
#[embassy_executor::task]
pub async fn tx_task(mut tx: CanTx<'static>) {
    let mut sequence: u8 = 0;

    loop {
        let payload = [0xAA, sequence];

        match Frame::new_standard(HEARTBEAT_ID, &payload) {
            Ok(frame) => {
                // `write` returns any frame evicted from a full TX FIFO; we do not
                // need it here, so it is explicitly discarded.
                let _ = tx.write(&frame).await;
                info!("CAN TX: id={:#x} seq={}", HEARTBEAT_ID, sequence);
            }
            Err(_error) => warn!("CAN TX: refused to build frame (invalid payload/id)"),
        }

        sequence = sequence.wrapping_add(1);
        Timer::after(TX_PERIOD).await;
    }
}

/// Async receive task: blocks on the RX FIFO and logs each incoming frame.
#[embassy_executor::task]
pub async fn rx_task(mut rx: CanRx<'static>) {
    loop {
        match rx.read().await {
            Ok(envelope) => {
                let (frame, _timestamp) = envelope.parts();
                let data = frame.data();
                info!("CAN RX: {} bytes {:?}", data.len(), data);
            }
            Err(error) => warn!("CAN RX bus error: {}", error),
        }
    }
}
