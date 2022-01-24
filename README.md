# Onekibu (One-Button Keyboard)

Onekibu is a firmware providing a USB keyboard interface for devices with a
single button, like the [nRF52840 dongle] or the [nRF52840 MDK dongle]. As such,
keys are pressed (and released) with patterns similar to [Morse code].

## How to release

To release for `$BOARD` (see below for board-specific instructions), run the
following command:

```
cargo xtask build --board=$BOARD --release --flash
```

The `--size` flag can be added to show the binary size before flashing.

### nRF52840 dongle

To release on the [nRF52840 dongle] using `nrfdfu` (which you can install with
`cargo install nrfdfu`):

1. Plug the dongle.
2. Press its reset button (may also be done while plugging).
3. Run the command above with `BOARD=nrf52840-dongle`.

### nRF52840 MDK dongle

To release on the [nRF52840 MDK dongle] using `uf2conv.py` (which you can copy
from [here][uf2conv]):

1. Plug the dongle while pressing its reset button.
2. Run the command above with `BOARD=nrf52840-mdk-dongle`.

## How to debug

To build and run on the nRF52840 dev-kit using `probe-run` (which you can
install with `cargo install probe-run`), run the following command:

```
cargo xtask build --flash
```

The `--log=<DEFMT_LOG>` flag can be used to set the [defmt logging filter]. For
example, `--log=warn` would only should warnings or errors.

To attach gdb, run the following command in a separate terminal:

```
cargo xtask gdb server
```

Then run the following command:

```
cargo xtask gdb client
```

[defmt logging filter]: https://defmt.ferrous-systems.com/filtering.html

## Disclaimer

This is not an official Google product.

[Morse code]: https://en.wikipedia.org/wiki/Morse_code
[nRF52840 MDK dongle]: https://wiki.makerdiary.com/nrf52840-mdk-usb-dongle
[nRF52840 dongle]: https://www.nordicsemi.com/Products/Development-hardware/nrf52840-dongle
[uf2conv]: https://github.com/microsoft/uf2/tree/master/utils
