// Copyright 2021-2022 Google LLC
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

use nrf52840_hal::clocks::{Clocks, ExternalOscillator, Internal, LfOscStopped};
use nrf52840_hal::gpio::{self, Input, Level, Output, Pin, PullUp, PushPull};
use nrf52840_hal::prelude::{InputPin, OutputPin};
use nrf52840_hal::usbd::{UsbPeripheral, Usbd};
use usb_device::class_prelude::UsbBusAllocator;

pub use nrf52840_hal::pac;

pub struct Board {
    #[cfg(feature = "board-nrf52840-dk")]
    button: [Pin<Input<PullUp>>; 4],
    #[cfg(any(feature = "board-nrf52840-dongle", feature = "board-nrf52840-mdk-dongle"))]
    button: Pin<Input<PullUp>>,
    #[cfg(any(feature = "board-nrf52840-dk", feature = "board-nrf52840-dongle"))]
    leds: [Pin<Output<PushPull>>; 4],
    #[cfg(feature = "board-nrf52840-mdk-dongle")]
    leds: [Pin<Output<PushPull>>; 3],
    timer: pac::TIMER0,
}

static mut CLOCKS: Option<Clocks<ExternalOscillator, Internal, LfOscStopped>> = None;
static mut USB_BUS: Option<UsbBusAllocator<Usbd<UsbPeripheral<'static>>>> = None;

impl super::BoardApi for Board {
    type UsbBus = Usbd<UsbPeripheral<'static>>;

    fn new(_c: rtic::export::Peripherals, p: pac::Peripherals) -> Board {
        let port0 = gpio::p0::Parts::new(p.P0);
        #[cfg(feature = "board-nrf52840-dongle")]
        let port1 = gpio::p1::Parts::new(p.P1);
        #[cfg(feature = "board-nrf52840-dk")]
        let button = [
            port0.p0_11.into_pullup_input().degrade(),
            port0.p0_12.into_pullup_input().degrade(),
            port0.p0_24.into_pullup_input().degrade(),
            port0.p0_25.into_pullup_input().degrade(),
        ];
        #[cfg(feature = "board-nrf52840-dongle")]
        let button = port1.p1_06.into_pullup_input().degrade();
        #[cfg(feature = "board-nrf52840-mdk-dongle")]
        let button = port0.p0_18.into_pullup_input().degrade();
        #[cfg(feature = "board-nrf52840-dk")]
        let leds = [
            port0.p0_13.into_push_pull_output(Level::High).degrade(),
            port0.p0_14.into_push_pull_output(Level::High).degrade(),
            port0.p0_15.into_push_pull_output(Level::High).degrade(),
            port0.p0_16.into_push_pull_output(Level::High).degrade(),
        ];
        #[cfg(feature = "board-nrf52840-dongle")]
        let leds = [
            port0.p0_06.into_push_pull_output(Level::High).degrade(),
            port0.p0_08.into_push_pull_output(Level::High).degrade(),
            port1.p1_09.into_push_pull_output(Level::High).degrade(),
            port0.p0_12.into_push_pull_output(Level::High).degrade(),
        ];
        #[cfg(feature = "board-nrf52840-mdk-dongle")]
        let leds = [
            port0.p0_23.into_push_pull_output(Level::High).degrade(),
            port0.p0_22.into_push_pull_output(Level::High).degrade(),
            port0.p0_24.into_push_pull_output(Level::High).degrade(),
        ];
        let timer = p.TIMER0;
        timer.prescaler.write(
            |w| unsafe { w.prescaler().bits(4) }, // 1 MHz
        );
        timer.bitmode.write(|w| w.bitmode()._32bit());
        timer.tasks_start.write(|w| w.tasks_start().set_bit());
        unsafe {
            CLOCKS = Some(Clocks::new(p.CLOCK).enable_ext_hfosc());
            let clocks = CLOCKS.as_ref().unwrap();
            USB_BUS = Some(Usbd::new(UsbPeripheral::new(p.USBD, clocks)));
        }
        Board { button, leds, timer }
    }

    fn usb_bus(&self) -> &'static UsbBusAllocator<Self::UsbBus> {
        unsafe { USB_BUS.as_ref().unwrap() }
    }

    fn config(&self) -> onekibu::Config {
        let dit = 80000; // 80ms
        onekibu::Config { maximum: u32::MAX as usize, period: 2 * dit }
    }

    fn input(&self) -> onekibu::Input {
        self.timer.tasks_capture[1].write(|w| w.tasks_capture().set_bit());
        let timestamp = self.timer.cc[1].read().bits() as usize;
        #[cfg(feature = "board-nrf52840-dk")]
        let button = self.button.iter().any(|x| x.is_low().unwrap());
        #[cfg(any(feature = "board-nrf52840-dongle", feature = "board-nrf52840-mdk-dongle"))]
        let button = self.button.is_low().unwrap();
        onekibu::Input { timestamp, button }
    }

    fn state(&mut self, state: onekibu::BitState) {
        let bits = match state {
            onekibu::BitState::Ready => [0, 0, 0, 0],
            onekibu::BitState::Short => [0, 0, 1, 0],
            onekibu::BitState::Long => [0, 1, 1, 0],
            onekibu::BitState::Cancel => [0, 1, 0, 0],
            onekibu::BitState::Done => [0, 0, 0, 1],
        };
        #[cfg(feature = "board-nrf52840-mdk-dongle")]
        let bits = &bits[1 ..];
        for (i, &b) in bits.iter().enumerate() {
            if b == 0 {
                self.leds[i].set_high().unwrap();
            } else {
                self.leds[i].set_low().unwrap();
            }
        }
    }
}
