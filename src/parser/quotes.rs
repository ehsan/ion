use std::str;

bitflags! {
    pub struct Flags : u8 {
        const SQUOTE = 1;
        const DQUOTE = 2;
        const TRIM   = 4;
        const ARRAY  = 8;
        const COMM   = 16;
        const EOF    = 32;
    }
}

pub(crate) struct QuoteTerminator {
    buffer:     String,
    eof:        Option<String>,
    eof_buffer: String,
    array:      usize,
    read:       usize,
    flags:      Flags,
}

impl QuoteTerminator {
    pub(crate) fn new(input: String) -> QuoteTerminator {
        QuoteTerminator {
            buffer:     input,
            eof:        None,
            eof_buffer: String::new(),
            array:      0,
            read:       0,
            flags:      Flags::empty(),
        }
    }

    pub(crate) fn append(&mut self, input: String) {
        if self.eof.is_none() {
            self.buffer.push_str(if self.flags.contains(TRIM) { input.trim() } else { &input });
        } else {
            self.eof_buffer.push_str(&input);
        }
    }

    pub(crate) fn check_termination(&mut self) -> bool {
        let mut eof_line = None;
        let eof = self.eof.clone();
        let status = if let Some(ref eof) = eof {
            let line = &self.eof_buffer;
            eof_line = Some([&line, "\n"].concat());
            line.trim() == eof
        } else {
            {
                let mut instance = Flags::empty();
                {
                    let mut bytes = self.buffer.bytes().skip(self.read);
                    while let Some(character) = bytes.next() {
                        self.read += 1;
                        match character {
                            b'\\' => { let _ = bytes.next(); },
                            b'\'' if !self.flags.intersects(DQUOTE) => self.flags ^= SQUOTE,
                            b'"' if !self.flags.intersects(SQUOTE) => self.flags ^= DQUOTE,
                            b'<' if !self.flags.contains(SQUOTE | DQUOTE) => {
                                let as_bytes = self.buffer.as_bytes();
                                if Some(&b'<') == as_bytes.get(self.read) {
                                    self.read += 1;
                                    if Some(&b'<') != as_bytes.get(self.read) {
                                        let eof_phrase = unsafe {
                                            str::from_utf8_unchecked(&as_bytes[self.read..])
                                        };
                                        self.eof = Some(eof_phrase.trim().to_owned());
                                        instance |= EOF;
                                        break;
                                    }
                                }
                            }
                            b'[' if !self.flags.intersects(DQUOTE | SQUOTE) => {
                                self.flags |= ARRAY;
                                self.array += 1;
                            }
                            b']' if !self.flags.intersects(DQUOTE | SQUOTE) => {
                                self.array -= 1;
                                if self.array == 0 { self.flags -= ARRAY }
                            }
                            b'#' if !self.flags.intersects(DQUOTE | SQUOTE) => {
                                if self.read > 1 {
                                    let character = self.buffer.as_bytes().get(self.read - 2).unwrap();
                                    if [b' ', b'\n'].contains(character) {
                                        instance |= COMM;
                                        break
                                    }
                                } else {
                                    instance |= COMM;
                                    break
                                }
                            }
                            _ => (),
                        }
                    }
                }
                if instance.contains(EOF) {
                    self.buffer.push('\n');
                    return false;
                } else if instance.contains(COMM) {
                    self.buffer.truncate(self.read - 1);
                    return !self.flags.intersects(SQUOTE | DQUOTE | ARRAY);
                }
            }

            if self.flags.intersects(SQUOTE | DQUOTE | ARRAY) {
                if let Some(b'\\') = self.buffer.bytes().last() {
                    let _ = self.buffer.pop();
                    self.read -= 1;
                    self.flags |= TRIM;
                } else {
                    self.read += 1;
                    self.buffer.push(if self.flags.contains(ARRAY) { ' ' } else { '\n' });
                }
                false
            } else {
                if let Some(b'\\') = self.buffer.bytes().last() {
                    let _ = self.buffer.pop();
                    self.read -= 1;
                    self.flags |= TRIM;
                    false
                } else {
                    // If the last two bytes are either '&&' or '||', we aren't terminated yet.
                    let bytes = self.buffer.as_bytes();
                    if bytes.len() >= 2 {
                        let bytes = &bytes[bytes.len() - 2..];
                        bytes != &[b'&', b'&'] && bytes != &[b'|', b'|']
                    } else {
                        true
                    }
                }
            }
        };

        if let Some(line) = eof_line {
            self.buffer.push_str(&line);
        }
        if self.eof.is_some() {
            self.eof_buffer.clear();
            if status {
                self.eof = None;
            }
        }
        status
    }

    pub(crate) fn consume(self) -> String { self.buffer }
}
