use crate::types::*;

pub struct Parser<'src> {
    src: &'src [TKind<'src>],
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src [TKind<'src>]) -> Self {
        Self { src }
    }

    fn is_next(&self, kind: TKind<'src>) -> bool {
        self.peek() == kind
    }

    fn expect(&mut self, kind: TKind<'src>) {
        let next = self.consume();
        if next != kind {
            panic!("Expected {kind}, found {next}");
        }
    }

    fn expect_ident(&mut self) -> &'src str {
        let next = self.consume();
        match next {
            TKind::Ident(s) => s,
            _ => panic!("Expected identifier, found {next}"),
        }
    }

    fn expect_strlit(&mut self) -> &'src [u8] {
        let next = self.consume();
        match next {
            TKind::Str(s) => s,
            _ => panic!("Expected string literal, found {next}"),
        }
    }

    fn expect_intlit(&mut self) -> u64 {
        let next = self.consume();
        match next {
            TKind::Int(s) => s,
            _ => panic!("Expected int literal, found {next}"),
        }
    }

    fn consume(&mut self) -> TKind<'src> {
        let first = self.src.first().unwrap_or(&TKind::Eof);
        if !self.src.is_empty() {
            self.src = &self.src[1..];
        }
        *first
    }

    fn peek(&self) -> TKind<'src> {
        *self.src.first().unwrap_or(&TKind::Eof)
    }

    pub fn parse_program(&mut self) -> Vec<OKind<'src, RawType<'src>>> {
        let mut objs = vec![];
        while !self.is_next(TKind::Eof) {
            match self.peek() {
                TKind::Fn => objs.push(self.parse_fn()),
                TKind::Struct => objs.push(self.parse_struct()),
                TKind::Global => objs.push(self.parse_global()),
                x => panic!("Expected function definition, found {x}"),
            }
        }
        objs
    }

    fn parse_struct(&mut self) -> OKind<'src, RawType<'src>> {
        todo!()
    }

    fn parse_global(&mut self) -> OKind<'src, RawType<'src>> {
        self.expect(TKind::Global);
        let name = self.expect_ident();
        self.expect(TKind::Colon);
        let ty = self.parse_type();
        self.expect(TKind::Eq);
        let rhs = self.parse_expr();
        OKind::Global(Symbol { name, ty })
    }

    fn parse_fn(&mut self) -> OKind<'src, RawType<'src>> {
        self.expect(TKind::Fn);
        let name = self.expect_ident();
        self.expect(TKind::LParen);

        let mut args = vec![];
        if !self.is_next(TKind::RParen) {
            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        loop {
            let cur = self.peek();
            if matches!(cur, TKind::RParen) {
                self.consume();
                break;
            }

            self.expect(TKind::Comma);

            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type();
            let var = Symbol { name, ty };
            args.push(var);
        }

        let returns = if self.is_next(TKind::Arrow) {
            self.expect(TKind::Arrow);
            self.parse_type()
        } else {
            RawType::Base("void")
        };

        let body = self.parse_block();

        OKind::Fn {
            name,
            body,
            args,
            returns,
        }
    }

    fn parse_type(&mut self) -> RawType<'src> {
        let mut ty = match self.peek() {
            TKind::Star => {
                self.consume();
                return RawType::Pointer(Box::new(self.parse_type()));
            }
            TKind::Ident(name) => RawType::Base(name),
            x => panic!("Expected type, found {x}"),
        };
        self.consume();
        ty
    }

    fn parse_prefix(&mut self) -> EKind<'src, RawType<'src>> {
        match self.consume() {
            TKind::Ident(s) => EKind::Symbol(Symbol {
                name: s,
                ty: RawType::Unknown,
            }),
            TKind::Bool(x) => EKind::Bool(x),
            TKind::Int(x) => EKind::Int(x),
            TKind::Str(s) => EKind::Str(s),
            op @ (TKind::Minus | TKind::Bang | TKind::And | TKind::Star) => EKind::Unary {
                op: match op {
                    TKind::Minus => UnOp::Negate,
                    TKind::Bang => UnOp::Not,
                    TKind::And => UnOp::AddrOf,
                    TKind::Star => UnOp::Deref,
                    _ => unreachable!(),
                },
                rhs: Box::new(expr(
                    self.parse_expr_kind(prefix_power(op).expect("Invalid operator")),
                )),
            },
            TKind::LParen => {
                let inner = self.parse_expr_kind(0.0);
                self.expect(TKind::RParen);
                inner
            }
            x => panic!("Expected start of expression, found \"{x}\""),
        }
    }

    fn parse_infix(
        &mut self,
        lhs: EKind<'src, RawType<'src>>,
        op: TKind<'src>,
        op_power: f32,
    ) -> EKind<'src, RawType<'src>> {
        match op {
            TKind::LParen => {
                let mut args = vec![];
                while !self.is_next(TKind::RParen) {
                    if !args.is_empty() {
                        self.expect(TKind::Comma);
                    }
                    args.push(expr(self.parse_expr_kind(0.0)));
                }
                self.expect(TKind::RParen);
                EKind::Call {
                    callee: Box::new(expr(lhs)),
                    args,
                }
            }
            op @ (TKind::Eq
            | TKind::Plus
            | TKind::Minus
            | TKind::Star
            | TKind::Slash
            | TKind::AndAnd
            | TKind::OrOr
            | TKind::EqEq
            | TKind::BangEq
            | TKind::Lt
            | TKind::LtEq
            | TKind::Gt
            | TKind::GtEq) => {
                let rhs = self.parse_expr_kind(op_power);
                let binop = match op {
                    TKind::Eq => BinOp::Assign,
                    TKind::Plus => BinOp::Add,
                    TKind::Minus => BinOp::Sub,
                    TKind::Star => BinOp::Mul,
                    TKind::Slash => BinOp::Div,
                    TKind::AndAnd => BinOp::LogAnd,
                    TKind::OrOr => BinOp::LogOr,
                    TKind::EqEq => BinOp::Eq,
                    TKind::BangEq => BinOp::Ne,
                    TKind::Lt => BinOp::Lt,
                    TKind::LtEq => BinOp::Le,
                    TKind::Gt => BinOp::Gt,
                    TKind::GtEq => BinOp::Ge,
                    _ => unreachable!(),
                };
                EKind::Bin {
                    op: binop,
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(rhs)),
                }
            }
            TKind::LBrack => {
                let index = self.parse_expr_kind(op_power);
                self.expect(TKind::RBrack);
                EKind::Index {
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(index)),
                }
            }
            TKind::Dot => {
                let field = self.parse_expr_kind(op_power);
                if !matches!(field, EKind::Symbol(_)) {
                    panic!("Fields can only be accessed with an identifier, not a {field:?}");
                }
                EKind::FieldAccess {
                    lhs: Box::new(expr(lhs)),
                    rhs: Box::new(expr(field)),
                }
            }
            x => panic!("Expected infix operator, found {x}"),
        }
    }

    // Pratt parsing!
    fn parse_expr_kind(&mut self, min_power: f32) -> EKind<'src, RawType<'src>> {
        let mut lhs = self.parse_prefix();

        loop {
            let op = self.peek();

            let Some(op_power) = infix_power(op) else {
                break;
            };

            if op_power.0 < min_power {
                break;
            }

            self.consume();

            lhs = self.parse_infix(lhs, op, op_power.1);
        }

        lhs
    }

    pub fn parse_expr(&mut self) -> Expr<'src, RawType<'src>> {
        Expr {
            kind: self.parse_expr_kind(0.0),
            ty: RawType::Unknown,
            
        }
    }

    fn parse_block(&mut self) -> SKind<'src, RawType<'src>> {
        self.expect(TKind::LCurly);
        let mut stmts = vec![];
        while !self.is_next(TKind::RCurly) {
            let stmt = self.parse_stmt();
            if matches!(stmt, SKind::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.expect(TKind::RCurly);
        SKind::Block(stmts)
    }

    fn parse_stmt(&mut self) -> SKind<'src, RawType<'src>> {
        match self.peek() {
            TKind::Let => {
                let mut ty = RawType::Unknown;
                self.expect(TKind::Let);
                let name = self.expect_ident();
                if self.is_next(TKind::Colon) {
                    self.expect(TKind::Colon);
                    ty = self.parse_type();
                }
                let lhs = Symbol { name, ty };
                self.expect(TKind::Eq);

                let rhs = self.parse_expr();
                self.expect(TKind::Semi);
                SKind::Let { lhs, rhs }
            }
            TKind::While => {
                self.expect(TKind::While);
                let cond = self.parse_expr();
                let body = self.parse_block();
                SKind::While {
                    cond,
                    body: Box::new(body),
                }
            }
            TKind::Continue => {
                self.expect(TKind::Continue);
                self.expect(TKind::Semi);
                SKind::Continue
            }
            TKind::Break => {
                self.expect(TKind::Break);
                self.expect(TKind::Semi);
                SKind::Break
            }
            TKind::If => {
                self.expect(TKind::If);
                let cond = self.parse_expr();
                let then_ = self.parse_block();

                let else_ = if self.is_next(TKind::Else) {
                    self.expect(TKind::Else);
                    if self.is_next(TKind::If) {
                        // If is a special case since it's allowed after an else
                        self.parse_stmt()
                    } else {
                        // Otherwise, else must be followed by a block
                        self.parse_block()
                    }
                } else {
                    SKind::Block(vec![])
                };
                SKind::If {
                    cond,
                    then_: Box::new(then_),
                    else_: Box::new(else_),
                }
            }
            TKind::Return => {
                self.expect(TKind::Return);
                let retval = if !self.is_next(TKind::Semi) {
                    self.parse_expr()
                } else {
                    expr(EKind::Nothing)
                };
                self.expect(TKind::Semi);

                SKind::Return(retval)
            }
            x => {
                let expr = self.parse_expr();
                self.expect(TKind::Semi);
                SKind::Expr(expr)
            }
        }
    }
}

fn prefix_power(kind: TKind) -> Option<f32> {
    let power = match kind {
        TKind::Bang | TKind::Minus | TKind::Star | TKind::And => 9.0,
        _ => return None, // Not an prefix operator
    };
    Some(power)
}

fn infix_power(kind: TKind) -> Option<(f32, f32)> {
    let power = match kind {
        // Assignment: Right-Associative
        // We use a lower Left power so it "gives up" easily,
        // but a higher Right power to "grab" everything to the right.
        TKind::Eq => (2.1, 2.0),

        TKind::AndAnd | TKind::OrOr => (4.1, 4.0),

        TKind::EqEq | TKind::BangEq => (5.0, 5.1),
        TKind::Lt | TKind::Gt => (6.0, 6.1),
        TKind::LtEq | TKind::GtEq => (6.0, 6.1),

        TKind::Plus | TKind::Minus => (7.0, 7.1),

        TKind::Star | TKind::Slash => (8.0, 8.1),

        // Postfix: Highest priority
        // (Call, Indexing, Member Access)
        TKind::LParen | TKind::LBrack | TKind::Dot => (10.0, 10.1),

        _ => return None, // Not an infix operator
    };
    Some(power)
}
