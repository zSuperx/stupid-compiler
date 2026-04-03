use std::collections::HashMap;

use crate::types::*;

pub struct Parser<'src> {
    tokens: &'src [Token],
    last_span: Span,
}

impl<'src> Parser<'src> {
    pub fn new(tokens: &'src [Token]) -> Self {
        Self {
            tokens,
            last_span: Span {
                ..Default::default()
            },
        }
    }

    fn mark(&self) -> Span {
        self.peek().span
    }

    fn commit_expr(&mut self, kind: EKind, span: Span) -> Expr {
        Expr::new(
            kind,
            TyKind::Infer,
            span.merge(self.last_span),
        )
    }

    fn commit_stmt(&mut self, kind: SKind, span: Span) -> Stmt {
        Stmt::new(kind, span.merge(self.last_span))
    }

    fn commit_type(&mut self, kind: TyKind, span: Span) -> Type {
        Type {
            kind,
            span: span.merge(self.last_span),
        }
    }

    /// Peeks the next token and checks its kind
    fn is_next(&self, kind: TKind) -> bool {
        self.peek().kind == kind
    }

    /// Peeks the next token. This does not consume.
    fn peek(&self) -> Token {
        self.tokens.first().copied().unwrap_or_default()
    }

    fn consume(&mut self) -> Token {
        let first = self.tokens.first().copied().unwrap_or_default();
        if !self.tokens.is_empty() {
            self.tokens = &self.tokens[1..];
        }
        self.last_span = first.span;
        first
    }

    fn expect(&mut self, kind: TKind) {
        let next = self.consume();
        if next.kind != kind {
            panic!("Expected {kind:?}, found {:?}", next.kind);
        }
    }

    fn expect_ident(&mut self) -> Symbol {
        let next = self.consume();
        match next.kind {
            TKind::Ident(s) => s,
            _ => panic!("Expected identifier, found {:?}", next.kind),
        }
    }

    fn _expect_strlit(&mut self) -> Symbol {
        let next = self.consume();
        match next.kind {
            TKind::Str(s) => s,
            _ => panic!("Expected string literal, found {:?}", next.kind),
        }
    }

    fn _expect_intlit(&mut self) -> u64 {
        let next = self.consume();
        match next.kind {
            TKind::Int(s) => s,
            _ => panic!("Expected int literal, found {:?}", next.kind),
        }
    }

    pub fn parse_program(&mut self, ctx: &mut Context) -> Vec<Object> {
        let mut objs = vec![];
        while !self.is_next(TKind::Eof) {
            let tok = self.peek();
            match tok.kind {
                TKind::Fn => objs.push(self.parse_fn(ctx)),
                TKind::Struct => objs.push(self.parse_struct(ctx)),
                TKind::Global => objs.push(self.parse_global(ctx)),
                x => panic!("Expected function definition, found {x:?}"),
            }
        }
        objs
    }

    fn parse_struct(&mut self, ctx: &mut Context) -> Object {
        todo!()
    }

    fn parse_global(&mut self, ctx: &mut Context) -> Object {
        let span_start = self.mark();
        self.expect(TKind::Global);
        let name = self.expect_ident();
        self.expect(TKind::Colon);
        let ty = self.parse_type(ctx);
        self.expect(TKind::Eq);
        let rhs = self.parse_expr(ctx);
        let sym_id = ctx.declare_global(name, ty);
        Object {
            kind: OKind::Global { lhs: sym_id, rhs },
            span: span_start.merge(self.last_span),
        }
    }

    fn parse_fn(&mut self, ctx: &mut Context) -> Object {
        let span_start = self.mark();
        self.expect(TKind::Fn);
        let name = self.expect_ident();
        self.expect(TKind::LParen);

        let mut args = vec![];
        if !self.is_next(TKind::RParen) {
            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type(ctx);
            let arg_sym_id = ctx.declare_local(name, ty);
            args.push(arg_sym_id);
        }

        loop {
            let cur = self.peek();
            if matches!(cur.kind, TKind::RParen) {
                self.consume();
                break;
            }

            self.expect(TKind::Comma);

            let name = self.expect_ident();
            self.expect(TKind::Colon);
            let ty = self.parse_type(ctx);
            let arg_sym_id = ctx.declare_local(name, ty);
            args.push(arg_sym_id);
        }

        let returns = if self.is_next(TKind::Arrow) {
            self.expect(TKind::Arrow);
            self.parse_type(ctx)
        } else {
            Type {
                kind: TyKind::Void,
                span: self.last_span,
            }
        };

        let body = self.parse_block(ctx);

        let ty = Type {
            kind: TyKind::Function {
                args: args
                    .iter()
                    .map(|a| ctx.symbols[*a].ty)
                    .collect(),
                returns: returns,
            },
            span: returns.span,
        };
        let fn_name = ctx.declare_global(name, ty);

        Object {
            kind: OKind::Fn {
                name: fn_name,
                body,
                args,
                returns,
            },
            span: span_start.merge(self.last_span),
        }
    }

    fn parse_type(&mut self, ctx: &mut Context) -> Type {
        let span = self.mark();
        let tok = self.consume();
        let kind = match tok.kind {
            TKind::Star => TyKind::Pointer(self.parse_type(ctx)),
            TKind::Ident(name) => TyKind::Unresolved(name),
            x => panic!("Expected type, found {x:?}"),
        };
        self.commit_type(kind, span)
    }

