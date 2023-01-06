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

use rustc_demangle::demangle;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::path::Path;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Flags {
    /// Builds the firmware
    Build(Build),

    /// Starts a gdb session
    Gdb(Gdb),

    /// Runs rustfmt
    Fmt,

    /// Runs clippy
    Clippy,

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

    /// Show the (top N) stack sizes of the firmware
    #[structopt(long)]
    stack_sizes: Option<Option<usize>>,

    /// Flash (and run, if the board supports it) the firmware
    #[structopt(long)]
    flash: bool,
}

#[derive(Debug, StructOpt)]
enum Gdb {
    /// Starts a gdb server in this terminal
    Server,

    /// Starts a gdb session in this terminal (a server must be running)
    Client {
        /// Whether the firmware was built in release mode
        #[structopt(long)]
        release: bool,
    },
}

const BOARDS: &[&str] = &["nrf52840-dk", "nrf52840-dongle", "nrf52840-mdk-dongle", "solo"];
const TARGET: &str = "thumbv7em-none-eabi";

impl Flags {
    fn execute(self) {
        match self {
            Flags::Build(x) => x.execute(),
            Flags::Gdb(x) => x.execute(),
            Flags::Fmt => {
                for dir in ["xtask", "firmware"] {
                    let mut cargo = Command::new("cargo");
                    cargo.dir(dir);
                    cargo.args(["fmt", "--", "--check"]);
                    cargo.spawn();
                }
            }
            Flags::Clippy => {
                let clippy = |dir, args: &[&str]| {
                    let mut cargo = Command::new("cargo");
                    cargo.dir(dir);
                    cargo.arg("clippy");
                    cargo.args(args);
                    cargo.arg("--");
                    cargo.arg("--deny=warnings");
                    cargo.spawn();
                };
                clippy("xtask", &[]);
                for board in BOARDS {
                    clippy(
                        "firmware",
                        &[&format!("--features=board-{board}"), &format!("--target={TARGET}")],
                    );
                }
            }
            Flags::Test => {
                let mut cargo = Command::new("cargo");
                cargo.dir("firmware");
                cargo.arg("test");
                cargo.arg("--lib");
                cargo.exec();
            }
        }
    }
}

impl Build {
    fn execute(self) {
        let mut cargo = Command::new("cargo");
        let mut rustflags = vec!["-C link-arg=--nmagic", "-C link-arg=-Tlink.x"];
        if self.stack_sizes.is_some() {
            rustflags.push("-Z emit-stack-sizes");
            rustflags.push("-C link-arg=-Tstack-sizes.x");
        }
        cargo.env("ONEKIBU_MEMORY_X", format!("{}.x", self.board));
        cargo.dir("firmware");
        cargo.arg("build");
        cargo.arg(format!("--target={TARGET}"));
        cargo.arg(format!("--features=board-{}", self.board));
        if self.release {
            cargo.arg("--release");
            rustflags.push("-C codegen-units=1");
            rustflags.push("-C embed-bitcode=yes");
            rustflags.push("-C lto=fat");
            rustflags.push("-C opt-level=z");
        }
        let log = match &self.log {
            None if self.release => "off",
            None => "trace",
            Some(x) => x,
        };
        cargo.env("DEFMT_LOG", log);
        if log != "off" {
            rustflags.push("-C link-arg=-Tdefmt.x");
            cargo.arg("--features=log");
        }
        cargo.env("RUSTFLAGS", rustflags.join(" "));
        cargo.spawn();
        let elf = elf(self.release);
        if self.size {
            let mut size = Command::new("rust-size");
            size.arg(&elf);
            size.spawn();
        }
        if let Some(stack_sizes) = self.stack_sizes {
            let elf = std::fs::read(&elf).unwrap();
            let symbols = stack_sizes::analyze_executable(&elf).unwrap();
            assert!(symbols.have_32_bit_addresses);
            assert!(symbols.undefined.is_empty());
            let max_stack_sizes = stack_sizes.unwrap_or(10);
            let mut top_stack_sizes = BinaryHeap::new();
            for (address, symbol) in symbols.defined {
                let stack = match symbol.stack() {
                    None => continue,
                    Some(x) => x,
                };
                // Multiple symbols can have the same address. Just use the first name.
                assert!(!symbol.names().is_empty());
                let name = *symbol.names().first().unwrap();
                top_stack_sizes.push((Reverse(stack), address, name));
                if top_stack_sizes.len() > max_stack_sizes {
                    top_stack_sizes.pop();
                }
            }
            while let Some((Reverse(stack), address, name)) = top_stack_sizes.pop() {
                println!("{:#010x}\t{}\t{}", address, stack, demangle(name));
            }
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
            "nrf52840-mdk-dongle" => {
                let hex = hex(self.release);
                let mut uf2conv = Command::new("uf2conv.py");
                uf2conv.arg("--family=0xADA52840");
                uf2conv.arg(hex);
                uf2conv.spawn();
            }
            "solo" => {
                let hex = hex(self.release);
                let mut solo = Command::new("solo");
                solo.arg("program");
                solo.arg("bootloader");
                solo.arg(hex);
                solo.spawn();
            }
            _ => unimplemented!("No flash support for {}.", self.board),
        }
    }
}

impl Gdb {
    fn execute(self) {
        match self {
            Gdb::Server => {
                let mut jlink = Command::new("JLinkGDBServer");
                jlink.args(["-device", "nrf52840_xxaa"]);
                jlink.args(["-if", "swd"]);
                jlink.args(["-speed", "4000"]);
                jlink.args(["-port", "2331"]);
                jlink.exec();
            }
            Gdb::Client { release } => {
                let mut gdb = Command::new("gdb-multiarch");
                gdb.args(["-ex", &format!("file {}", elf(release))]);
                gdb.args(["-ex", "target remote localhost:2331"]);
                gdb.exec();
            }
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

    fn dir(&mut self, dir: impl AsRef<Path>) {
        self.command.current_dir(dir);
    }

    fn arg(&mut self, arg: impl AsRef<OsStr>) {
        self.command.arg(arg);
    }

    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) {
        self.command.args(args);
    }

    fn debug(&self) {
        if let Some(d) = self.command.get_current_dir() {
            eprint!("cd {d:?} && ");
        }
        for (k, v) in self.command.get_envs() {
            eprint!("{k:?}={v:?} ");
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

fn elf(release: bool) -> String {
    format!("target/{}/{}/onekibu", TARGET, if release { "release" } else { "debug" })
}

fn hex(release: bool) -> String {
    let elf = elf(release);
    let hex = format!("{elf}.hex");
    let mut objcopy = Command::new("arm-none-eabi-objcopy");
    objcopy.arg("-O");
    objcopy.arg("ihex");
    objcopy.arg(&elf);
    objcopy.arg(&hex);
    objcopy.spawn();
    hex
}

fn main() {
    Flags::from_args().execute();
}
