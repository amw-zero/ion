// Bit Twiddling Guide:
// var & FLAG != 0 checks if FLAG is enabled
// var & FLAG == 0 checks if FLAG is disabled
// var |= FLAG enables the FLAG
// var &= 255 ^ FLAG disables the FLAG
// var ^= FLAG swaps the state of FLAG

const BACKSL: u8 = 1;
const SQUOTE: u8 = 2;
const DQUOTE: u8 = 4;


#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Index {
    // TODO: Ranged and ID
    All
}

#[derive(Debug, PartialEq, Clone)]
pub enum WordToken<'a> {
    Normal(&'a str),
    Whitespace(&'a str),
    Tilde(&'a str),
    Brace(Vec<&'a str>),
    // Array(Vec<&str>, bool, Index)
    Variable(&'a str, bool),
    ArrayVariable(&'a str, bool, Index),
    ArrayProcess(&'a str, bool, Index),
    Process(&'a str, bool),
    // ArrayToString(&'a str, &'a str, &'a str, bool),
    // StringToArray(&'a str, &'a str, &'a str, bool),
}

pub struct WordIterator<'a> {
    data:          &'a str,
    read:          usize,
    flags:         u8,
}

impl<'a> WordIterator<'a> {
    pub fn new(data: &'a str) -> WordIterator<'a> {
        WordIterator { data: data, read: 0, flags: 0 }
    }

    // Contains the grammar for collecting whitespace characters
    fn whitespaces<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        self.read += 1;
        while let Some(character) = iterator.next() {
            if character == b' ' {
                self.read += 1;
            } else {
                return WordToken::Whitespace(&self.data[start..self.read]);
            }
        }

        WordToken::Whitespace(&self.data[start..self.read])
    }

    /// Contains the logic for parsing tilde syntax
    fn tilde<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read - 1;
        while let Some(character) = iterator.next() {
            match character {
                0...47 | 58...64 | 91...94 | 96 | 123...127 => {
                    return WordToken::Tilde(&self.data[start..self.read]);
                },
                _ => (),
            }
            self.read += 1;
        }

        WordToken::Tilde(&self.data[start..])
    }

    // Contains the logic for parsing braced variables
    fn braced_variable<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        while let Some(character) = iterator.next() {
            if character == b'}' {
                let output = &self.data[start..self.read];
                self.read += 1;
                return WordToken::Variable(output, self.flags & DQUOTE != 0);
            }
            self.read += 1;
        }

        // The validator at the frontend should catch unterminated braced variables.
        panic!("ion: fatal error with syntax validation parsing: unterminated braced variable");
    }

    /// Contains the logic for parsing variable syntax
    fn variable<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        self.read += 1;
        while let Some(character) = iterator.next() {
            match character {
                // If found, this is not a `Variable` but an `ArrayToString`
                // b'(' => {
                //     unimplemented!()
                // },
                // Only alphanumerical and underscores are allowed in variable names
                0...47 | 58...64 | 91...94 | 96 | 123...127 => {
                    return WordToken::Variable(&self.data[start..self.read], self.flags & DQUOTE != 0);
                },
                _ => (),
            }
            self.read += 1;
        }

        WordToken::Variable(&self.data[start..], self.flags & DQUOTE != 0)
    }

    /// Contains the logic for parsing array variable syntax
    fn array_variable<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        self.read += 1;
        while let Some(character) = iterator.next() {
            match character {
                // TODO: Detect Index
                // TODO: ArrayFunction
                // Only alphanumerical and underscores are allowed in variable names
                0...47 | 58...64 | 91...94 | 96 | 123...127 => {
                    return WordToken::Variable(&self.data[start..self.read], self.flags & DQUOTE != 0);
                },
                _ => (),
            }
            self.read += 1;
        }

        WordToken::ArrayVariable(&self.data[start..], self.flags & DQUOTE != 0, Index::All)
    }

    /// Contains the logic for parsing subshell syntax.
    fn process<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        let mut level = 0;
        while let Some(character) = iterator.next() {
            match character {
                _ if self.flags & BACKSL != 0     => self.flags ^= BACKSL,
                b'\\'                             => self.flags ^= BACKSL,
                b'\'' if self.flags & DQUOTE == 0 => self.flags ^= SQUOTE,
                b'"'  if self.flags & SQUOTE == 0 => self.flags ^= DQUOTE,
                b'$'  if self.flags & SQUOTE == 0 => {
                    if self.data.as_bytes()[self.read+1] == b'(' {
                        level += 1;
                    }
                },
                b')' if self.flags & SQUOTE == 0 => {
                    if level == 0 {
                        let output = &self.data[start..self.read];
                        self.read += 1;
                        return WordToken::Process(output, self.flags & DQUOTE != 0);
                    } else {
                        level -= 1;
                    }
                }
                _ => (),
            }
            self.read += 1;
        }

        // The validator at the frontend should catch unterminated processes.
        panic!("ion: fatal error with syntax validation: unterminated process");
    }

    /// Contains the logic for parsing array subshell syntax.
    fn array_process<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let start = self.read;
        let mut level = 0;
        while let Some(character) = iterator.next() {
            match character {
                _ if self.flags & BACKSL != 0     => self.flags ^= BACKSL,
                b'\\'                             => self.flags ^= BACKSL,
                b'\'' if self.flags & DQUOTE == 0 => self.flags ^= SQUOTE,
                b'"'  if self.flags & SQUOTE == 0 => self.flags ^= DQUOTE,
                b'@'  if self.flags & SQUOTE == 0 => {
                    if self.data.as_bytes()[self.read+1] == b'[' {
                        level += 1;
                    }
                },
                b']' if self.flags & SQUOTE == 0 => {
                    if level == 0 {
                        // TODO: Detect Index
                        let output = &self.data[start..self.read];
                        self.read += 1;
                        return WordToken::ArrayProcess(output, self.flags & DQUOTE != 0, Index::All);
                    } else {
                        level -= 1;
                    }
                }
                _ => (),
            }
            self.read += 1;
        }

        // The validator at the frontend should catch unterminated processes.
        panic!("ion: fatal error with syntax validation: unterminated array process");
    }

    /// Contains the grammar for parsing brace expansion syntax
    fn braces<I>(&mut self, iterator: &mut I) -> WordToken<'a>
        where I: Iterator<Item = u8>
    {
        let mut start = self.read;
        let mut level = 0;
        let mut elements = Vec::new();
        while let Some(character) = iterator.next() {
            match character {
                _ if self.flags & BACKSL != 0     => self.flags ^= BACKSL,
                b'\\'                             => self.flags ^= BACKSL,
                b'\'' if self.flags & DQUOTE == 0 => self.flags ^= SQUOTE,
                b'"'  if self.flags & SQUOTE == 0 => self.flags ^= DQUOTE,
                b','  if self.flags & (SQUOTE + DQUOTE) == 0 && level == 0 => {
                    elements.push(&self.data[start..self.read]);
                    start = self.read + 1;
                },
                b'{' if self.flags & (SQUOTE + DQUOTE) == 0 => level += 1,
                b'}' if self.flags & (SQUOTE + DQUOTE) == 0 => {
                    if level == 0 {
                        elements.push(&self.data[start..self.read]);
                        self.read += 1;
                        return WordToken::Brace(elements);
                    } else {
                        level -= 1;
                    }

                },
                _ => ()
            }
            self.read += 1;
        }

        panic!("ion: fatal error with syntax validation: unterminated brace")
    }
}

