use std::{default, fmt::Display};

#[derive(Debug, Clone, Copy, Default)]
pub struct Token<'src> {
    pub kind: TKind<'src>,
    pub span: Span<'src>,
}

impl<'src> std::fmt::Display for TKind<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TKind::Int(x) => f.write_fmt(format_args!("Int literal {}", x)),
            TKind::Str(x) => f.write_fmt(format_args!(
                "String literal \"{}\"",
                str::from_utf8(x).unwrap()
            )),
            TKind::Ident(x) => f.write_fmt(format_args!("Identifer \"{}\"", x)),
            x => f.write_fmt(format_args!("{:?}", x)),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum TKind<'src> {
    #[default]
    Eof,
    Whitespace,
    Int(u64),
    Bool(bool),
    Str(&'src [u8]),
    Ident(&'src str),

    // Declarator keywords
    Let,
    Fn,
    Struct,
    Global,

    // Control flow keywords
    While,
    Continue,
    Break,
    If,
    Else,
    Return,

    // Delimiters
    LParen, // (
    RParen, // )
    LCurly, // {
    RCurly, // }
    LBrack, // [
    RBrack, // ]

    // Separators
    Comma, // ,
    Dot,   // .
    Colon, // :
    Semi,  // ;
    Arrow, // ->

    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    And,     // &
    Or,      // |
    Caret,   // ^
    Bang,    // !
    Eq,      // =
    AndAnd,  // &&
    OrOr,    // ||

    // Relationals
    EqEq,   // ==
    BangEq, // !=
    Lt,     // <
    Gt,     // >
    LtEq,   // <=
    GtEq,   // >=
}

/// Represents top-level "things", which includes:
/// - function definitions
/// - type definitions
/// - global variables
#[derive(Debug, Clone)]
pub enum OKind<'src, T> {
    Fn {
        name: &'src str,
        returns: T,
        args: Vec<Symbol<'src, T>>,
        body: Stmt<'src, T>,
    },
    Global(Symbol<'src, T>),
    Struct {
        name: &'src str,
        fields: Vec<Symbol<'src, T>>,
    },
}

#[derive(Debug, Clone)]
pub struct Object<'src, T> {
    pub kind: OKind<'src, T>,
    pub span: Span<'src>,
}

#[derive(Debug, Clone)]
pub enum RawType<'src> {
    Unknown,
    Base(&'src str),
    Pointer(Box<RawType<'src>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
    Infer,

    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    Bool,
    Void,

    Function {
        args: Vec<ResolvedType>,
        returns: Box<ResolvedType>,
    },
    Pointer(Box<ResolvedType>),
    // Struct(&'src str, Vec<Field>),
}

#[derive(Debug, Clone, Copy)]
pub struct Symbol<'src, T> {
    pub name: &'src str,
    pub ty: T,
}

impl<'src> std::fmt::Display for RawType<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RawType::Unknown => "{unknown}".to_string(),
            RawType::Base(s) => s.to_string(),
            RawType::Pointer(raw_type) => format!("*{raw_type}"),
        };
        f.write_str(&s)
    }
}

impl std::fmt::Display for ResolvedType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ResolvedType::Infer => "{unknown}".to_string(),
            ResolvedType::U8 => "u8".to_string(),
            ResolvedType::U16 => "u16".to_string(),
            ResolvedType::U32 => "u32".to_string(),
            ResolvedType::U64 => "u64".to_string(),
            ResolvedType::I8 => "i8".to_string(),
            ResolvedType::I16 => "i16".to_string(),
            ResolvedType::I32 => "i32".to_string(),
            ResolvedType::I64 => "i64".to_string(),
            ResolvedType::Bool => "bool".to_string(),
            ResolvedType::Void => "void".to_string(),
            ResolvedType::Function { args, returns } => {
                let args = args
                    .iter()
                    .map(|arg| format!("{arg}"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("fn({args}) -> {returns}")
            }
            ResolvedType::Pointer(resolved_type) => format!("*{resolved_type}"),
        };
        f.write_str(&s)
    }
}

