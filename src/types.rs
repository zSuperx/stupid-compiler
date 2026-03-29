#[derive(Debug, Clone)]
pub struct Token<'src> {
    pub kind: Kind<'src>,
    pub length: usize,
}

impl<'src> std::fmt::Display for Kind<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Int(x) => f.write_fmt(format_args!("Int literal {}", x)),
            Kind::Str(x) => f.write_fmt(format_args!(
                "String literal \"{}\"",
                str::from_utf8(x).unwrap()
            )),
            Kind::Ident(x) => f.write_fmt(format_args!("Identifer \"{}\"", x)),
            x => f.write_fmt(format_args!("{:?}", x)),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Kind<'src> {
    EOF,
    Int(u64),
    Str(&'src [u8]),
    Ident(&'src str),

    // Declarator keywords
    Let,
    Fn,

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
    Or,     // |
    Caret,   // ^
    Bang,    // !
    Eq,      // =
    AndAnd,  // &&
    OrOr,  // ||

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
#[derive(Debug)]
pub enum Object<'src> {
    FnDef {
        name: &'src str,
        returns: Option<Type<'src>>,
        args: Vec<Variable<'src>>,
        body: Stmt<'src>,
    },
    // I'll add the others later...
}

#[derive(Debug)]
pub enum Type<'src> {
    Base(&'src str),
    Pointer(Box<Type<'src>>),
}

#[derive(Debug)]
pub struct Variable<'src> {
    pub name: String,
    pub ty: Type<'src>,
}

#[derive(Debug)]
pub enum Stmt<'src> {
    Let {
        lhs: Variable<'src>,
        rhs: Expr<'src>,
    },
    While {
        cond: Expr<'src>,
        body: Box<Stmt<'src>>,
    },
    Continue,
    Break,
    If {
        cond: Expr<'src>,
        then_: Box<Stmt<'src>>,
        else_: Box<Stmt<'src>>,
    },
    Return(Option<Expr<'src>>),
    Block(Vec<Stmt<'src>>),
    Assign {
        lhs: Expr<'src>,
        rhs: Expr<'src>,
    },
    Expr(Expr<'src>),
}

#[derive(Debug, Clone)]
pub enum Expr<'src> {
    Ident(&'src str),
    Int(u64),
    Bool(bool),
    Str(&'src [u8]),
    Call {
        callee: Box<Expr<'src>>,
        args: Vec<Expr<'src>>,
    },
    Unary {
        op: UnOp,
        rhs: Box<Expr<'src>>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr<'src>>,
        rhs: Box<Expr<'src>>,
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
    FieldAccess,
    Index,
    LogOr,
    LogAnd,
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
    Ne,
}