impl<'a> Iterator for WordIterator<'a> {
    type Item = WordToken<'a>;

    fn next(&mut self) -> Option<WordToken<'a>> {
        if self.read == self.data.len() { return None }

        let mut iterator = self.data.bytes().skip(self.read);
        let mut start = self.read;

        loop {
            if let Some(character) = iterator.next() {
                match character {
                    b'\'' if self.flags & DQUOTE == 0 => {
                        start += 1;
                        self.read += 1;
                        self.flags ^= SQUOTE;
                    },
                    b'"' if self.flags & SQUOTE == 0 => {
                        start += 1;
                        self.read += 1;
                        self.flags ^= DQUOTE;
                    }
                    b' ' if self.flags & (SQUOTE + DQUOTE) == 0 => {
                        return Some(self.whitespaces(&mut iterator));
                    }
                    b'~' if self.flags & (SQUOTE + DQUOTE) == 0 => {
                        self.read += 1;
                        return Some(self.tilde(&mut iterator));
                    },
                    b'{' if self.flags & (SQUOTE + DQUOTE) == 0 => {
                        self.read += 1;
                        return Some(self.braces(&mut iterator));
                    },
                    b'@' if self.flags & SQUOTE == 0 => {
                        match iterator.next() {
                            Some(b'[') => {
                                self.read += 2;
                                return Some(self.array_process(&mut iterator));
                            },
                            // Some(b'{') => {
                            //     self.read += 2;
                            //     return Some(self.braced_variable(&mut iterator));
                            // }
                            _ => {
                                self.read += 1;
                                return Some(self.array_variable(&mut iterator));
                            }
                        }
                    }
                    b'$' if self.flags & SQUOTE == 0 => {
                        match iterator.next() {
                            Some(b'(') => {
                                self.read += 2;
                                return Some(self.process(&mut iterator));
                            },
                            Some(b'{') => {
                                self.read += 2;
                                return Some(self.braced_variable(&mut iterator));
                            }
                            _ => {
                                self.read += 1;
                                return Some(self.variable(&mut iterator));
                            }
                        }
                    }
                    _ => { self.read += 1; break },
                }
            } else {
                return None
            }
        }

