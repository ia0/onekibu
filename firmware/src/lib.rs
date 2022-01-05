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

#![no_std]

use defmt::Format;

pub struct Config {
    /// Maximum timestamp (timestamps wrap back to 0 after this value).
    pub maximum: usize,

    /// Time period after which the state may step without interaction.
    pub period: usize,
}

impl Config {
    /// Returns the difference between 2 timestamps.
    fn diff(&self, reference: usize, current: usize) -> usize {
        if current < reference {
            self.maximum - reference + current + 1
        } else {
            current - reference
        }
    }
}

#[derive(Format, Clone, Copy)]
pub struct Input {
    /// The current timestamp.
    pub timestamp: usize,
    /// Whether the button is being pressed.
    // TODO: Add a layer for debouncing? See https://github.com/TyberiusPrime/debouncing
    pub button: bool,
}

#[derive(Format, Clone, Copy)]
enum Bit {
    Zero,
    One,
    End,
    Cancel,
}

#[derive(Format, Clone, Copy)]
pub enum BitState {
    Ready,
    Short,
    Long,
    Cancel,
    Done,
}

struct BitLayer {
    config: Config,
    state: BitState,
    /// Timestamp of the last state change.
    reference: usize,
    /// Previous timestamp.
    previous: usize,
}

impl BitLayer {
    fn new(config: Config) -> BitLayer {
        BitLayer { config, state: BitState::Ready, reference: 0, previous: 0 }
    }

    fn step(&mut self, input: Input) -> Option<Bit> {
        if self.config.diff(self.previous, input.timestamp) > self.config.period / 4 {
            defmt::warn!(
                "Lag detected {} << {} ({})",
                self.previous,
                input.timestamp,
                self.config.period
            );
        }
        self.previous = input.timestamp;
        let timeout = self.config.diff(self.reference, input.timestamp) > self.config.period;
        use BitState::*;
        let (state, reset, bit) = match (self.state, input.button, timeout) {
            (Ready, false, _) => (Ready, true, None),
            (Ready, true, _) => (Short, true, None),
            (Short, false, _) => (Done, true, Some(Bit::Zero)),
            (Short, true, false) => (Short, false, None),
            (Short, true, true) => (Long, true, None),
            (Long, false, _) => (Done, true, Some(Bit::One)),
            (Long, true, false) => (Long, false, None),
            (Long, true, true) => (Cancel, true, None),
            (Cancel, false, _) => (Ready, true, Some(Bit::Cancel)),
            (Cancel, true, _) => (Cancel, true, None),
            (Done, false, false) => (Done, false, None),
            (Done, false, true) => (Ready, true, Some(Bit::End)),
            (Done, true, _) => (Short, true, None),
        };
        self.state = state;
        if reset {
            self.reference = input.timestamp;
        }
        bit
    }
}

#[derive(Format, Debug, Clone, Copy, PartialEq, Eq)]
enum Seq {
    Modifier(u8),
    Key(u8),
    Invalid,
    Cancel,
}

impl From<u8> for Seq {
    fn from(x: u8) -> Seq {
        use Seq::*;
        match x {
            224 ..= 231 => Modifier(1 << (x - 224)),
            _ => Key(x),
        }
    }
}

// TODO: Those are actually actions. Most of them are just key press+release. But we can add a
// PrepareSequence and CommitSequence actions. Keys in between are toggled. All keys are released
// automatically (if not already) during at the end of the sequence.

// TODO: Allow pre-configured sequence of keys (e.g. to output unicode with Ctrl+Shift+U xxx). See
// https://github.com/TyberiusPrime/KeyToKey/blob/91ba3fe917e626c820f681fd2e2a97637ef16344/src/lib.rs#L313-L322

