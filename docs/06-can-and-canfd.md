# 06 — CAN and CAN FD: the automotive bus

This is the heart of the "automotive" in this project. [`src/can_fd.rs`](../src/can_fd.rs)
implements a CAN FD talker/listener. To read it with understanding you need to know what
CAN *is*, why cars use it, and what "FD" adds. This doc builds that from zero and then maps
every concept onto the code.

---

## 1. Why a bus at all?

Recall from Doc 01 that a car has dozens of ECUs (engine, brakes, doors, dashboard…). If
every ECU needed a dedicated wire to every other ECU, the wiring harness would be enormous,
heavy, and expensive. Instead they share a **bus**: a single pair of wires that every ECU
connects to, taking turns to talk. **CAN** (Controller Area Network) is that bus.

CAN was created by Bosch in the 1980s specifically for automobiles and is standardized as
**ISO 11898**. It's now everywhere: cars, trucks, tractors, industrial machines, medical
devices. Its design goals map exactly onto Doc 01's constraints:

- **Robust in electrical noise** — uses a *differential* pair (see §2).
- **Multi-master, prioritized** — any node can talk; the most urgent message wins automatically.
- **Error-detecting and self-healing** — built-in CRCs, acknowledgments, and fault confinement.
- **Cheap and simple** — just two wires and a transceiver per node.

---

## 2. The physical layer: two wires, differential signaling

CAN uses two wires, **CAN-H** and **CAN-L**. A bit is encoded as the *voltage difference*
between them, not an absolute level:

- **Dominant bit (logical 0)** — the wires are driven apart.
- **Recessive bit (logical 1)** — the wires float together (idle).

Because electrical noise hits both wires almost equally, taking their *difference* cancels
the noise out — that's why CAN survives the harsh automotive environment. The bus needs
**120 Ω termination** resistors at each end to prevent signal reflections.

The MCU itself only produces logic-level (0/3.3 V) TX and RX signals. A separate chip, the
**CAN transceiver**, converts between those and the real differential CAN-H/CAN-L voltages.
So the signal path is:

```
MCU FDCAN peripheral ──TX/RX(logic)──▶ CAN transceiver ──CAN-H/CAN-L(differential)──▶ bus ──▶ other ECUs
```

> This is why Doc 05 flags that pins `PB8`/`PB9` alone can't drive a bus — you need a
> transceiver wired to them. On a Nucleo you typically add a CAN FD transceiver
> shield/board.

The clever part of dominant/recessive: if two nodes transmit at once and one sends dominant
while the other sends recessive, the **dominant wins** and the recessive transmitter *sees*
that it lost — the basis of arbitration below.

---

## 3. A CAN frame

Communication happens in **frames**. The important fields of a classic CAN data frame:

| Field | Purpose |
| ----- | ------- |
| **Identifier (ID)** | Names the *message* (not the sender). 11-bit "standard" or 29-bit "extended". Also sets priority. |
| **Control / DLC** | Data Length Code — how many data bytes follow. |
| **Data** | The payload: 0–8 bytes in classic CAN. |
| **CRC** | Checksum for error detection. |
| **ACK** | Any node that received it correctly pulls this dominant to acknowledge. |

Two ideas that surprise newcomers:

1. **Messages are addressed by content, not destination.** An ID like `0x100` identifies
   *what* the message is ("engine RPM"), not who it's for. Every node hears every frame and
   uses **filters** to decide which to keep. Our code uses ID `0x100` for its heartbeat
   frame (`HEARTBEAT_ID`).
2. **Priority is the ID.** Lower numeric ID = higher priority.

### Arbitration (how collisions are resolved without loss)

When multiple nodes start transmitting simultaneously, they all send their ID bit-by-bit
while listening. The moment a node sends a recessive bit but hears a dominant one, it knows
a higher-priority message is on the bus and **backs off** — without any data being
corrupted or retransmitted. The highest-priority frame proceeds uninterrupted. This is
**CSMA/CD with non-destructive arbitration**, and it's why CAN gives deterministic latency
for urgent messages (brakes beat infotainment, by ID).

