use crate::types::*;

pub struct Lexer<'src> {
    src: &'src [u8],
    rest: &'src [u8],
    position: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src [u8]) -> Self {
        Self {
            rest: src,
            src,
            position: 0,
        }
    }

    fn make_token(&mut self, kind: TKind<'src>, length: usize) -> Token<'src> {
        let start = self.position;
        self.position += length;
        let end = self.position;
        Token {
            kind,
            span: Span(start, end),
        }
    }

    fn read_num(&mut self) -> Option<Token<'src>> {
        let mut buf = vec![];
        let mut cursor = 0;

        // Make sure the number starts with a digit
        let first = *self.rest.get(cursor)?;
        if !first.is_ascii_digit() {
            return None;
        }

        // Now read all digits and underscores
        while let Some(c) = self.rest.get(cursor)
            && b"0123456789_".contains(c)
        {
            if *c != b'_' {
                buf.push(*c);
            }
            cursor += 1;
        }

        let kind = str::from_utf8(&buf)
            .map(|s| s.parse().expect("LEXER: Integer literal too large"))
            .ok()
            .map(TKind::Int)?;

        // How far did we traverse to complete this token?
        let length = cursor;
        // On a successful parse, advance the cursor
        self.rest = &self.rest[cursor..];
        Some(self.make_token(kind, length))
    }

    // This returns either an Ident or a Keyword, depending on what the string equates to
    fn read_word(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        // Identifiers can only start with letters or underscores
        let first = *self.rest.get(cursor)?;
        if !(first.is_ascii_alphabetic() || first == b'_') {
            return None;
        }

        while let Some(c) = self.rest.get(cursor)
            && (c.is_ascii_alphanumeric() || *c == b'_')
        {
            cursor += 1;
        }

        let s = str::from_utf8(self.rest.get(0..cursor)?)
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

        // How far did we traverse to complete this token?
        let length = cursor;
        // On a successful parse, advance the cursor
        self.rest = &self.rest[cursor..];

        Some(self.make_token(kind, length))
    }

    // This reader will always return None, but a "successful" read will advance the cursor.
    // This is done to make the function pointer the same type as the others so it can be used in
    // funky ways :)
    fn read_whitespace(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        while let Some(c) = self.rest.get(cursor)
            && c.is_ascii_whitespace()
        {
            cursor += 1;
        }
        self.rest = &self.rest[cursor..];
        None
    }

    fn read_strlit(&mut self) -> Option<Token<'src>> {
        let mut cursor = 0;
        let first = *self.rest.get(cursor)?;
        if first == b'"' {
            cursor += 1;
        } else {
            return None;
        }

        loop {
            let curr = *self.rest.get(cursor).expect("LEXER: Unclosed quote");
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
        let kind = TKind::Str(self.rest.get(0..cursor)?);
        let length = cursor;
        self.rest = &self.rest[cursor..];
        Some(self.make_token(kind, length))
    }

    fn read_punct(&mut self) -> Option<Token<'src>> {
        let known_punctuators = &["==", "!=", "<=", ">=", "->", "&&", "||", "<<", ">>"];

        let mut length = 0;
        let mut found_long_punct = false;
        let src = self.rest.get(0..)?;
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

        use TKind::*;
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

        self.rest = &self.rest[length..];
        Some(self.make_token(kind, length))
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