// TODO: Add the default mapping in the README too.
/// Maps sequences to keycodes.
const MAP: [u8; 127] = [
    0,   //
    8,   // . E
    23,  // - T
    12,  // .. I
    4,   // .- A
    17,  // -. N
    16,  // -- M
    22,  // ... S
    24,  // ..- U
    21,  // .-. R
    26,  // .-- W
    7,   // -.. D
    14,  // -.- K
    10,  // --. G
    18,  // --- O
    11,  // .... H
    25,  // ...- V
    9,   // ..-. F
    44,  // ..-- Space
    15,  // .-.. L
    42,  // .-.- BSp
    19,  // .--. P
    13,  // .--- J
    5,   // -... B
    27,  // -..- X
    6,   // -.-. C
    28,  // -.-- Y
    29,  // --.. Z
    20,  // --.- Q
    43,  // ---. Tab
    40,  // ---- Enter
    34,  // ..... 5
    33,  // ....- 4
    224, // ...-. LCtrl
    32,  // ...-- 3
    225, // ..-.. LShift
    41,  // ..-.- Esc
    45,  // ..--. -/_
    31,  // ..--- 2
    226, // .-... LAlt
    // TODO: We also need RCtrl, RShift, and RAlt (used for compose)
    46,  // .-..- =/+
    47,  // .-.-. [/{
    48,  // .-.-- ]/}
    49,  // .--.. \/|
    51,  // .--.- ;/:
    52,  // .---. '/"
    30,  // .---- 1
    35,  // -.... 6
    53,  // -...- `/~
    54,  // -..-. ,/<
    55,  // -..-- ./>
    56,  // -.-.. //?
    76,  // -.-.- Delete
    101, // -.--. Applic (but actually Menu)
    0, 36, // --... 7
    0, 0, 0, 37, // ---.. 8
    0, 38, // ----. 9
    39, // ----- 0
    0, 79, // .....- Right
    82, // ....-. Up
    0, 81, // ...-.. Down
    0, 0, 0, 80, // ..-... Left
    0, 0, 0, 0, 0, 0, 0, 0, 77, // .-...- End
    75, // .-..-. PgUp
    0, 78, // .-.-.. PgDown
    0, 0, 0, 74, // .--... Home
    0, 0, 0, 0, 0, 0, 0, 0, 58, // -....- F1
    59, // -...-. F2
    60, // -...-- F3
    61, // -..-.. F4
    62, // -..-.- F5
    63, // -..--. F6
    64, // -..--- F7
    65, // -.-... F8
    66, // -.-..- F9
    67, // -.-.-. F10
    68, // -.-.-- F11
    69, // -.--.. F12
    0, 0, 0, 70,  // --.... PrtScr
    72,  // --...- Pause
    73,  // --..-. Insert
    154, // --..-- SysRq
    0, 0, 0, 0, 0, 57, // ---..- Caps Lock
    83, // ---.-. Num Lock
    71, // ---.-- Scroll Lock
    0, 0, 0, 0,
];

struct SeqLayer {
    state: usize, // < MAP.len()
}

impl SeqLayer {
    fn new() -> SeqLayer {
        SeqLayer { state: 0 }
    }

    fn step(&mut self, input: Bit) -> Option<Seq> {
        use Bit::*;
        let bit = match input {
            Zero => 0,
            One => 1,
            End if self.state < MAP.len() && MAP[self.state] > 0 => {
                let seq = MAP[self.state].into();
                self.state = 0;
                return Some(seq);
            }
            End if self.state < 255 => {
                defmt::warn!("Reserved sequence {:#b}", self.state);
                self.state = 0;
                return Some(Seq::Invalid);
            }
            End => {
                let seq = (self.state - 255) as u8;
                defmt::info!("Low-level sequence {:#b} {}", self.state, seq);
                self.state = 0;
                return Some(seq.into());
            }
            Cancel if self.state == 0 => {
                return Some(Seq::Cancel);
            }
            Cancel => {
                self.state = 0;
                return None;
            }
        };
        let new_state = 2 * self.state + bit + 1;
        if new_state < 511 {
            self.state = new_state;
        } else {
            defmt::warn!("Sequence too long {:#b}. Dropping bits.", self.state);
        }
        None
    }
}

// TODO: Make this a sequence of key presses and releases. Unreleased keys are released at the end
// of the sequence. A maximum of 6 non-modifiers can be pressed at the same time.
#[derive(Default, Clone, Copy)]
pub struct Output {
    pub modifiers: u8,
    pub key: u8,
}

pub struct State {
    bit: BitLayer,
    seq: SeqLayer,
    out: Output,
}

impl State {
    pub fn new(config: Config) -> State {
        let bit = BitLayer::new(config);
        let seq = SeqLayer::new();
        State { bit, seq, out: Output::default() }
    }

    pub fn step(&mut self, input: Input) -> Option<Output> {
        let bit = self.bit.step(input)?;
        let seq = self.seq.step(bit)?;
        defmt::trace!("{:?}", seq);
        match seq {
            Seq::Modifier(x) => self.out.modifiers |= x,
            Seq::Key(x) => {
                self.out.key = x;
                let out = self.out;
                self.out = Output::default();
                return Some(out);
            }
            Seq::Invalid | Seq::Cancel => self.out = Output::default(),
        };
        None
    }

    pub fn bit_state(&self) -> BitState {
        self.bit.state
    }
}

