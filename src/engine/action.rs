use std::str::FromStr;

use crate::engine::world::Direction;

pub enum Action {
    CreateRoom { direction: Option<Direction> },
    Look,
    // Room {
    //     room: RoomId,
    //     subcommand: RoomCommand,
    // },
    Say { message: String },
    Shutdown,
}

impl Action {
    pub fn parse(input: &str) -> Result<Action, String> {
        let mut tokenizer = Tokenizer::new(input);
        if let Some(token) = tokenizer.next() {
            match token.to_lowercase().as_str() {
                "look" => Ok(Action::Look),
                "room" => parse_room(tokenizer),
                "say" => Ok(Action::Say {
                    message: tokenizer.rest().to_string(),
                }),
                "shutdown" => Ok(Action::Shutdown),
                _ => Err("I don't know what that means.".to_string()),
            }
        } else {
            Err("Go on, then.".to_string())
        }
    }
}

// Valid shapes:
// room new - creates a new unlinked room
// room new [Direction] - creates a room to the [Direction] of this one with a two way link
fn parse_room(mut tokenizer: Tokenizer) -> Result<Action, String> {
    if let Some(subcommand) = tokenizer.next() {
        match subcommand.to_lowercase().as_str() {
            "new" => {
                let direction = if let Some(direction) = tokenizer.next() {
                    match Direction::from_str(direction) {
                        Ok(direction) => Some(direction),
                        Err(_) => return Err(format!("'{}' is not a valid direction.", direction)),
                    }
                } else {
                    None
                };

                Ok(Action::CreateRoom { direction })
            }
            s => Err(format!("'{}' is not a valid room subcommand.", s)),
        }
    } else {
        Err("'room' requires a subcommand.".to_string())
    }
}

// https://nitschinger.at/Text-Analysis-in-Rust-Tokenization/
pub struct Tokenizer<'a> {
    input: &'a str,
    byte_offset: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Tokenizer {
            input,
            byte_offset: 0,
        }
    }

    pub fn rest(&self) -> &'a str {
        &self.input[self.byte_offset..]
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        // Accounts for skipped whitespace at the beginning of the next token
        let mut skipped_bytes = 0;

        // Iterate through whitespace indices with various offsets
        for (byte_index, c) in self.input[self.byte_offset..]
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
        {
            let char_len = c.len_utf8();

            // Leading whitespace should be consumed
            if byte_index - skipped_bytes == 0 {
                skipped_bytes += char_len;
                continue;
            }

            // We found non-leading whitespace, return the token in between
            let slice =
                &self.input[self.byte_offset + skipped_bytes..self.byte_offset + byte_index];
            self.byte_offset += byte_index + char_len;
            return Some(slice);
        }

        // If there is no trailing whitespace, consume the rest as the final token
        if self.byte_offset < self.input.len() {
            let slice = &self.input[self.byte_offset + skipped_bytes..];
            self.byte_offset = self.input.len();
            Some(slice)
        } else {
            None
        }
    }
}
