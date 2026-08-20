# Heartbeat Specification

## Purpose

The firmware runs a single continuous liveness beacon: an on-board LED that toggles
forever, proving the microcontroller booted, the Embassy `async` executor is running,
and the timer subsystem ticks. It is the smallest possible Embassy task and the seed
onto which further tasks are added.

## Architecture

The behavior lives entirely in `src/main.rs`:

- **`main` (`#[embassy_executor::main]`)**: initializes the HAL with
  `embassy_stm32::init(Config::default())`, constructs an `Output` LED on pin `PA5`
  (LD1 on the NUCLEO-C562RE) at `Level::Low` / `Speed::Low`, then spawns the
  `heartbeat` task onto the executor. It emits two boot log lines via `defmt`.
- **`heartbeat` (`#[embassy_executor::task]`)**: an infinite `loop` that toggles the
  LED and `.await`s a `Timer` for `HEARTBEAT_PERIOD`.
- **`HEARTBEAT_PERIOD` (`const Duration`)**: the blink interval, `500 ms`.

Logging is exclusively `defmt` over RTT (`defmt_rtt`); panics are printed by
`panic-probe` and then abort (`panic = "abort"`). The executor is single-core,
cooperatively scheduled; while the task `.await`s the timer the core sleeps.

## Requirements

### Requirement: Firmware SHALL blink the on-board LED as a liveness beacon

The firmware SHALL toggle the on-board user LED at a fixed interval indefinitely,
demonstrating the executor and timer are live.

#### Scenario: LED toggles at the heartbeat period

- **GIVEN** the firmware is flashed and running on the NUCLEO-C562RE
- **WHEN** the executor starts the `heartbeat` task
- **THEN** the LED on `PA5` toggles every `HEARTBEAT_PERIOD` (500 ms)
- **AND** the toggling continues forever without a terminating condition

#### Scenario: Core sleeps between toggles

- **GIVEN** the `heartbeat` task has toggled the LED
- **WHEN** it `.await`s the `Timer`
- **THEN** the core yields to the executor and is free to sleep or run other tasks
  until the timer fires

### Requirement: Firmware SHALL emit boot diagnostics over defmt/RTT

The firmware SHALL log its boot and task-spawn milestones through `defmt` over RTT so
a host running `probe-rs` can confirm startup.

#### Scenario: Boot log lines appear on the host

- **GIVEN** a host is streaming defmt logs via `probe-rs`
- **WHEN** the firmware boots
- **THEN** a boot line identifying the target (`rt-learn boot: STM32C562RE / Cortex-M33F`)
  is emitted
- **AND** a `heartbeat task spawned` line is emitted after the task is spawned

### Requirement: Firmware SHALL halt safely on panic

The firmware SHALL route panics through `panic-probe` (printed over defmt) and abort
rather than unwind.

#### Scenario: A panic is reported and halts

- **GIVEN** the firmware encounters a panic
- **WHEN** the panic handler runs
- **THEN** the panic message is printed over defmt
- **AND** the firmware aborts (no stack unwinding, `panic = "abort"`)
