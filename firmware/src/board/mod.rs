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

#[cfg(feature = "chip-nrf52840")]
mod nrf52840;
#[cfg(feature = "chip-nrf52840")]
pub use nrf52840::{pac, Board};

#[cfg(feature = "board-solo")]
mod solo;
#[cfg(feature = "board-solo")]
pub use solo::{pac, Board};

pub trait BoardApi {
    type UsbBus: usb_device::bus::UsbBus;

    fn new(c: rtic::export::Peripherals, p: pac::Peripherals) -> Self;
    fn usb_bus(&self) -> &'static usb_device::class_prelude::UsbBusAllocator<Self::UsbBus>;
    fn config(&self) -> onekibu::Config;
    fn input(&self) -> onekibu::Input;
    fn state(&mut self, state: onekibu::BitState);
}
