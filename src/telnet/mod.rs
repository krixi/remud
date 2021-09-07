use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
};

use bitflags::bitflags;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use lazy_static::lazy_static;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Copy, Clone)]
pub enum Negotiate {
    Dont,
    Do,
    Wont,
    Will,
}

impl Negotiate {
    fn byte(&self) -> u8 {
        match self {
            Negotiate::Dont => DONT,
            Negotiate::Do => DO,
            Negotiate::Wont => WONT,
            Negotiate::Will => WILL,
        }
    }
}

impl TryFrom<u8> for Negotiate {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            DONT => Ok(Negotiate::Dont),
            DO => Ok(Negotiate::Do),
            WONT => Ok(Negotiate::Wont),
            WILL => Ok(Negotiate::Will),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum OptionCode {
    TerminalType,
    Naws,
    Unknown(u8),
}

impl OptionCode {
    fn byte(self) -> u8 {
        match self {
            OptionCode::TerminalType => 24,
            OptionCode::Naws => 31,
            OptionCode::Unknown(option) => option,
        }
    }
}

impl From<u8> for OptionCode {
    fn from(value: u8) -> Self {
        match value {
            24 => OptionCode::TerminalType,
            31 => OptionCode::Naws,
            option => OptionCode::Unknown(option),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Frame {
    Data(Bytes),
    Negotiate(Negotiate, OptionCode),
    Subnegotiate(OptionCode, Bytes),
    Command(u8),
}

impl From<Frame> for Bytes {
    fn from(frame: Frame) -> Self {
        let mut bytes = BytesMut::new();

        match frame {
            Frame::Data(data) => {
                bytes.reserve(data.len());
                bytes.put(data);
            }
            Frame::Negotiate(command, option) => {
                bytes.reserve(3);
                bytes.extend(&[IAC, command.byte(), option.byte()]);
            }
            Frame::Subnegotiate(option, data) => {
                bytes.reserve(5 + data.len());
                bytes.extend(&[IAC, SB, option.byte()]);
                bytes.extend(data);
                bytes.extend(&[IAC, SE]);
            }
            Frame::Command(command) => {
                bytes.reserve(2);
                bytes.extend(&[IAC, command]);
            }
        }

        bytes.freeze()
    }
}

pub struct Codec;

impl Decoder for Codec {
    type Item = Frame;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            return Ok(None);
        }

        let frame = if src[0] == IAC {
            // IAC means "Interpret as Command" and indicates the following bytes form a telnet command
            if src.len() > 1 {
                match src[1] {
                    IAC => {
                        // Two IACs in a row is an escaped IAC, send as a data frame
                        src.advance(2);
                        let mut data = BytesMut::new();
                        data.put_u8(IAC);
                        Some(Frame::Data(data.freeze()))
                    }
                    WILL | WONT | DO | DONT => {
                        // Negotiations resemble IAC <NEGOTIATION_COMMAND> <OPTION CODE>
                        if src.len() > 2 {
                            if let Ok(negotiate) = Negotiate::try_from(src[1]) {
                                let negotiation =
                                    Frame::Negotiate(negotiate, OptionCode::from(src[2]));
                                src.advance(3);
                                Some(negotiation)
                            } else {
                                tracing::error!("Invalid negotiation received: {:?}", src);
                                None
                            }
                        } else {
                            None
                        }
                    }
                    SB => {
                        // Subnegotations resemble IAC SB <OPTION CODE> <DATA> IAC SE
                        if src.len() > 4 {
                            src.as_ref()
                                .windows(2)
                                .position(|b| b[0] == IAC && b[1] == SE)
                                .map(|suffix_pos| {
                                    let mut data = src.split_to(suffix_pos);
                                    src.advance(2);

                                    let prefix = data.split_to(3);
                                    Frame::Subnegotiate(OptionCode::from(prefix[2]), data.freeze())
                                })
                        } else {
                            None
                        }
                    }
                    _ => {
                        let command = Frame::Command(src[1]);
                        src.advance(2);
                        Some(command)
                    }
                }
            } else {
                // incomplete command
                None
            }
        } else if let Some(iac_pos) = src.as_ref().iter().position(|b| *b == IAC) {
            Some(Frame::Data(src.split_to(iac_pos).freeze()))
        } else {
            Some(Frame::Data(src.split_to(src.len()).freeze()))
        };

        tracing::debug!("-> {:?}", frame);

        Ok(frame)
    }
}

impl Encoder<Frame> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, frame: Frame, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        tracing::debug!("<- {:?}", frame);

