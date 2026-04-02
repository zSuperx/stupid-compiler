use crate::types::*;

pub struct Lexer<'src> {
    src: &'src [u8],
    rest: &'src [u8],
    cursor: usize,
    row: usize,
    col: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src [u8]) -> Self {
        Self {
            rest: src,
            src,
            cursor: 0,
            row: 0,
            col: 0,
        }
    }

    fn make_token(&mut self, kind: TKind<'src>, lo: usize) -> Token<'src> {
        let hi = self.cursor;
        Token {
            kind,
            span: Span {
                lo,
                hi,
                src: self.src,
                row: self.row,
                col: self.col,
            },
        }
    }

    fn consume(&mut self) -> Option<u8> {
        self.cursor += 1;
        match self.rest.get(self.cursor - 1).copied() {
            Some(c) => {
                if c == b'\n' {
                    self.row += 1;
                    self.col = 0;
                } else {
                    self.col += 1;
                }
                Some(c)
            }
            None => None,
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.rest.get(self.cursor).copied()
    }

    fn read_num(&mut self) -> Option<Token<'src>> {
        let start = self.cursor;
        let mut buf = vec![];

        // Make sure the number starts with a digit
        let first = self.peek()?;
        if !first.is_ascii_digit() {
            return None;
        }

        // Now read all digits and underscores
        while let Some(c) = self.peek()
            && b"0123456789_".contains(&c)
        {
            let c = self.consume()?;
            if c != b'_' {
                buf.push(c);
            }
        }

        let kind = str::from_utf8(&buf)
            .map(|s| s.parse().expect("LEXER: Integer literal too large"))
            .ok()
            .map(TKind::Int)?;
        // On a successful parse, advance the cursor
        Some(self.make_token(kind, start))
    }

    // This returns either an Ident or a Keyword, depending on what the string equates to
    fn read_word(&mut self) -> Option<Token<'src>> {
        let start = self.cursor;
        // Identifiers can only start with letters or underscores
        let first = self.peek()?;
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return None;
        }

        while let Some(c) = self.peek()
            && (c.is_ascii_alphanumeric() || c == b'_')
        {
            self.consume()?;
        }

        let s = str::from_utf8(self.rest.get(start..self.cursor)?)
            .expect("LEXER: Non-utf8 characters are not supported");

        let kind = match s {
            "let" => TKind::Let,
            "fn" => TKind::Fn,
            "struct" => TKind::Struct,
            "global" => TKind::Global,
            "while" => TKind::While,
            "continue" => TKind::Continue,
            "break" => TKind::Break,
            "if" => TKind::If,
            "else" => TKind::Else,
            "return" => TKind::Return,
            "true" => TKind::Bool(true),
            "false" => TKind::Bool(false),
            _ => TKind::Ident(s),
        };
        Some(self.make_token(kind, start))
    }

    // This reader will always return None, but a "successful" read will advance the cursor.
    // This is done to make the function pointer the same type as the others so it can be used in
    // funky ways :)
    fn read_whitespace(&mut self) -> Option<Token<'src>> {
        while let Some(c) = self.peek()
            && c.is_ascii_whitespace()
        {
            self.consume()?;
        }
        None
    }

    fn read_strlit(&mut self) -> Option<Token<'src>> {
        let start = self.cursor;
        if self.peek()? == b'"' {
            self.consume();
        } else {
            return None;
        }

        loop {
            let curr = self.consume().expect("LEXER: Unclosed quote");
            match curr {
                b'"' => break,
                b'\\' => {
                    self.consume().expect("LEXER: Unclosed quote");
                }
                _ => {}
            }
        }

        // +1/-1 to disclude the surrounding "..."
        let kind = TKind::Str(self.rest.get(start + 1..self.cursor - 1)?);
        Some(self.make_token(kind, start))
    }

    fn read_punct(&mut self) -> Option<Token<'src>> {
        let start = self.cursor;
        let known_punctuators = &["==", "!=", "<=", ">=", "->", "&&", "||", "<<", ">>"];

        let mut length = 1;
        let src = self.rest.get(start..)?;
        for p in known_punctuators {
            if src.starts_with(p.as_bytes()) {
                length = p.len();
                break;
            }
        }

        let first = self.peek()?;

        if length == 1 {
            if !first.is_ascii_punctuation() || first == b'_' {
                return None;
            }
        }

        let s = (0..length)
            .map(|_| self.consume())
            .flatten()
            .collect::<Vec<_>>();

        use TKind::*;
        let kind = match s.as_slice() {
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
            b"|" => Or,      // |
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

        Some(self.make_token(kind, start))
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
