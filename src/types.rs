use std::collections::HashMap;

pub type TypeId = usize;
#[derive(Debug)]
pub struct Context {
    // The original source code
    pub source: Vec<u8>,
    // A String<->Symbol map that lets us refer to strings with usize
    pub interner: StringStore,
    // SymbolId can be used to map to SymbolData, which holds info like type, addressed, etc
    pub symbols: Vec<SymbolData>,
    // TypeId can be used to map to TypeData
    pub types: Vec<TypeData>,
}

impl Context {
    pub fn new(src: Vec<u8>) -> Self {
        Self {
            source: src,
            interner: StringStore::new(),
            symbols: vec![],
            types: vec![],
        }
    }

    pub fn to_symbol(&mut self, name: &str) -> Symbol {
        // If we've already added the string to the store, get its ID
        if let Some(id) = self.interner.mapping.get(name) {
            return *id;
        } else {
            // Else add it
            let next_id = Symbol(self.interner.store.len());
            self.interner.mapping.insert(name.to_string(), next_id);
            self.interner.store.push(name.to_string());
            next_id
        }
    }

    pub fn lookup_symbol(&self, id: Symbol) -> &str {
        &self.interner.store[id.0]
    }

    pub fn declare_type(&mut self, name: Symbol, size: usize) -> TypeId {
        let id = self.types.len();
        self.types.push(TypeData {
            id,
            name,
            size,
            ..Default::default()
        });
        id
    }

    pub fn declare_local(&mut self, name: Symbol, ty: Type) -> SymbolId {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }

    pub fn declare_function(&mut self, name: Symbol, ty: Type) -> SymbolId {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }

    pub fn declare_global(&mut self, name: Symbol, ty: Type) -> usize {
        let id = self.symbols.len();
        self.symbols.push(SymbolData {
            id,
            name,
            ty,
            addressed: true,
        });
        id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct Symbol(pub usize);

#[derive(Debug)]
pub struct SymbolData {
    pub id: usize,
    pub name: Symbol,
    pub ty: TypeId,
    pub addressed: bool,
}

#[derive(Debug, Default)]
pub struct TypeData {
    pub id: usize,
    pub name: Symbol,
    pub size: usize,
    pub is_primitive: bool,
    pub is_integral: bool,
    pub is_signed: bool,
}

#[derive(Debug)]
pub struct StringStore {
    mapping: HashMap<String, Symbol>,
    store: Vec<String>,
}

impl StringStore {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            store: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Token {
    pub kind: TKind,
    pub span: Span,
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum TKind {
    #[default]
    Eof,
    Int(u64),
    Bool(bool),
    Str(Symbol),
    Ident(Symbol),

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

pub type SymbolId = usize;

/// Represents top-level "things", which includes:
/// - function definitions
/// - type definitions
/// - global variables
#[derive(Debug, Clone)]
pub enum OKind {
    Fn {
        name: SymbolId,
        returns: Type,
        args: Vec<SymbolId>,
        body: Stmt,
    },
    Global {
        lhs: SymbolId,
        rhs: Expr,
    },
    Struct {
        name: SymbolId,
        fields: Vec<SymbolId>,
    },
}

#[derive(Debug, Clone)]
pub struct Object {
    pub kind: OKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TyKind {
    // After the parsing stage, MOST nodes will have an Infer type
    // After the resolve stage, ALL nodes should have a concrete type
    Infer,
    // This is for CUSTOM type names. The
    Unresolved(Symbol),

    Base,
    Function { args: Vec<TypeId>, returns: TypeId },
    Pointer(TypeId),
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: SKind,
    pub span: Span,
}

impl Stmt {
    pub fn new(kind: SKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone)]
pub enum SKind {
    Let {
        lhs: SymbolId,
        rhs: Expr,
    },
    While {
        cond: Expr,
        body: Box<Stmt>,
    },
    Continue,
    Break,
    If {
        cond: Expr,
        then_: Box<Stmt>,
        else_: Box<Stmt>,
    },
    Return(Expr),
    Block(Vec<Stmt>),
    Expr(Expr),
}

#[derive(Clone, Copy, Default, PartialEq, PartialOrd)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
    pub row: usize,
    pub col: usize,
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("lo", &self.lo)
            .field("hi", &self.hi)
            // Omit `internal_secret` entirely
            .finish()
    }
}

impl Span {
    fn print_span(&self, src: &str) {
        let line_start = src[..self.lo]
            .chars()
            .rev()
            .position(|b| b == '\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let line_end = src[self.lo..]
            .chars()
            .position(|b| b == '\n')
            .map(|pos| self.lo + pos)
            .unwrap_or(src.len());

        let line_text = &src[line_start..line_end];

        let before_span_count = self.lo.saturating_sub(line_start);

        let is_multiline = self.hi > line_end;
        let effective_hi = if is_multiline { line_end } else { self.hi };

        let caret_count = effective_hi.saturating_sub(self.lo).max(1);

        let spaces = " ".repeat(before_span_count);
        let mut carets = "^".repeat(caret_count);

        if is_multiline {
            carets.push_str("...");
        }

        println!(
            "\n\n{}:{}:\n{}\n{}{}",
            self.row + 1,
            self.col,
            line_text,
            spaces,
            carets
        )
    }
}

impl Span {
    pub fn merge(self, other: Self) -> Self {
        let smaller = if self.lo <= other.lo { &self } else { &other };
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
            row: smaller.row,
            col: smaller.col,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: EKind,
    pub ty: TyKind,
    pub span: Span,
}

impl std::fmt::Display for EKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EKind::Unresolved(symbol) => "unresolved variable",
            EKind::Variable(_) => "variable",
            EKind::Int(_) => "int",
            EKind::Bool(_) => "bool",
            EKind::Nothing => "nothing",
            EKind::Str(_) => "string",
            EKind::Call { .. } => "function call",
            EKind::Unary { .. } => "unary operation",
            EKind::Bin { .. } => "binary operation",
            EKind::FieldAccess { .. } => "field access",
            EKind::Index { .. } => "array index",
        };
        f.write_str(s)
    }
}

impl Expr {
    pub fn new(kind: EKind, ty: TyKind, span: Span) -> Self {
        Self { kind, ty, span }
    }
}

#[derive(Debug, Clone)]
pub enum EKind {
    Unresolved(Symbol), // This should resolve to a Variable after resolver
    Variable(SymbolId),

    Int(u64),
    Bool(bool),
    Nothing,
    Str(Symbol),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        rhs: Box<Expr>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    FieldAccess {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Index {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
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