        while let Some(character) = iterator.next() {
            match character {
                _ if self.flags & BACKSL != 0     => self.flags ^= BACKSL,
                b'\\'                             => self.flags ^= BACKSL,
                b'\'' if self.flags & DQUOTE == 0 => {
                    self.flags ^= SQUOTE;
                    let output = &self.data[start..self.read];
                    self.read += 1;
                    return Some(WordToken::Normal(output));
                },
                b'"' if self.flags & SQUOTE == 0 => {
                    self.flags ^= DQUOTE;
                    let output = &self.data[start..self.read];
                    self.read += 1;
                    return Some(WordToken::Normal(output));
                },
                b' ' | b'{' if self.flags & (SQUOTE + DQUOTE) == 0 => {
                    return Some(WordToken::Normal(&self.data[start..self.read]));
                },
                b'$' | b'@' if self.flags & SQUOTE == 0 => {
                    return Some(WordToken::Normal(&self.data[start..self.read]));
                },
                _ => (),
            }
            self.read += 1;
        }

        if start == self.read {
            None
        } else {
            Some(WordToken::Normal(&self.data[start..]))
        }
    }
}

// TODO: Write More Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(input: &str, expected: Vec<WordToken>) {
        let mut correct = 0;
        for (actual, expected) in WordIterator::new(input).zip(expected.iter()) {
            assert_eq!(actual, *expected, "{:?} != {:?}", actual, expected);
            correct += 1;
        }
        assert_eq!(expected.len(), correct);
    }

    #[test]
    fn words_process_recursion() {
        let input = "echo $(echo $(echo one)) $(echo one $(echo two) three)";
        let expected = vec![
            WordToken::Normal("echo"),
            WordToken::Whitespace(" "),
            WordToken::Process("echo $(echo one)", false),
            WordToken::Whitespace(" "),
            WordToken::Process("echo one $(echo two) three", false),
        ];
        compare(input, expected);
    }

    #[test]
    fn words_process_with_quotes() {
        let input = "echo $(git branch | rg '[*]' | awk '{print $2}')";
        let expected = vec![
            WordToken::Normal("echo"),
            WordToken::Whitespace(" "),
            WordToken::Process("git branch | rg '[*]' | awk '{print $2}'", false),
        ];
        compare(input, expected);

        let input = "echo $(git branch | rg \"[*]\" | awk '{print $2}')";
        let expected = vec![
            WordToken::Normal("echo"),
            WordToken::Whitespace(" "),
            WordToken::Process("git branch | rg \"[*]\" | awk '{print $2}'", false),
        ];
        compare(input, expected);
    }

    #[test]
    fn test_words() {
        let input = "echo $ABC \"${ABC}\" one{$ABC,$ABC} ~ $(echo foo) \"$(seq 1 100)\"";
        let expected = vec![
            WordToken::Normal("echo"),
            WordToken::Whitespace(" "),
            WordToken::Variable("ABC", false),
            WordToken::Whitespace(" "),
            WordToken::Variable("ABC", true),
            WordToken::Whitespace(" "),
            WordToken::Normal("one"),
            WordToken::Brace(vec!["$ABC", "$ABC"]),
            WordToken::Whitespace(" "),
            WordToken::Tilde("~"),
            WordToken::Whitespace(" "),
            WordToken::Process("echo foo", false),
            WordToken::Whitespace(" "),
            WordToken::Process("seq 1 100", true)
        ];
        compare(input, expected);
    }
}
