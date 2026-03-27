#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    length: usize,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Num(i) => f.write_fmt(format_args!("{i}")),
            TokenKind::Str(s) => f.write_fmt(format_args!("\"{s}\"")),
            TokenKind::Ident(i) => f.write_fmt(format_args!("identifier \"{i}\"")),
            TokenKind::Keyword(k) => f.write_fmt(format_args!("{k}")),
            TokenKind::Punct(p) => f.write_fmt(format_args!("{p}")),
            TokenKind::EOF => f.write_fmt(format_args!("EOF token")),
        }
    }
}

#[derive(Debug)]
pub enum TokenKind {
    Num(i64),
    Str(String),
    Ident(String),
    Keyword(String),
    Punct(String),
    EOF,
}

pub struct Lexer {
    file: String,
    curr_row: usize,
    curr_col: usize,
    stream: Vec<char>,
}

impl Lexer {
    pub fn new(path: &str) -> Self {
        let stream: Vec<_> = std::fs::read_to_string(path).unwrap().chars().collect();
        Self {
            curr_row: 0,
            curr_col: 0,
            file: path.to_string(),
            stream,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        let mut i = 0;

        let readers = &[read_strlit, read_string, read_punctuator, read_intlit];

        while let Some(mut src) = self.stream.get(i..)
            && i < self.stream.len()
        {
            if src[0].is_whitespace() {
                i += 1;
                continue;
            }

            if let Some(tok) = readers.iter().find_map(|f| f(src)) {
                i += tok.length;
                tokens.push(tok);
                continue;
            }
        }
        tokens
    }
}

fn read_intlit(src: &[char]) -> Option<Token> {
    if !src.get(0)?.is_digit(10) {
        return None;
    }

    let mut buf = String::new();
    let mut i = 0;
    while src.get(i).is_some_and(|c| c.is_digit(10)) {
        buf.push(src[i]);
        i += 1;
    }

    Some(Token {
        length: buf.len(),
        kind: TokenKind::Num(buf.parse().unwrap()),
    })
}

fn read_string(src: &[char]) -> Option<Token> {
    if !is_ident_start(src.get(0)?) {
        return None;
    };

    let mut buf = String::new();
    let mut i = 0;
    while src.get(i).is_some_and(is_ident) {
        buf.push(src[i]);
        i += 1;
    }

    #[rustfmt::skip]
    let known_keywords = &[
        "return", "if", "else", "for", "while", "sizeof", "struct", "union", "void", "typedef",
        "enum", "static", "goto", "break", "continue", "switch", "case", "default", "do",
        "const", "volatile", "restrict", "typeof", "asm", "fn", "u8", "u32",
    ];

    Some(Token {
        length: buf.len(),
        kind: if known_keywords.contains(&buf.as_str()) {
            TokenKind::Keyword(buf)
        } else {
            TokenKind::Ident(buf)
        },
    })
}

fn read_punctuator(src: &[char]) -> Option<Token> {
    if !src.get(0)?.is_ascii_punctuation() {
        return None;
    }

    let known_punctuators = &[
        "->", "<<=", ">>=", "...", "==", "!=", "<=", ">=", "->", "+=", "-=", "*=", "/=", "++",
        "--", "%=", "&=", "|=", "^=", "&&", "||", "<<", ">>", "##",
    ];

    for p in known_punctuators {
        if src
            .get(0..p.len())
            .is_some_and(|c| c.iter().copied().eq(p.chars()))
        {
            return Some(Token {
                kind: TokenKind::Punct(p.to_string()),
                length: p.len(),
            });
        }
    }

    src.get(0).map(|c| Token {
        kind: TokenKind::Punct(c.to_string()),
        length: 1,
    })
}

fn read_strlit(src: &[char]) -> Option<Token> {
    let mut iter = src.iter().enumerate();

    if *iter.next()?.1 != '"' {
        return None;
    }

    let mut buf = String::new();

    while let Some((pos, c)) = iter.next() {
        if *c == '\n' {
            break;
        }
        match *c {
            '"' => {
                buf.push('\0');
                return Some(Token {
                    length: pos + 1, // Count both "s, but don't count the \0
                    kind: TokenKind::Str(buf),
                });
            }
            '\\' => {
                let Some((_, escaped)) = iter.next() else {
                    break;
                };
                let c = match *escaped {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '0' => '\0',
                    _ => panic!("Invalid escape character"),
                };
                buf.push(c);
            }
            c => buf.push(c),
        }
    }
    panic!("Illegal string literal. Did you forget to add a closing \"?");
    None
}

fn is_ident(c: &char) -> bool {
    c.is_alphanumeric() || *c == '_'
}

fn is_ident_start(c: &char) -> bool {
    c.is_alphabetic() || *c == '_'
}