        let bytes = Bytes::from(frame);
        dst.reserve(bytes.len());
        dst.put(bytes.as_ref());
        Ok(())
    }
}

bitflags! {
    pub struct TerminalFeatures: u16 {
        const ANSI = 0b0000_0000_0000_0001;
        const VT100 = 0b0000_0000_0000_0010;
        const UTF_8 = 0b0000_0000_0000_0100;
        const COLORS_256 = 0b0000_0000_0000_1000;
        const MOUSE_TRACKING = 0b0000_0000_0001_0000;
        const OSC_COLOR_PALETTE = 0b0000_0000_0010_0000;
        const SCREEN_READER = 0b0000_0000_0100_0000;
        const PROXY = 0b0000_0000_1000_0000;
        const TRUE_COLOR = 0b0000_0001_0000_0000;
        const MNES = 0b0000_0010_0000_0000;
        const MSLP = 0b0000_0100_0000_0000;
    }
}

pub struct Telnet {
    options: Options,
    terminal_selection_state: TerminalSelectionState,
}

impl Telnet {
    pub fn new() -> Self {
        Telnet {
            options: Options::default(),
            terminal_selection_state: TerminalSelectionState::Begin,
        }
    }

    pub fn initiate(&mut self) -> Vec<Frame> {
        let mut frames = Vec::new();
        if let Some(frame) = self.options.enable(OptionCode::TerminalType) {
            frames.push(frame);
        }
        if let Some(frame) = self.options.enable(OptionCode::Naws) {
            frames.push(frame);
        }
        frames
    }

    pub fn negotiate(&mut self, command: Negotiate, option: OptionCode) -> Vec<Frame> {
        let mut frames = Vec::new();
        if let Some(response) = self.options.negotiate(command, option) {
            frames.push(response);
        }
        frames
    }

    pub fn subnegotiate(&mut self, option: OptionCode, mut data: Bytes) -> Vec<Frame> {
        let mut frames = Vec::new();
        if self.options.enabled(option) {
            match option {
                OptionCode::TerminalType => {
                    if let TerminalSelectionState::List(list) = &mut self.terminal_selection_state {
                        data.advance(1);
                        if let Some(response) = list.checked_add_type(data) {
                            frames.push(response);
                        } else if let Some(best) = list.best() {
                            tracing::info!("Chosen TType: {:?}", best);
                            if best.mtts {
                                self.terminal_selection_state =
                                    TerminalSelectionState::Done(Some(best));
                            } else {
                                if let Some(request) =
                                    self.options.disable(OptionCode::TerminalType)
                                {
                                    frames.push(request);
                                }
                                self.terminal_selection_state =
                                    TerminalSelectionState::Select(best);
                            }
                        } else {
                            if let Some(request) = self.options.disable(OptionCode::TerminalType) {
                                frames.push(request);
                            }
                            self.terminal_selection_state = TerminalSelectionState::Done(None);
                        }
                    }
                }
                OptionCode::Naws | OptionCode::Unknown(_) => (),
            }
        }
        frames
    }

    pub fn configure(&mut self) -> Vec<Frame> {
        let mut frames = Vec::new();

        match &mut self.terminal_selection_state {
            TerminalSelectionState::Begin => {
                if self.options.enabled(OptionCode::TerminalType) {
                    self.terminal_selection_state =
                        TerminalSelectionState::List(TerminalTypes::new());
                    let mut bytes = BytesMut::new();
                    bytes.put_u8(TERMINAL_TYPE_SEND);
                    frames.push(Frame::Subnegotiate(
                        OptionCode::TerminalType,
                        bytes.freeze(),
                    ));
                } else {
                    self.terminal_selection_state = TerminalSelectionState::Done(None);
                }
            }
            TerminalSelectionState::Select(best) => {
                if self.options.disabled(OptionCode::TerminalType) {
                    if let Some(request) = self.options.enable(OptionCode::TerminalType) {
                        frames.push(request);
                        for _ in 0..=best.index {
                            let mut bytes = BytesMut::new();
                            bytes.put_u8(TERMINAL_TYPE_SEND);
                            let next =
                                Frame::Subnegotiate(OptionCode::TerminalType, bytes.freeze());
                            frames.push(next);
                        }
                    }
                    self.terminal_selection_state =
                        TerminalSelectionState::Done(Some(best.clone()));
                }
            }
            TerminalSelectionState::List(_) | TerminalSelectionState::Done(_) => (),
        }

        frames
    }

