use std::{default, fmt::Display};

#[derive(Debug, Clone, Copy, Default)]
pub struct Token<'src> {
    pub kind: TKind<'src>,
    pub span: Span,
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
        body: SKind<'src, T>,
    },
    Global(Symbol<'src, T>),
    Struct {
        name: &'src str,
        fields: Vec<Symbol<'src, T>>,
    },
}

#[derive(Debug, Clone)]
pub enum RawType<'src> {
    Unknown,
    Base(&'src str),
    Pointer(Box<RawType<'src>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedType {
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

#[derive(Debug, Clone)]
pub enum SKind<'src, T> {
    Let {
        lhs: Symbol<'src, T>,
        rhs: Expr<'src, T>,
    },
    While {
        cond: Expr<'src, T>,
        body: Box<SKind<'src, T>>,
    },
    Continue,
    Break,
    If {
        cond: Expr<'src, T>,
        then_: Box<SKind<'src, T>>,
        else_: Box<SKind<'src, T>>,
    },
    Return(Expr<'src, T>),
    Block(Vec<SKind<'src, T>>),
    Expr(Expr<'src, T>),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Span(pub usize, pub usize);

impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}-{}", self.0, self.1))
    }
}

impl Span {
    pub fn merge(self, other: Self) -> Self {
        Self(self.0.min(other.0), self.1.max(other.1))
    }
}

#[derive(Debug, Clone)]
pub struct Expr<'src, T> {
    pub kind: EKind<'src, T>,
    pub ty: T,
}

impl<'src> Expr<'src, RawType<'src>> {
    pub fn unknown(kind: EKind<'src, RawType<'src>>, span: Span) -> Self {
        Self {
            kind,
            ty: RawType::Unknown,
            // span,
        }
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
