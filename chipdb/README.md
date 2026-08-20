# chipdb — bundled probe-rs chip description

`STM32C5_Series.yaml` is a **probe-rs target description** for the STM32C5 series
(including the board's `STM32C562RET6`). It carries the flash algorithm and memory
map probe-rs needs to program the chip.

## Why it's vendored here

The STM32C5 series is newer than probe-rs's built-in chip database (probe-rs
`0.32` stops at STM32C0). Rather than make every developer download a CMSIS pack
and run `target-gen`, the generated description is committed and wired into
[`.cargo/config.toml`](../.cargo/config.toml):

```
runner = "probe-rs run --chip STM32C562RET6 --chip-description-path chipdb/STM32C5_Series.yaml"
```

So `cargo run --release` flashes with **zero manual setup** — no pack download,
no `target-gen`. The only host prerequisites remain `probe-rs` and `flip-link`
(see the root `README.md`).

## Provenance

- Source: ST's CMSIS Device Family Pack **`STMicroelectronics.stm32c5xx_dfp` v2.1.0**
  (`https://developer.st.com/st-pack-server/api/v1/pack/STMicroelectronics.stm32c5xx_dfp.2.1.0.pack`).
- The flash algorithm within is © STMicroelectronics, redistributed from that pack
  (the same source probe-rs's own bundled targets are built from).

## Regenerating / bumping the chip pack

Bump deliberately (like the pinned Embassy `rev`), then re-verify a flash:

```bash
cargo install --git https://github.com/probe-rs/probe-rs target-gen
curl -fsSL -o /tmp/stm32c5.pack \
  https://developer.st.com/st-pack-server/api/v1/pack/STMicroelectronics.stm32c5xx_dfp.<VERSION>.pack
target-gen pack /tmp/stm32c5.pack /tmp/chipout
cp /tmp/chipout/STM32C5_Series.yaml chipdb/STM32C5_Series.yaml
cargo run --release        # confirm it still flashes + logs
```