    pub fn configured(&self) -> bool {
        !self.options.negotiating()
            && matches! {self.terminal_selection_state, TerminalSelectionState::Done(_)}
    }

    pub fn features(&self) -> Option<TerminalFeatures> {
        if let TerminalSelectionState::Done(Some(terminal_type)) = &self.terminal_selection_state {
            return Some(terminal_type.features);
        }
        None
    }
}

const IAC: u8 = 255;
const DONT: u8 = 254;
const DO: u8 = 253;
const WONT: u8 = 252;
const WILL: u8 = 251;
const SB: u8 = 250;
const SE: u8 = 240;

const TERMINAL_TYPE_SEND: u8 = 1;

lazy_static! {
    static ref ALLOWED_OPTIONS: HashSet<OptionCode> = {
        let mut allowed = HashSet::new();
        allowed.insert(OptionCode::TerminalType);
        allowed.insert(OptionCode::Naws);
        allowed
    };
}

enum OptionState {
    No,
    Yes,
    WantNo,
    WantYes,
}

#[derive(Default)]
struct Options {
    state: HashMap<OptionCode, OptionState>,
    queue_opposite: HashSet<OptionCode>,
}

impl Options {
    fn enabled(&self, option: OptionCode) -> bool {
        matches!(self.state.get(&option), Some(OptionState::Yes))
    }

    fn disabled(&self, option: OptionCode) -> bool {
        matches!(self.state.get(&option), Some(OptionState::No))
    }

    fn negotiating_option(&self, option: OptionCode) -> bool {
        self.state.get(&option).map_or(false, |state| match state {
            OptionState::No | OptionState::Yes => false,
            OptionState::WantNo | OptionState::WantYes => true,
        })
    }

    fn negotiating(&self) -> bool {
        return self
            .state
            .keys()
            .any(|option| self.negotiating_option(*option))
            || !self.queue_opposite.is_empty();
    }

    fn enable(&mut self, option: OptionCode) -> Option<Frame> {
        if !ALLOWED_OPTIONS.contains(&option) {
            return None;
        }

        match self.state.entry(option).or_insert(OptionState::No) {
            OptionState::No => {
                self.state.insert(option, OptionState::WantYes);
                Some(Frame::Negotiate(Negotiate::Do, option))
            }
            OptionState::Yes => None,
            OptionState::WantNo => {
                self.queue_opposite.insert(option);
                None
            }
            OptionState::WantYes => {
                self.queue_opposite.remove(&option);
                None
            }
        }
    }

    fn disable(&mut self, option: OptionCode) -> Option<Frame> {
        match self.state.entry(option).or_insert(OptionState::No) {
            OptionState::No => None,
            OptionState::Yes => {
                self.state.insert(option, OptionState::WantNo);
                Some(Frame::Negotiate(Negotiate::Dont, option))
            }
            OptionState::WantNo => {
                self.queue_opposite.remove(&option);
                None
            }
            OptionState::WantYes => {
                self.queue_opposite.insert(option);
                None
            }
        }
    }

    fn negotiate(&mut self, command: Negotiate, option: OptionCode) -> Option<Frame> {
        match command {
            Negotiate::Will => match self.state.entry(option).or_insert(OptionState::No) {
                OptionState::No => {
                    if ALLOWED_OPTIONS.contains(&option) {
                        self.state.insert(option, OptionState::Yes);
                        Some(Frame::Negotiate(Negotiate::Do, option))
                    } else {
                        Some(Frame::Negotiate(Negotiate::Dont, option))
                    }
                }
                OptionState::Yes => None,
                OptionState::WantNo => {
                    if self.queue_opposite.contains(&option) {
                        self.state.insert(option, OptionState::Yes);
                        self.queue_opposite.remove(&option);
                        None
                    } else {
                        self.state.insert(option, OptionState::No);
                        None
                    }
                }
                OptionState::WantYes => {
                    if self.queue_opposite.contains(&option) {
                        self.state.insert(option, OptionState::WantNo);
                        self.queue_opposite.remove(&option);
                        Some(Frame::Negotiate(Negotiate::Dont, option))
                    } else {
                        self.state.insert(option, OptionState::Yes);
                        None
                    }
                }
            },
            Negotiate::Wont => match self.state.entry(option).or_insert(OptionState::No) {
                OptionState::No => None,
                OptionState::Yes => {
                    self.state.insert(option, OptionState::No);
                    Some(Frame::Negotiate(Negotiate::Dont, option))
                }
                OptionState::WantNo => {
                    if self.queue_opposite.contains(&option) {
                        self.state.insert(option, OptionState::WantYes);
                        self.queue_opposite.remove(&option);
                        Some(Frame::Negotiate(Negotiate::Do, option))
                    } else {
                        self.state.insert(option, OptionState::No);
                        None
                    }
                }
                OptionState::WantYes => {
                    self.state.insert(option, OptionState::No);
                    if self.queue_opposite.contains(&option) {
                        self.queue_opposite.remove(&option);
                        None
                    } else {
                        None
                    }
                }
            },
            Negotiate::Dont | Negotiate::Do => None,
        }
    }
}

