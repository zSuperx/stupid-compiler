use crate::types::*;
use std::{io::Read, iter::Peekable};

pub struct Lexer<'src> {
    src: &'src [u8],
    buf: Vec<u8>,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src [u8]) -> Self {
        Self { src, buf: vec![] }
    }

    fn read_num(&mut self) -> Option<Token<'src>> {
        self.buf.clear();
        let mut cursor = 0;

        // Make sure the number starts with a digit
        let first = *self.src.get(cursor)?;
        if !first.is_ascii_digit() {
            return None;
        }

        // Now read all digits and underscores
        while let Some(c) = self.src.get(cursor)
            && b"0123456789_".contains(c)
        {
            if *c != b'_' {
                self.buf.push(*c);
            }
            cursor += 1;
        }

        let kind = str::from_utf8(&self.buf)
            .map(|s| u64::from_str_radix(s, 10).expect("LEXER: Integer literal too large"))
            .ok()
            .map(Kind::Int)?;

        // How far did we traverse to complete this token?
        let length = cursor;
        // On a successful parse, advance the cursor
        self.src = &self.src[cursor..];

        Some(Token { length, kind })
    }

    // This returns either an Ident or a Keyword, depending on what the string equates to
    fn read_word(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        // Identifiers can only start with letters or underscores
        let first = *self.src.get(cursor)?;
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return None;
        }

        while let Some(c) = self.src.get(cursor)
            && (c.is_ascii_alphanumeric() || *c == b'_')
        {
            cursor += 1;
        }

        let s = str::from_utf8(self.src.get(0..cursor)?)
            .expect("LEXER: Non-utf8 characters are not supported");

        let kind = match s {
            "let" => Kind::Let,
            "fn" => Kind::Fn,
            "while" => Kind::While,
            "continue" => Kind::Continue,
            "break" => Kind::Break,
            "if" => Kind::If,
            "else" => Kind::Else,
            "return" => Kind::Return,
            _ => Kind::Ident(s),
        };

        // How far did we traverse to complete this token?
        let length = cursor;
        // On a successful parse, advance the cursor
        self.src = &self.src[cursor..];

        Some(Token { length, kind })
    }

    // This reader will always return None, but a "successful" read will advance the cursor.
    // This is done to make the function pointer the same type as the others so it can be used in
    // funky ways :)
    fn read_whitespace(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        while let Some(c) = self.src.get(cursor)
            && c.is_ascii_whitespace()
        {
            cursor += 1;
        }
        self.src = &self.src[cursor..];
        None
    }

    fn read_strlit(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        let first = *self.src.get(cursor)?;
        if first == b'"' {
            cursor += 1;
        } else {
            return None;
        }

        loop {
            let curr = *self.src.get(cursor).expect("LEXER: Unclosed quote");
            cursor += 1;

            match curr {
                b'"' => break,
                b'\\' => {
                    cursor += 1;
                }
                _ => {}
            }
        }

        // +1/-1 to disclude the surrounding "..."
        let kind = Kind::Str(self.src.get(0..cursor)?);
        let length = cursor;
        self.src = &self.src[cursor..];
        Some(Token { length, kind })
    }

    fn read_punct(&mut self) -> Option<Token<'src>> {
        let known_punctuators = &["==", "!=", "<=", ">=", "->", "&&", "||", "<<", ">>"];

        let mut length = 0;
        let mut found_long_punct = false;
        let src = self.src.get(0..)?;
        for p in known_punctuators {
            if src.starts_with(p.as_bytes()) {
                length = p.len();
                found_long_punct = true;
                break;
            }
        }

        let first = *src.first()?;

        if !found_long_punct {
            if !(first.is_ascii_punctuation() && first != b'_') {
                return None;
            }
            length = 1;
        }

        use Kind::*;
        let kind = match src.get(0..length)? {
            // Delimiters
            b"(" => LParen,
            b")" => RParen,
            b"{" => LCurly,
            b"}" => RCurly,
            b"[" => LBrack,
            b"]" => RBrack,
            // Separators
            b"," => Comma,
            b"." => Dot,
            b":" => Colon,
            b";" => Semi,
            b"->" => Arrow,
            // Operators
            b"+" => Plus,    // +
            b"-" => Minus,   // -
            b"*" => Star,    // *
            b"/" => Slash,   // /
            b"%" => Percent, // %
            b"&" => And,     // &
            b"|" => Or,     // |
            b"^" => Caret,   // ^
            b"!" => Bang,    // !
            b"=" => Eq,      // =
            b"&&" => AndAnd,
            b"||" => OrOr,
            // Relationals
            b"==" => EqEq,
            b"!=" => BangEq,
            b"<" => Lt,
            b">" => Gt,
            b"<=" => LtEq,
            b">=" => GtEq,
            x => panic!("Unknown token: \"{}\"", str::from_utf8(x).unwrap()),
        };

        self.src = &self.src[length..];
        return Some(Token { length, kind });
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_whitespace()
            .or_else(|| self.read_word())
            .or_else(|| self.read_num())
            .or_else(|| self.read_strlit())
            .or_else(|| self.read_punct())
    }
}

#[cfg(test)]
mod tests {
    use crate::tokenize::{Lexer, Token};

    #[test]
    fn test_read_num() {
        let src = b"123_220_199";
        let mut lexer = Lexer::new(src);

        let x = lexer.read_num();
        println!("{x:?}");
    }

    #[test]
    fn test_read_ident() {
        let src = b"Hi there";
        let mut lexer = Lexer::new(src);

        let x = lexer.read_word();
        println!("{x:?}");
    }

    #[test]
    fn test_read_punct() {
        let src = b"!=";
        let mut lexer = Lexer::new(src);

        let x = lexer.read_punct();
        println!("{x:?}");
    }

    #[test]
    fn test_read_whitespace() {
        let src = b" 123";
        let mut lexer = Lexer::new(src);
        lexer.read_whitespace();
        let x = lexer.read_num();
        assert!(x.is_some());
        println!("{x:?}");
    }

    #[test]
    fn test_iterator() {
        let src = br#"
            fn main() -> i32 {
                let s = "bobby";
                let y = 69-5;!!
            }
"#;
        let mut lexer = Lexer::new(src);

        lexer.for_each(|t| println!("{}", t.kind));
    }
}
