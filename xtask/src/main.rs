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

use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Flags {
    /// Builds the firmware
    Build(Build),

    /// Runs the unit tests
    Test,
}

#[derive(Debug, StructOpt)]
struct Build {
    /// Build or flash the firmware for this board
    #[structopt(long, default_value = "nrf52840-dk")]
    board: String,

    /// Build the firmware in release mode
    #[structopt(long)]
    release: bool,

    /// Log filter [default: off for --release, trace otherwise]
    #[structopt(long)]
    log: Option<String>,

    /// Show the size of the firmware
    #[structopt(long)]
    size: bool,

    /// Flash (and run, if the board supports it) the firmware
    #[structopt(long)]
    flash: bool,
}

impl Flags {
    fn execute(self) {
        match self {
            Flags::Build(x) => x.execute(),
            Flags::Test => {
                let mut cargo = Command::new("cargo");
                cargo.arg("test");
                cargo.arg("--package=onekibu");
                cargo.arg("--lib");
                cargo.spawn();
            }
        }
    }
}

impl Build {
    fn execute(self) {
        let mut cargo = Command::new("cargo");
        let mut rustflags = vec!["-C link-arg=--nmagic", "-C link-arg=-Tlink.x"];
        cargo.env("ONEKIBU_MEMORY_X", format!("{}.x", self.board));
        cargo.arg("build");
        cargo.arg("--package=onekibu");
        let target = "thumbv7em-none-eabi";
        cargo.arg(format!("--target={}", target));
        cargo.arg(format!("--features=board-{}", self.board));
        let mode = if self.release {
            cargo.arg("--release");
            rustflags.push("-C codegen-units=1");
            rustflags.push("-C embed-bitcode=yes");
            rustflags.push("-C lto=fat");
            rustflags.push("-C opt-level=z");
            cargo.env("DEFMT_LOG", "off");
            "release"
        } else {
            rustflags.push("-C link-arg=-Tdefmt.x");
            cargo.env("DEFMT_LOG", "trace");
            "debug"
        };
        if let Some(filter) = self.log {
            cargo.env("DEFMT_LOG", filter);
        }
        cargo.env("RUSTFLAGS", rustflags.join(" "));
        cargo.spawn();
        let elf = format!("target/{}/{}/onekibu", target, mode);
        if self.size {
            let mut size = Command::new("rust-size");
            size.arg(&elf);
            size.spawn();
        }
        if !self.flash {
            return;
        }
        match self.board.as_str() {
            "nrf52840-dk" => {
                let mut flash = Command::new("probe-run");
                flash.arg("--chip=nRF52840_xxAA");
                flash.arg(&elf);
                flash.exec();
            }
            "nrf52840-dongle" => {
                let mut nrfdfu = Command::new("nrfdfu");
                nrfdfu.arg(&elf);
                nrfdfu.spawn();
            }
            "solo" => {
                let hex = format!("{}.hex", elf);
                let mut objcopy = Command::new("arm-none-eabi-objcopy");
                objcopy.arg("-O");
                objcopy.arg("ihex");
                objcopy.arg(&elf);
                objcopy.arg(&hex);
                objcopy.spawn();
                let mut solo = Command::new("solo");
                solo.arg("program");
                solo.arg("bootloader");
                solo.arg(&hex);
                solo.spawn();
            }
            _ => unimplemented!("No flash support for {}.", self.board),
        }
    }
}

struct Command {
    command: std::process::Command,
}

impl Command {
    fn new(program: &str) -> Command {
        let program = match program {
            "cargo" => option_env!("CARGO").unwrap_or("cargo"),
            x => x,
        };
        Command { command: std::process::Command::new(program) }
    }

    fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) {
        self.command.env(key, value);
    }

    fn arg(&mut self, arg: impl AsRef<OsStr>) {
        self.command.arg(arg);
    }

    fn debug(&self) {
        for (k, v) in self.command.get_envs() {
            eprint!("{:?}={:?} ", k, v);
        }
        eprintln!("{:?}", self.command);
    }

    fn spawn(mut self) {
        self.debug();
        let code = self.command.spawn().unwrap().wait().unwrap().code().unwrap();
        if code != 0 {
            std::process::exit(0);
        }
    }

    fn exec(mut self) {
        self.debug();
        panic!("{}", self.command.exec());
    }
}

fn main() {
    Flags::from_args().execute();
}