enum TerminalSelectionState {
    Begin,
    List(TerminalTypes),
    Select(TerminalType),
    Done(Option<TerminalType>),
}

#[derive(Default)]
struct TerminalTypes {
    types: Vec<Bytes>,
}

impl TerminalTypes {
    fn new() -> Self {
        TerminalTypes { types: Vec::new() }
    }

    fn checked_add_type(&mut self, ttype: Bytes) -> Option<Frame> {
        if self.types.last().map_or(false, |last| last == &ttype) {
            tracing::info!("TType List: {:?}", self.types);
        } else {
            self.types.push(ttype);

            let mut bytes = BytesMut::new();
            bytes.put_u8(TERMINAL_TYPE_SEND);

            return Some(Frame::Subnegotiate(
                OptionCode::TerminalType,
                bytes.freeze(),
            ));
        }

        None
    }

    fn best(&self) -> Option<TerminalType> {
        // Check for MTTS support
        if self
            .types
            .last()
            .map_or(false, |b| b.to_ascii_uppercase().starts_with(b"MTTS"))
        {
            let mut features = TerminalFeatures::empty();

            let name = self.types.last().expect("last TType exists").clone();
            let mut mtts_flags = name.clone();
            mtts_flags.advance(5);
            if let Ok(flag_string) = std::str::from_utf8(&mtts_flags) {
                if let Ok(int_flags) = flag_string.parse::<u16>() {
                    if let Some(flags) = TerminalFeatures::from_bits(int_flags) {
                        features = flags;
                    }
                }
            }

            return Some(TerminalType {
                name,
                mtts: true,
                index: self.types.len() - 1,
                features,
            });
        }

        // Check for common terminal emulators with or without additional color support
        if let Some(index) = self
            .types
            .iter()
            .position(|b| b[..].to_ascii_uppercase().starts_with(b"XTERM"))
        {
            let mut features = TerminalFeatures::ANSI
                | TerminalFeatures::VT100
                | TerminalFeatures::COLORS_256
                | TerminalFeatures::MOUSE_TRACKING;

            if supports_true_color(&self.types[index]) {
                features |= TerminalFeatures::TRUE_COLOR;
            }

            return Some(TerminalType {
                name: self.types[index].clone(),
                mtts: false,
                index,
                features,
            });
        }

        if let Some(index) = self
            .types
            .iter()
            .position(|b| b[..].to_ascii_uppercase().starts_with(b"VT100"))
        {
            let mut features = TerminalFeatures::ANSI | TerminalFeatures::VT100;

            if supports_true_color(&self.types[index]) {
                features = features | TerminalFeatures::TRUE_COLOR | TerminalFeatures::COLORS_256;
            }

            if supports_256_color(&self.types[index]) {
                features |= TerminalFeatures::COLORS_256;
            }

            return Some(TerminalType {
                name: self.types[index].clone(),
                mtts: false,
                index,
                features,
            });
        }

        if let Some(index) = self
            .types
            .iter()
            .position(|b| b[..].to_ascii_uppercase().starts_with(b"ANSI"))
        {
            let mut features = TerminalFeatures::ANSI;

            if supports_true_color(&self.types[index]) {
                features = features | TerminalFeatures::TRUE_COLOR | TerminalFeatures::COLORS_256;
            }

            if supports_256_color(&self.types[index]) {
                features |= TerminalFeatures::COLORS_256;
            }

            return Some(TerminalType {
                name: self.types[index].clone(),
                mtts: false,
                index,
                features,
            });
        }

        None
    }
}

#[derive(Debug, Clone)]
struct TerminalType {
    pub name: Bytes,
    pub mtts: bool,
    pub index: usize,
    pub features: TerminalFeatures,
}

fn supports_true_color(terminal_type: &Bytes) -> bool {
    terminal_type.to_ascii_uppercase().ends_with(b"TRUECOLOR")
}

fn supports_256_color(terminal_type: &Bytes) -> bool {
    terminal_type.to_ascii_uppercase().ends_with(b"256COLOR")
}
