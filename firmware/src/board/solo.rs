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

use stm32l4xx_hal::delay::Delay;
use stm32l4xx_hal::gpio::{
    gpioa::{PAx, PA0},
    Input, Output, PullUp, PushPull,
};
use stm32l4xx_hal::prelude::*;
use stm32l4xx_hal::stm32;
use stm32l4xx_hal::usb::{Peripheral, UsbBus};
use usb_device::class_prelude::UsbBusAllocator;

pub use stm32l4xx_hal::pac;

pub struct Board {
    button: PA0<Input<PullUp>>,
    leds: [PAx<Output<PushPull>>; 3],
    timer: Delay,
}

static mut USB_BUS: Option<UsbBusAllocator<UsbBus<Peripheral>>> = None;

impl Board {
    pub fn blink(&mut self, i: usize) {
        for _ in 0 .. 3 {
            self.leds[i].set_low().unwrap();
            self.timer.delay_ms(100u32);
            self.leds[i].set_high().unwrap();
            self.timer.delay_ms(100u32);
        }
    }
}

impl super::BoardApi for Board {
    type UsbBus = UsbBus<Peripheral>;

    fn new(mut c: rtic::export::Peripherals, p: pac::Peripherals) -> Board {
        c.DCB.enable_trace();
        pac::DWT::unlock();
        c.DWT.enable_cycle_counter();
        let mut flash = p.FLASH.constrain();
        let mut rcc = p.RCC.constrain();
        let mut pwr = p.PWR.constrain(&mut rcc.apb1r1);
        // TODO: There is still some problem with USB. The button and leds seem to work (although
        // red takes precedence over blue and green).
        let clocks = rcc
            .cfgr
            .hclk(8.mhz())
            // .hsi48(true)
            // .sysclk(48.mhz())
            // .pclk1(24.mhz())
            // .pclk2(24.mhz())
            .freeze(&mut flash.acr, &mut pwr);

        {
            let rcc = unsafe { &(*stm32::RCC::ptr()) };
            rcc.apb1enr1.modify(|_, w| w.crsen().set_bit());
            let crs = unsafe { &(*stm32::CRS::ptr()) };
            // Initialize clock recovery
            // Set autotrim enabled.
            crs.cr.modify(|_, w| w.autotrimen().set_bit());
            // Enable CR
            crs.cr.modify(|_, w| w.cen().set_bit());

            // Enable PWR peripheral
            let rcc = unsafe { &(*stm32::RCC::ptr()) };
            rcc.apb1enr1.modify(|_, w| w.pwren().set_bit());

            // Enable VddUSB
            let pwr = unsafe { &*stm32::PWR::ptr() };
            pwr.cr2.modify(|_, w| w.usv().set_bit());
        }

        let mut gpioa = p.GPIOA.split(&mut rcc.ahb2);
        let button = gpioa.pa0.into_pull_up_input(&mut gpioa.moder, &mut gpioa.pupdr);
        let mut leds = [
            gpioa.pa2.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper).downgrade(),
            gpioa.pa3.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper).downgrade(),
            gpioa.pa1.into_push_pull_output(&mut gpioa.moder, &mut gpioa.otyper).downgrade(),
        ];
        // For some reason, this seems to be needed.
        for led in leds.iter_mut() {
            led.set_high().unwrap();
        }
        let timer = Delay::new(c.SYST, clocks);
        let usb = Peripheral {
            usb: p.USB,
            pin_dm: gpioa.pa11.into_af10(&mut gpioa.moder, &mut gpioa.afrh),
            pin_dp: gpioa.pa12.into_af10(&mut gpioa.moder, &mut gpioa.afrh),
        };
        unsafe {
            USB_BUS = Some(UsbBus::new(usb));
        }
        Board { button, leds, timer }
    }

    fn usb_bus(&self) -> &'static UsbBusAllocator<Self::UsbBus> {
        unsafe { USB_BUS.as_ref().unwrap() }
    }

    fn config(&self) -> onekibu::Config {
        let period = 1000000;
        onekibu::Config { maximum: u32::MAX as usize, period }
    }

    fn input(&self) -> onekibu::Input {
        onekibu::Input {
            timestamp: pac::DWT::get_cycle_count() as usize,
            button: self.button.is_low().unwrap(),
        }
    }

    fn state(&mut self, state: onekibu::BitState) {
        let bits = match state {
            onekibu::BitState::Ready => [0, 0, 0],
            onekibu::BitState::Short => [0, 1, 0],
            onekibu::BitState::Long => [0, 1, 1],
            onekibu::BitState::Cancel => [1, 0, 0],
            onekibu::BitState::Done => [0, 0, 1],
        };
        for i in 0 .. 3 {
            if bits[i] == 0 {
                self.leds[i].set_high().unwrap();
            } else {
                self.leds[i].set_low().unwrap();
            }
        }
    }
}