---

## 4. CAN FD — "Flexible Data-rate"

Classic CAN tops out at ~1 Mbit/s and 8 data bytes. Modern cars need more, so Bosch
introduced **CAN FD** (standardized in ISO 11898-1:2015). Two improvements:

1. **Bigger payloads** — up to **64 data bytes** per frame (vs 8), so more data moves per
   frame with less overhead.
2. **Two bit rates per frame** — the *arbitration phase* (the ID) runs at a slower,
   robust **nominal** rate so arbitration still works across the whole bus; then, once one
   node has "won" and is alone on the bus, the *data phase* (payload + CRC) switches to a
   much faster **data** rate. This is the "flexible data-rate."

That two-speed design is *exactly* why the code has two bitrate constants:

```rust
const NOMINAL_BITRATE: u32 = 500_000;   // arbitration phase (500 kbit/s) — robust
const DATA_BITRATE:    u32 = 2_000_000; // data phase (2 Mbit/s) — fast payload
```

**FDCAN** is ST's peripheral that implements CAN FD (and is backward-compatible with
classic CAN). This chip has one instance, `FDCAN1`.

### Bit timing and TDC (why the `false`)

At high data rates, the round-trip delay through the transceiver becomes significant
relative to a bit, so CAN FD supports **Transmitter Delay Compensation (TDC)**. The code
disables it for the demo and documents why:

```rust
configurator.set_fd_data_bitrate(DATA_BITRATE, false); // false = TDC off
```

The comment correctly notes you'd enable TDC for high data rates once the transceiver's
loop delay is characterized. For a bench demo at 2 Mbit/s it's optional; for production
tuning it matters.

---

## 5. Reading `can_fd.rs` end to end

### 5.1 Interrupt binding (`mod irqs`)

Covered in Doc 04: `bind_interrupts!` wires FDCAN's two interrupt lines to Embassy's
handlers, fenced with a local `#[allow(unsafe_code)]` so the crate stays `deny(unsafe_code)`.
IT0/IT1 are the two FDCAN interrupt groups; binding both lets RX and TX events wake tasks.

### 5.2 `init` — configure and split

```rust
let mut configurator = CanConfigurator::new(instance, rx_pin, tx_pin, irqs::Irqs);
```

`CanConfigurator` is the HAL's builder for setting the peripheral up *before* it goes live.
Note the argument order is **(instance, RX pin, TX pin, interrupts)** — RX before TX.

```rust
configurator.properties().set_standard_filter(
    StandardFilterSlot::_0,
    StandardFilter::accept_all_into_fifo0(),
);
```

An **acceptance filter** decides which incoming IDs the hardware keeps. Here it accepts
*every* standard-ID frame into **RX FIFO 0** (a hardware queue of received frames). The
comment rightly says a real bus should use *specific* filters so the hardware discards
irrelevant traffic — saving CPU and preventing overload. Filtering in hardware is a key CAN
efficiency technique.

```rust
configurator.set_bitrate(NOMINAL_BITRATE);
configurator.set_fd_data_bitrate(DATA_BITRATE, false);
let can = configurator.start(OperatingMode::NormalOperationMode);
let (tx, rx, _properties) = can.split();
```

- `set_bitrate` / `set_fd_data_bitrate` — program the two-phase timing from §4.
- `start(NormalOperationMode)` — bring the peripheral online and actually drive the bus.
  (Other modes exist, e.g. loopback/internal test modes for bench testing without a bus.)
- `split()` — separate the live peripheral into an independent **`CanTx`** (transmit half)
  and **`CanRx`** (receive half). Splitting is what lets `tx_task` and `rx_task` each own
  one half and run concurrently with no shared-mutable-state problem (Doc 04). The
  `_properties` handle is discarded here.

### 5.3 `tx_task` — transmit a frame per second