    fn parse_prefix(&mut self, ctx: &mut Context) -> Expr {
        let span_start = self.mark();
        let tok = self.consume();
        let kind = match tok.kind {
            TKind::Ident(s) => EKind::Unresolved(s),
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
                rhs: Box::new(self._parse_expr(ctx, prefix_power(op).expect("Invalid operator"))),
            },
            TKind::LParen => {
                let inner = self.parse_expr(ctx);
                self.expect(TKind::RParen);
                inner.kind
            }
            x => panic!("Expected start of expression, found \"{x:?}\""),
        };
        self.commit_expr(kind, span_start.merge(self.last_span))
    }

    fn parse_infix(&mut self, ctx: &mut Context, lhs: Expr, op: TKind, op_power: f32) -> Expr {
        let span_start = lhs.span;
        let kind = match op {
            TKind::LParen => {
                let mut args = vec![];
                while !self.is_next(TKind::RParen) {
                    if !args.is_empty() {
                        self.expect(TKind::Comma);
                    }
                    args.push(self.parse_expr(ctx));
                }
                self.expect(TKind::RParen);
                EKind::Call {
                    callee: Box::new(lhs),
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
                let rhs = self._parse_expr(ctx, op_power);
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
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            }
            TKind::LBrack => {
                let index = self._parse_expr(ctx, op_power);
                self.expect(TKind::RBrack);
                EKind::Index {
                    lhs: Box::new(lhs),
                    rhs: Box::new(index),
                }
            }
            TKind::Dot => {
                let field = self._parse_expr(ctx, op_power);
                if !matches!(field.kind, EKind::Unresolved(_)) {
                    panic!(
                        "Fields can only be accessed with an identifier, not a {}",
                        field.kind,
                    );
                }
                EKind::FieldAccess {
                    lhs: Box::new(lhs),
                    rhs: Box::new(field),
                }
            }
            x => panic!("Expected infix operator, found {x:?}"),
        };
        self.commit_expr(kind, span_start)
    }

    // Pratt parsing!
    fn _parse_expr(&mut self, ctx: &mut Context, min_power: f32) -> Expr {
        let mut lhs = self.parse_prefix(ctx);

        loop {
            let op = self.peek().kind;

            let Some(op_power) = infix_power(op) else {
                break;
            };

            if op_power.0 < min_power {
                break;
            }

            self.consume();

            lhs = self.parse_infix(ctx, lhs, op, op_power.1);
        }

        lhs
    }

    pub fn parse_expr(&mut self, ctx: &mut Context) -> Expr {
        self._parse_expr(ctx, 0.0)
    }

    fn parse_block(&mut self, ctx: &mut Context) -> Stmt {
        let span_start = self.mark();
        self.expect(TKind::LCurly);
        let mut stmts = vec![];
        while !self.is_next(TKind::RCurly) {
            let stmt = self.parse_stmt(ctx);
            if matches!(stmt.kind, SKind::Block(ref inner) if inner.is_empty()) {
                // Skip nested empty {} blocks, they're useless.
                continue;
            }
            stmts.push(stmt);
        }
        self.expect(TKind::RCurly);
        self.commit_stmt(SKind::Block(stmts), span_start)
    }

    fn parse_stmt(&mut self, ctx: &mut Context) -> Stmt {
        let span_start = self.mark();
        let tok = self.peek();
        let kind = match tok.kind {
            TKind::Let => {
                self.expect(TKind::Let);
                let name = self.expect_ident();
                let mut ty = Type { kind: TyKind::Infer, span: self.last_span };
                if self.is_next(TKind::Colon) {
                    self.expect(TKind::Colon);
                    ty = self.parse_type(ctx);
                }
                let lhs = ctx.declare_local(name, ty);
                self.expect(TKind::Eq);
                let rhs = self.parse_expr(ctx);
                self.expect(TKind::Semi);
                SKind::Let { lhs, rhs }
            }
            TKind::While => {
                self.expect(TKind::While);
                let cond = self.parse_expr(ctx);
                let body = self.parse_block(ctx);
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
                let cond = self.parse_expr(ctx);
                let then_ = self.parse_block(ctx);

                let else_ = if self.is_next(TKind::Else) {
                    self.expect(TKind::Else);
                    if self.is_next(TKind::If) {
                        // If is a special case since it's allowed after an else
                        self.parse_stmt(ctx)
                    } else {
                        // Otherwise, else must be followed by a block
                        self.parse_block(ctx)
                    }
                } else {
                    self.commit_stmt(SKind::Block(vec![]), self.last_span)
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
                    self.parse_expr(ctx)
                } else {
                    self.commit_expr(EKind::Nothing, span_start)
                };
                self.expect(TKind::Semi);

                SKind::Return(retval)
            }
            TKind::LCurly => {
                return self.parse_block(ctx);
            }
            _ => {
                let expr = self.parse_expr(ctx);
                self.expect(TKind::Semi);
                SKind::Expr(expr)
            }
        };

        self.commit_stmt(kind, span_start)
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
