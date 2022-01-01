// Copyright 2021 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_main]
#![no_std]

mod board;

// TODO: Allow configuration with as mass storage (e.g. a file with mapping, or one file per
// mapping).

// TODO: Use NFC to display current mapping (or configuration) as text. And switch between
// pre-configured mappings (or to configure new mapping? is it possible?).

#[rtic::app(device = crate::board::pac, peripherals = true)]
mod app {
    use crate::board::{Board, BoardApi};
    use defmt::Debug2Format;
    #[cfg(debug_assertions)]
    use defmt_rtt as _;
    #[cfg(not(debug_assertions))]
    use panic_abort as _;
    #[cfg(debug_assertions)]
    use panic_probe as _;
    use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
    use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
    use usbd_hid::hid_class::HIDClass;
    use usbd_hid::UsbError;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        board: Board,
        usb: Usb,
        state: onekibu::State,
    }

    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::trace!("init");
        let board = Board::new(c.core, c.device);
        // TODO: Somehow show when the board is ready (USB ready), e.g. red light from here until
        // USB ready.
        let usb_bus = board.usb_bus();
        let usb_hid = HIDClass::new(usb_bus, KeyboardReport::desc(), 60);
        let usb_dev =
            UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x04ca, 0x0020)).product("onekibu").build();
        let usb = Usb { hid: usb_hid, dev: usb_dev };
        let state = onekibu::State::new(board.config());
        (Shared {}, Local { board, usb, state }, init::Monotonics())
    }

    #[idle(local = [board, usb, state])]
    fn idle(c: idle::Context) -> ! {
        defmt::trace!("idle");
        let idle::LocalResources { board, usb, state } = c.local;
        loop {
            let key = state.step(board.input());
            board.state(state.bit_state());
            usb_push(usb, key);
        }
    }

    pub struct Usb {
        dev: UsbDevice<'static, <Board as BoardApi>::UsbBus>,
        hid: HIDClass<'static, <Board as BoardApi>::UsbBus>,
    }

    fn usb_push(usb: &mut Usb, key: Option<onekibu::Output>) {
        usb_poll(usb);
        let mut key = match key {
            None => return,
            Some(x) => x,
        };
        loop {
            let mut input = [0; 8];
            input[0] = key.modifiers;
            input[2] = key.key;
            match usb.hid.push_raw_input(&input) {
                Ok(len) if len != input.len() => defmt::error!("pushed only {} bytes", len),
                Ok(_) => {
                    defmt::trace!("push {=[u8]:#x}", &input[..]);
                    if key.key == 0 {
                        break;
                    }
                    key = onekibu::Output::default();
                }
                Err(UsbError::WouldBlock) => (),
                Err(err) => defmt::error!("push failed: {:?}", Debug2Format(&err)),
            }
            usb_poll(usb);
        }
    }

    fn usb_poll(usb: &mut Usb) {
        if !usb.dev.poll(&mut [&mut usb.hid]) {
            return;
        }
        let mut buf = [0; 32];
        match usb.hid.pull_raw_output(&mut buf) {
            Ok(len) => defmt::warn!("poll {=[u8]:#x}", &buf[.. len]),
            Err(UsbError::WouldBlock) => (),
            Err(err) => defmt::error!("poll failed: {:?}", Debug2Format(&err)),
        }
    }
}