```rust
let payload = [0xAA, sequence];
match Frame::new_standard(HEARTBEAT_ID, &payload) {
    Ok(frame)   => { let _ = tx.write(&frame).await; info!("CAN TX: id={:#x} seq={}", HEARTBEAT_ID, sequence); }
    Err(_error) => warn!("CAN TX: refused to build frame (invalid payload/id)"),
}
sequence = sequence.wrapping_add(1);
Timer::after(TX_PERIOD).await;
```

- `Frame::new_standard(id, &data)` builds a **standard (11-bit ID)** frame and returns a
  `Result` — construction can fail for an invalid id/length, so it's handled with `match`
  (Doc 02's safety rule: no `unwrap()` here).
- The 2-byte payload is a marker byte `0xAA` plus a rolling `sequence` counter, so a
  listener can see frames advancing.
- `tx.write(&frame).await` — asynchronously enqueue the frame into the TX FIFO; it yields
  if the FIFO is full and resumes when there's room. Its return value (a frame evicted from
  a full FIFO) isn't needed, so it's discarded with `let _ =`.
- `sequence.wrapping_add(1)` — increments and **wraps** `255 → 0` instead of overflowing.
  With `overflow-checks = true` (Doc 08/10) a plain `+ 1` would *panic* at 255; `wrapping_add`
  expresses "wrapping is intended," which is correct for a cyclic counter.
- `Timer::after(TX_PERIOD).await` — wait one second, yielding the CPU (Doc 04).

### 5.4 `rx_task` — receive and log forever

```rust
match rx.read().await {
    Ok(envelope) => {
        let (frame, _timestamp) = envelope.parts();
        let data = frame.data();
        info!("CAN RX: {} bytes {:?}", data.len(), data);
    }
    Err(error) => warn!("CAN RX bus error: {}", error),
}
```

- `rx.read().await` — asynchronously wait for a frame; yields (core may sleep) until the
  FDCAN RX interrupt wakes it (Doc 04). Returns a `Result` because the bus can report errors.
- On success you get an **envelope** = frame + a hardware **timestamp**; `parts()` splits
  them. The timestamp is ignored here (`_timestamp`).
- `frame.data()` is the `&[u8]` payload; it's logged with `defmt`'s `{:?}`.
- On a bus **error** (recall CAN's built-in error detection), it logs a warning and **keeps
  running** — the loop never dies. This error-tolerance is the automotive discipline from
  Doc 01: a malformed frame or bus glitch must not take the ECU down.

---

## 6. What you'd add for a real bus

The template is a teaching baseline. Toward production you would:

- **Configure RCC/clock and precise bit timing** so the baud rate exactly matches the bus
  (Doc 05). Mismatched timing is the #1 cause of a silent CAN link.
- **Use specific acceptance filters** instead of accept-all, so hardware drops irrelevant IDs.
- **Enable TDC** at high data rates after characterizing transceiver delay.
- **Define a message set / DBC** — real vehicles use higher-level protocols on top of CAN
  (e.g. **CANopen**, **J1939** for trucks, **UDS** for diagnostics), where IDs and byte
  layouts have agreed meanings.
- **Handle bus-off recovery** — CAN's fault confinement can take a node "bus-off" after too
  many errors; production firmware manages re-integration.

**Next:** [07-architecture.md](07-architecture.md) — how all the files fit together.

---

### Where to go deeper (official / authoritative)

- [Bosch CAN specification & CAN FD](https://www.bosch-semiconductors.com/ip-modules/can-ip-modules/) — the origin of CAN/CAN FD.
- ISO 11898-1 (CAN data link layer, incl. CAN FD) and ISO 11898-2 (high-speed physical layer) — the standards (paywalled; summaries widely available).
- [`embassy-stm32` CAN module docs](https://docs.embassy.dev/embassy-stm32/) — `CanConfigurator`, `CanTx`, `CanRx`, `Frame`, filters.
- [STM32 FDCAN section in the reference manual (ST)](https://www.st.com/en/microcontrollers-microprocessors/stm32c5-series.html) — the peripheral's registers and modes.