#[test]
fn keycodes() {
    #[track_caller]
    fn test(xs: &[u8], s: usize, q: Seq) {
        let mut r = 0;
        for &x in xs {
            match x {
                b'.' => r = 2 * r + 1,
                b'-' => r = 2 * r + 2,
                _ => unreachable!(),
            }
        }
        assert_eq!(r, s);
        assert_eq!(Seq::from(MAP[r]), q);
    }
    // Use https://www.win.tue.nl/~aeb/linux/kbd/scancodes-14.html to check the key.
    use Seq::*;
    test(b"", 0, Key(0));

    // Official codes
    test(b".-", 4, Key(4)); // A
    test(b"-...", 23, Key(5)); // B
    test(b"-.-.", 25, Key(6)); // C
    test(b"-..", 11, Key(7)); // D
    test(b".", 1, Key(8)); // E
    test(b"..-.", 17, Key(9)); // F
    test(b"--.", 13, Key(10)); // G
    test(b"....", 15, Key(11)); // H
    test(b"..", 3, Key(12)); // I
    test(b".---", 22, Key(13)); // J
    test(b"-.-", 12, Key(14)); // K
    test(b".-..", 19, Key(15)); // L
    test(b"--", 6, Key(16)); // M
    test(b"-.", 5, Key(17)); // N
    test(b"---", 14, Key(18)); // O
    test(b".--.", 21, Key(19)); // P
    test(b"--.-", 28, Key(20)); // Q
    test(b".-.", 9, Key(21)); // R
    test(b"...", 7, Key(22)); // S
    test(b"-", 2, Key(23)); // T
    test(b"..-", 8, Key(24)); // U
    test(b"...-", 16, Key(25)); // V
    test(b".--", 10, Key(26)); // W
    test(b"-..-", 24, Key(27)); // X
    test(b"-.--", 26, Key(28)); // Y
    test(b"--..", 27, Key(29)); // Z
    test(b".----", 46, Key(30)); // 1
    test(b"..---", 38, Key(31)); // 2
    test(b"...--", 34, Key(32)); // 3
    test(b"....-", 32, Key(33)); // 4
    test(b".....", 31, Key(34)); // 5
    test(b"-....", 47, Key(35)); // 6
    test(b"--...", 55, Key(36)); // 7
    test(b"---..", 59, Key(37)); // 8
    test(b"----.", 61, Key(38)); // 9
    test(b"-----", 62, Key(39)); // 0

    // Custom codes
    test(b"..--", 18, Key(44)); // Space
    test(b".-.-", 20, Key(42)); // BSp
    test(b"---.", 29, Key(43)); // Tab
    test(b"----", 30, Key(40)); // Enter
    test(b"...-.", 33, Modifier(1)); // LCtrl
    test(b"..-..", 35, Modifier(2)); // LShift
    test(b".-...", 39, Modifier(4)); // LAlt
    test(b"..-.-", 36, Key(41)); // Esc
    test(b"..--.", 37, Key(45)); // -/_
    test(b".-..-", 40, Key(46)); // =/+
    test(b".-.-.", 41, Key(47)); // [/{
    test(b".-.--", 42, Key(48)); // ]/}
    test(b".--..", 43, Key(49)); // \/|
    test(b".--.-", 44, Key(51)); // ;/:
    test(b".---.", 45, Key(52)); // '/"
    test(b"-...-", 48, Key(53)); // `/~
    test(b"-..-.", 49, Key(54)); // ,/<
    test(b"-..--", 50, Key(55)); // ./>
    test(b"-.-..", 51, Key(56)); // //?
    test(b"-.-.-", 52, Key(76)); // Delete
    test(b"-.--.", 53, Key(101)); // Applic (but actually Menu)
    test(b".....-", 64, Key(79)); // Right
    test(b"....-.", 65, Key(82)); // Up
    test(b"...-..", 67, Key(81)); // Down
    test(b"..-...", 71, Key(80)); // Left
    test(b".-...-", 80, Key(77)); // End
    test(b".-..-.", 81, Key(75)); // PgUp
    test(b".-.-..", 83, Key(78)); // PgDown
    test(b".--...", 87, Key(74)); // Home
    test(b"-....-", 96, Key(58)); // F1
    test(b"-...-.", 97, Key(59)); // F2
    test(b"-...--", 98, Key(60)); // F3
    test(b"-..-..", 99, Key(61)); // F4
    test(b"-..-.-", 100, Key(62)); // F5
    test(b"-..--.", 101, Key(63)); // F6
    test(b"-..---", 102, Key(64)); // F7
    test(b"-.-...", 103, Key(65)); // F8
    test(b"-.-..-", 104, Key(66)); // F9
    test(b"-.-.-.", 105, Key(67)); // F10
    test(b"-.-.--", 106, Key(68)); // F11
    test(b"-.--..", 107, Key(69)); // F12
    test(b"--....", 111, Key(70)); // PrtScr
    test(b"--...-", 112, Key(72)); // Pause
    test(b"--..-.", 113, Key(73)); // Insert
    test(b"--..--", 114, Key(154)); // SysRq
    test(b"---..-", 120, Key(57)); // Caps Lock
    test(b"---.-.", 121, Key(83)); // Num Lock
    test(b"---.--", 122, Key(71)); // Scroll Lock
}
