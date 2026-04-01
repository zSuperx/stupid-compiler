use std::collections::HashMap;

use crate::types::*;

pub struct Resolver<'src> {
    types: HashMap<&'src str, ResolvedType>,
    scope_stack: Vec<HashMap<&'src str, Symbol<'src, ResolvedType>>>,
    return_type_stack: Vec<ResolvedType>,
    loop_stack: usize,
    src: &'src [u8],
}

pub type Terminates = bool;

impl<'src> Resolver<'src> {
    pub fn new(src: &'src [u8]) -> Self {
        use ResolvedType::*;
        let types = HashMap::from([
            ("u8", U8),
            ("u16", U16),
            ("u32", U32),
            ("u64", U64),
            ("i8", I8),
            ("i16", I16),
            ("i32", I32),
            ("i64", I64),
            ("bool", Bool),
            ("void", Void),
        ]);

        Self {
            types,
            scope_stack: vec![HashMap::new()],
            return_type_stack: vec![],
            loop_stack: 0,
            src,
        }
    }

    pub fn resolve_program(
        &mut self,
        objs: &[Object<'src, RawType<'src>>],
    ) -> Vec<Object<'src, ResolvedType>> {
        objs.iter().map(|o| self.resolve_object(o)).collect()
    }

    fn resolve_type(&mut self, ty: &RawType<'src>) -> Option<ResolvedType> {
        match ty {
            RawType::Unknown => Some(ResolvedType::Infer),
            RawType::Base(x) => self.types.get(x).cloned(),
            RawType::Pointer(raw_type) => self
                .resolve_type(raw_type)
                .map(|t| ResolvedType::Pointer((Box::new(t)))),
        }
    }

    pub fn resolve_object(
        &mut self,
        obj: &Object<'src, RawType<'src>>,
    ) -> Object<'src, ResolvedType> {
        let kind = match &obj.kind {
            OKind::Fn {
                name,
                args,
                returns,
                body,
            } => {
                let Some(resolved_return) = self.resolve_type(returns) else {
                    panic!("Unknown type {}\n\n{}", returns, obj.span);
                };
                let mut resolved_args = vec![];
                let mut scope = HashMap::new();
                for arg in args {
                    let resolved_arg = Symbol {
                        name: arg.name,
                        ty: self
                            .resolve_type(&arg.ty)
                            .expect(&format!("Unknown type {}", arg.ty)),
                    };
                    if scope.contains_key(arg.name) {
                        panic!("Argument {} already defined", arg.name);
                    } else {
                        scope.insert(arg.name, resolved_arg.clone());
                    }
                    resolved_args.push(resolved_arg);
                }

                let fn_sym = Symbol {
                    name,
                    ty: ResolvedType::Function {
                        args: resolved_args.iter().map(|arg| arg.ty.clone()).collect(),
                        returns: Box::new(resolved_return.clone()),
                    },
                };

                if self.scope_stack[0].insert(name, fn_sym).is_some() {
                    panic!(r#"Function "{name}" already defined"#);
                }

                self.return_type_stack.push(resolved_return.clone());
                self.scope_stack.push(scope);
                let (resolved_body, terminates) = self.resolve_stmt(body);
                if !terminates && resolved_return != ResolvedType::Void {
                    panic!(
                        "Function {} expects return type {}, but not all paths return a value",
                        name, resolved_return
                    );
                }
                self.scope_stack.pop();
                self.return_type_stack.pop();
                OKind::Fn {
                    name,
                    returns: resolved_return,
                    args: resolved_args,
                    body: resolved_body,
                }
            }
            OKind::Global(symbol) => todo!(),
            OKind::Struct { name, fields } => todo!(),
        };

        Object {
            kind,
            span: obj.span,
        }
    }

    fn resolve_symbol(&mut self, sym: &Symbol<'src, RawType<'src>>) -> Symbol<'src, ResolvedType> {
        Symbol {
            name: sym.name,
            ty: self.resolve_type(&sym.ty).expect(&format!(
                "Unknown type on variable {}: {}",
                sym.name, sym.ty
            )),
        }
    }

    pub fn resolve_expr(
        &mut self,
        expr: &Expr<'src, RawType<'src>>,
        hint: &ResolvedType,
    ) -> Expr<'src, ResolvedType> {
        match &expr.kind {
            EKind::Nothing => Expr {
                ty: ResolvedType::Void,
                kind: EKind::Nothing,
                span: expr.span,
            },
            EKind::Symbol(x) => {
                let sym = self
                    .scope_stack
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(x.name))
                    .expect(&format!("Variable used but not defined: {}", x.name))
                    .clone();
                Expr {
                    ty: sym.ty.clone(),
                    kind: EKind::Symbol(sym),
                    span: expr.span,
                }
            }
            EKind::Int(x) => {
                let mut ty = ResolvedType::I32;
                if *hint != ResolvedType::Infer {
                    if is_integral(&hint) {
                        ty = hint.clone()
                    } else {
                        panic!("Integer literal is not of type: {hint}");
                    }
                }
                Expr {
                    kind: EKind::Int(*x),
                    ty,
                    span: expr.span,
                }
            }
            EKind::Bool(x) => {
                if *hint != ResolvedType::Bool {
                    panic!(
                        "Mismatched types. Expected bool, found {hint}\n\n{}",
                        expr.span
                    );
                };
                Expr {
                    kind: EKind::Bool(*x),
                    ty: ResolvedType::Bool,
                    span: expr.span,
                }
            }
            EKind::Str(x) => {
                if let ResolvedType::Pointer(ty) = hint
                    && **ty != ResolvedType::U8
                {
                    panic!("Mismatched types: Lhs = {ty}, Rhs = *u8");
                }
                Expr {
                    kind: EKind::Str(x),
                    ty: ResolvedType::Pointer(Box::new(ResolvedType::U8)),
                    span: expr.span,
                }
            }
            EKind::Call { callee, args } => {
                let callee = self.resolve_expr(callee, hint);
                let ResolvedType::Function {
                    args: expected_args,
                    returns,
                } = &callee.ty
                else {
                    panic!(
                        "Expression does not resolve to a function\n\n{}",
                        callee.span
                    );
                };
                if args.len() != expected_args.len() {
                    panic!(
                        "Function takes {} arguments, but {} were given\n\n{}",
                        expected_args.len(),
                        args.len(),
                        callee.span,
                    );
                }
                let args = args
                    .iter()
                    .zip(expected_args.iter())
                    .map(|(a, e)| {
                        let r = self.resolve_expr(a, e);
                        if r.ty != *e {
                            panic!("Mismatched types. Expected {} but got {}", e, r.ty);
                        }
                        r
                    })
                    .collect();
                Expr {
                    ty: *returns.clone(),
                    kind: EKind::Call {
                        callee: Box::new(callee),
                        args,
                    },
                    span: expr.span,
                }
            }
            EKind::Unary { op, rhs } => match op {
                UnOp::Negate => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if !is_integral(&rhs.ty) {
                        panic!("Negation (-) can only be used on literal integral types");
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Unary {
                            op: UnOp::Negate,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::Not => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {}",
                            rhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Unary {
                            op: UnOp::Not,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::AddrOf => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if is_literal(&rhs) {
                        panic!("Cannot take the address of a literal type {}", rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Pointer(Box::new(rhs.ty.clone())),
                        kind: EKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                UnOp::Deref => {
                    let rhs = self.resolve_expr(rhs, hint);
                    let ResolvedType::Pointer(inner_type) = rhs.ty.clone() else {
                        panic!("Cannot dereference a literal type {}", rhs.ty);
                    };
                    Expr {
                        ty: *inner_type,
                        kind: EKind::Unary {
                            op: UnOp::Deref,
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
            },
            EKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    let mut valid_lvalue = match &lhs.kind {
                        EKind::Symbol(symbol) => {
                            !matches!(symbol.ty, ResolvedType::Function { .. })
                        }
                        EKind::Unary {
                            op: UnOp::Deref,
                            rhs,
                        } => true,
                        EKind::FieldAccess { lhs, rhs } => true,
                        x => false,
                    };
                    if !valid_lvalue {
                        panic!("Expression is not assignable\n\n{}", lhs.span);
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Bin {
                            op: BinOp::Assign,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Add | BinOp::Sub | BinOp::Sub | BinOp::Mul | BinOp::Div) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to perform arithmetic on non-integer type: {}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Gt | BinOp::Ge | BinOp::Lt | BinOp::Le) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to ordered comparison on non-integer type: {}",
                            lhs.ty
                        );
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::Eq | BinOp::Ne) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
                op @ (BinOp::LogOr | BinOp::LogAnd) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, &lhs.ty);
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {}, rhs = {}", lhs.ty, rhs.ty);
                    }
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {}\n\n{}",
                            lhs.ty, expr.span
                        );
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                        span: expr.span,
                    }
                }
            },
            EKind::FieldAccess { lhs, rhs } => todo!(),
            EKind::Index { lhs, rhs } => todo!(),
        }
    }

    fn resolve_stmt(
        &mut self,
        stmt: &Stmt<'src, RawType<'src>>,
    ) -> (Stmt<'src, ResolvedType>, Terminates) {
        let span = stmt.span;
        let (kind, terminates) = match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let Some(hint) = self.resolve_type(&lhs.ty) else {
                    panic!("Unknown type {}\n\n{}", lhs.ty, span);
                };
                let rhs = self.resolve_expr(rhs, &hint);
                if hint != ResolvedType::Infer && hint != rhs.ty {
                    panic!(
                        "Mismatched types lhs = {}, rhs = {}\n\n{}",
                        hint, rhs.ty, span
                    )
                }
                let sym = Symbol {
                    name: lhs.name,
                    ty: rhs.ty.clone(),
                };
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(lhs.name, sym.clone());
                (SKind::Let { lhs: sym, rhs }, false)
            }
            SKind::While { cond, body } => {
                let cond = self.resolve_expr(&cond, &ResolvedType::Bool);
                if cond.ty != ResolvedType::Bool {
                    panic!(
                        "Loop condition resolves to a non-boolean type: {}\n\n{}",
                        cond.ty, cond.span
                    );
                }
                self.loop_stack += 1;
                let (body, _) = self.resolve_stmt(body.as_ref());
                self.loop_stack -= 1;
                (
                    SKind::While {
                        cond,
                        body: Box::new(body),
                    },
                    false,
                )
            }
            SKind::Continue => {
                if self.loop_stack == 0 {
                    panic!(
                        "'continue' statements can only be called from within loops\n\n{}",
                        span
                    );
                }
                (SKind::Continue, true)
            }
            SKind::Break => {
                if self.loop_stack == 0 {
                    panic!(
                        "'break' statements can only be called from within loops\n\n{}",
                        span
                    );
                }
                (SKind::Break, true)
            }
            SKind::If { cond, then_, else_ } => {
                let cond = self.resolve_expr(&cond, &ResolvedType::Bool);
                if cond.ty != ResolvedType::Bool {
                    panic!(
                        "If condition resolves to a non-boolean type: {}\n\n{}",
                        cond.ty, cond.span
                    );
                }

                let (then_, then_terminates) = self.resolve_stmt(&then_);
                let (else_, else_terminates) = self.resolve_stmt(&else_);
                (
                    SKind::If {
                        cond,
                        then_: Box::new(then_),
                        else_: Box::new(else_),
                    },
                    then_terminates && else_terminates,
                )
            }
            SKind::Return(expr) => {
                let expected_return_type = self.return_type_stack.last().unwrap().clone();
                let expr = self.resolve_expr(&expr, &expected_return_type);
                if expr.ty != expected_return_type {
                    panic!(
                        "Function has return type {}, but {} was returned instead\n\n{}",
                        expected_return_type, expr.ty, expr.span
                    );
                }
                (SKind::Return(expr), true)
            }
            SKind::Block(stmts) => {
                self.scope_stack.push(HashMap::new());
                let mut terminates = false;
                let mut last_span = span;
                let stmts = stmts
                    .iter()
                    .map(|s| {
                        let (s, t) = self.resolve_stmt(s);
                        if terminates {
                            panic!("Unreachable code after this statement\n\n{}", last_span);
                        }
                        last_span = s.span;
                        terminates = t;
                        s
                    })
                    .collect();
                self.scope_stack.pop();
                (SKind::Block(stmts), terminates)
            }
            SKind::Expr(expr) => (
                SKind::Expr(self.resolve_expr(&expr, &ResolvedType::Void)),
                false,
            ),
        };
        (Stmt { kind, span }, terminates)
    }
}

fn is_integral(ty: &ResolvedType) -> bool {
    use ResolvedType::*;
    matches!(ty, U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64)
}

fn is_literal<T>(expr: &Expr<T>) -> bool {
    matches!(expr.kind, EKind::Int(_) | EKind::Str(_) | EKind::Bool(_))
}