#[derive(Debug, Clone)]
pub struct Stmt<'src, T> {
    pub kind: SKind<'src, T>,
    pub span: Span<'src>,
}

impl<'src, T> Stmt<'src, T> {
    pub fn new(kind: SKind<'src, T>, span: Span<'src>) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum SKind<'src, T> {
    Let {
        lhs: Symbol<'src, T>,
        rhs: Expr<'src, T>,
    },
    While {
        cond: Expr<'src, T>,
        body: Box<Stmt<'src, T>>,
    },
    Continue,
    Break,
    If {
        cond: Expr<'src, T>,
        then_: Box<Stmt<'src, T>>,
        else_: Box<Stmt<'src, T>>,
    },
    Return(Expr<'src, T>),
    Block(Vec<Stmt<'src, T>>),
    Expr(Expr<'src, T>),
}

#[derive(Clone, Copy, Default)]
pub struct Span<'src> {
    pub lo: usize,
    pub hi: usize,
    pub src: &'src [u8],
}

impl<'src> std::fmt::Debug for Span<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("lo", &self.lo)
            .field("hi", &self.hi)
            // Omit `internal_secret` entirely
            .finish()
    }
}

impl<'src> std::fmt::Display for Span<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let line_start = self.src[..self.lo]
            .iter()
            .rposition(|&b| b == b'\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let line_end = self.src[self.lo..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|pos| self.lo + pos)
            .unwrap_or(self.src.len());

        let line_text =
            std::str::from_utf8(&self.src[line_start..line_end]).unwrap_or("<invalid utf8>");

        let before_span_count = self.lo.saturating_sub(line_start);

        let is_multiline = self.hi > line_end;
        let effective_hi = if is_multiline { line_end } else { self.hi };

        let caret_count = effective_hi.saturating_sub(self.lo).max(1);

        let spaces = " ".repeat(before_span_count);
        let mut carets = "^".repeat(caret_count);

        if is_multiline {
            carets.push_str("...");
        }

        write!(f, "{}\n{}{}", line_text, spaces, carets)
    }
}

impl<'src> Span<'src> {
    pub fn merge(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            src: self.src,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr<'src, T> {
    pub kind: EKind<'src, T>,
    pub ty: T,
    pub span: Span<'src>,
}

impl<'src, T> std::fmt::Display for EKind<'src, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EKind::Symbol(symbol) => "variable",
            EKind::Int(_) => "int",
            EKind::Bool(_) => "bool",
            EKind::Nothing => "nothing",
            EKind::Str(items) => "string",
            EKind::Call { callee, args } => "function call",
            EKind::Unary { op, rhs } => "unary operation",
            EKind::Bin { op, lhs, rhs } => "binary operation",
            EKind::FieldAccess { lhs, rhs } => "field access",
            EKind::Index { lhs, rhs } => "array index",
        };
        f.write_str(s)
    }
}

impl<'src, T> Expr<'src, T> {
    pub fn new(kind: EKind<'src, T>, ty: T, span: Span<'src>) -> Self {
        Self { kind, ty, span }
    }
}

#[derive(Debug, Clone)]
pub enum EKind<'src, T> {
    Symbol(Symbol<'src, T>),
    Int(u64),
    Bool(bool),
    Nothing,
    Str(&'src [u8]),
    Call {
        callee: Box<Expr<'src, T>>,
        args: Vec<Expr<'src, T>>,
    },
    Unary {
        op: UnOp,
        rhs: Box<Expr<'src, T>>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr<'src, T>>,
        rhs: Box<Expr<'src, T>>,
    },
    FieldAccess {
        lhs: Box<Expr<'src, T>>,
        rhs: Box<Expr<'src, T>>,
    },
    Index {
        lhs: Box<Expr<'src, T>>,
        rhs: Box<Expr<'src, T>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Negate,
    Not,
    AddrOf,
    Deref,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    LogOr,
    LogAnd,
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
    Ne,
}
