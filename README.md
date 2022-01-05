# Onekibu (One-Button Keyboard)

Onekibu is a firmware providing a USB keyboard interface for devices with a
single button, like the [nRF52840 dongle]. As such, keys are pressed (and
released) with patterns similar to [Morse code].

## How to debug

To build and run on the nRF52840 dev-kit, run the following command:

```
cargo xtask build --flash
```

The `--log=<DEFMT_LOG>` flag can be used to set the [defmt logging filter]. For
example, `--log=warn` would only should warnings or errors.

[defmt logging filter]: https://defmt.ferrous-systems.com/filtering.html

## How to release

To release on the nRF52840 dongle using DFU, run the following command:

```
cargo xtask build --board=nrf52840-dongle --release --flash
```

The `--size` flag can be added to show the binary size before flashing.

## Disclaimer

This is not an official Google product.

[nRF52840 dongle]: https://www.nordicsemi.com/Products/Development-hardware/nrf52840-dongle
[Morse code]: https://en.wikipedia.org/wiki/Morse_code
