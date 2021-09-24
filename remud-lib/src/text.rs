pub fn word_list(mut words: Vec<String>) -> String {
    if words.is_empty() {
        String::new()
    } else if words.len() == 1 {
        words.pop().unwrap()
    } else if words.len() == 2 {
        words.join(" and ")
    } else {
        let last = words.pop().unwrap();
        let joined = words.join(", ");
        format!("{}, and {}", joined, last)
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
