use std::collections::HashMap;

use crate::types::*;

pub struct Resolver<'src> {
    types: HashMap<&'src str, ResolvedType>,
    scope_stack: Vec<HashMap<&'src str, Symbol<'src, ResolvedType>>>,
    return_type_stack: Vec<ResolvedType>,
    loop_stack: usize,
}

pub type Terminates = bool;

impl<'src> Resolver<'src> {
    pub fn new() -> Self {
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
            RawType::Unknown => None,
            RawType::Base(x) => Some(
                self.types
                    .get(x)
                    .expect(&format!("Unrecognized type: {ty:?}"))
                    .clone(),
            ),
            RawType::Pointer(raw_type) => Some(ResolvedType::Pointer(Box::new(
                self.resolve_type(raw_type).unwrap(),
            ))),
        }
    }

    pub fn resolve_object(
        &mut self,
        obj: &Object<'src, RawType<'src>>,
    ) -> Object<'src, ResolvedType> {
        match obj {
            Object::Fn {
                name,
                args,
                returns,
                body,
            } => {
                let resolved_return = self
                    .resolve_type(returns)
                    .expect(&format!("Unknown type {:?}", returns));
                let mut resolved_args = vec![];
                let mut scope = HashMap::new();
                for arg in args {
                    let resolved_arg = Symbol {
                        name: arg.name,
                        ty: self
                            .resolve_type(&arg.ty)
                            .expect(&format!("Unknown type {:?}", arg.ty)),
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
                    panic!("Function expects return type {:?}, but nothing was returned", resolved_return);
                }
                self.scope_stack.pop();
                self.return_type_stack.pop();
                Object::Fn {
                    name,
                    returns: resolved_return,
                    args: resolved_args,
                    body: resolved_body,
                }
            }
            Object::Global(symbol) => todo!(),
            Object::Struct { name, fields } => todo!(),
        }
    }

    fn resolve_symbol(&mut self, sym: &Symbol<'src, RawType<'src>>) -> Symbol<'src, ResolvedType> {
        Symbol {
            name: sym.name,
            ty: self.resolve_type(&sym.ty).expect(&format!(
                "Unknown type on variable {}: {:?}",
                sym.name, sym.ty
            )),
        }
    }

    pub fn resolve_expr(
        &mut self,
        expr: &Expr<'src, RawType<'src>>,
        hint: Option<&ResolvedType>,
    ) -> Expr<'src, ResolvedType> {
        match &expr.kind {
            EKind::Nothing => Expr {
                ty: ResolvedType::Void,
                kind: EKind::Nothing,
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
                }
            }
            EKind::Int(x) => {
                let mut ty = ResolvedType::I32;
                if let Some(hint_ty) = hint {
                    if is_integral(hint_ty) {
                        ty = hint_ty.clone()
                    } else {
                        panic!("Integer literal is not of type: {hint_ty:?}");
                    }
                }
                Expr {
                    kind: EKind::Int(*x),
                    ty,
                }
            }
            EKind::Bool(x) => {
                if hint.is_some_and(|t| *t != ResolvedType::Bool) {
                    panic!("Mismatched types: Lhs = {hint:?}, Rhs = Bool");
                };
                Expr {
                    kind: EKind::Bool(*x),
                    ty: ResolvedType::Bool,
                }
            }
            EKind::Str(x) => {
                if let Some(ResolvedType::Pointer(ty)) = hint
                    && **ty != ResolvedType::U8
                {
                    panic!("Mismatched types: Lhs = {hint:?}, Rhs = *u8");
                }
                Expr {
                    kind: EKind::Str(x),
                    ty: ResolvedType::Pointer(Box::new(ResolvedType::U8)),
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
                        "The expression: \"{:?}\" does not resolve to a function",
                        callee
                    );
                };
                if args.len() != expected_args.len() {
                    panic!(
                        "Function \"{:?}\" takes {} arguments, but {} were given",
                        callee,
                        expected_args.len(),
                        args.len()
                    );
                }
                let args = args
                    .iter()
                    .zip(expected_args.iter())
                    .map(|(a, e)| {
                        let r = self.resolve_expr(a, Some(e));
                        if r.ty != *e {
                            panic!("Mismatched types. Expected {:?} but got {:?}", e, r.ty);
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
                    }
                }
                UnOp::Not => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {:?}",
                            rhs.ty
                        );
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Unary {
                            op: UnOp::Not,
                            rhs: Box::new(rhs),
                        },
                    }
                }
                UnOp::AddrOf => {
                    let rhs = self.resolve_expr(rhs, hint);
                    if is_literal(&rhs) {
                        panic!("Cannot take the address of a literal type {:?}", rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Pointer(Box::new(rhs.ty.clone())),
                        kind: EKind::Unary {
                            op: UnOp::AddrOf,
                            rhs: Box::new(rhs),
                        },
                    }
                }
                UnOp::Deref => {
                    let rhs = self.resolve_expr(rhs, hint);
                    let ResolvedType::Pointer(inner_type) = rhs.ty.clone() else {
                        panic!("Cannot dereference a literal type {:?}", rhs.ty);
                    };
                    Expr {
                        ty: *inner_type,
                        kind: EKind::Unary {
                            op: UnOp::Deref,
                            rhs: Box::new(rhs),
                        },
                    }
                }
            },
            EKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
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
                        panic!("The expression \"{:?}\" is not assignable", lhs.kind);
                    }
                    Expr {
                        ty: rhs.ty.clone(),
                        kind: EKind::Bin {
                            op: BinOp::Assign,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::Add | BinOp::Sub | BinOp::Sub | BinOp::Mul | BinOp::Div) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to perform arithmetic on non-integer type: {:?}",
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
                    }
                }
                op @ (BinOp::Gt | BinOp::Ge | BinOp::Lt | BinOp::Le) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if !is_integral(&rhs.ty) {
                        panic!(
                            "Attempted to ordered comparison on non-integer type: {:?}",
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
                    }
                }
                op @ (BinOp::Eq | BinOp::Ne) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    use ResolvedType::*;
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    Expr {
                        ty: ResolvedType::Bool,
                        kind: EKind::Bin {
                            op: *op,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        },
                    }
                }
                op @ (BinOp::LogOr | BinOp::LogAnd) => {
                    let lhs = self.resolve_expr(lhs, hint);
                    let rhs = self.resolve_expr(rhs, Some(&lhs.ty));
                    if lhs.ty != rhs.ty {
                        panic!("Mismatched types lhs = {:?}, rhs = {:?}", lhs.ty, rhs.ty);
                    }
                    if rhs.ty != ResolvedType::Bool {
                        panic!(
                            "Attempted to perform logical comparison on non-boolean type: {:?}",
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
        match stmt {
            Stmt::Let { lhs, rhs } => {
                let hint_ty = self.resolve_type(&lhs.ty);
                let rhs = self.resolve_expr(rhs, hint_ty.as_ref());
                if let Some(ref ty) = hint_ty
                    && ty != &rhs.ty
                {
                    panic!("Mismatched types lhs = {ty:?}, rhs = {:?}", rhs.ty)
                }
                let sym = Symbol {
                    name: lhs.name,
                    ty: rhs.ty.clone(),
                };
                self.scope_stack
                    .last_mut()
                    .unwrap()
                    .insert(lhs.name, sym.clone());
                (Stmt::Let { lhs: sym, rhs }, false)
            }
            Stmt::While { cond, body } => {
                let cond = self.resolve_expr(cond, None);
                if cond.ty != ResolvedType::Bool {
                    panic!("Loop conditions must resolve to a Bool");
                }
                self.loop_stack += 1;
                let (body, _) = self.resolve_stmt(body);
                self.loop_stack -= 1;
                (Stmt::While {
                    cond,
                    body: Box::new(body),
                }, false)
            }
            Stmt::Continue => {
                if self.loop_stack == 0 {
                    panic!("'continue' statements can only be called from within loops");
                }
                (Stmt::Continue, true)
            }
            Stmt::Break => {
                if self.loop_stack == 0 {
                    panic!("'break' statements can only be called from within loops");
                }
                (Stmt::Break, true)
            }
            Stmt::If { cond, then_, else_ } => {
                let cond = self.resolve_expr(cond, None);
                if cond.ty != ResolvedType::Bool {
                    panic!("If conditions must resolve to a Bool");
                }

                let (then_, then_terminates) = self.resolve_stmt(then_);
                let (else_, else_terminates) = self.resolve_stmt(else_);
                (
                    Stmt::If {
                        cond,
                        then_: Box::new(then_),
                        else_: Box::new(else_),
                    },
                    then_terminates && else_terminates,
                )
            }
            Stmt::Return(expr) => {
                let expected_return_type = self.return_type_stack.last().unwrap().clone();
                let expr = self.resolve_expr(
                    expr,
                    (expected_return_type != ResolvedType::Void).then_some(&expected_return_type),
                );
                if expr.ty != expected_return_type {
                    panic!(
                        "Function has return type {:?}, but {:?} is returned instead",
                        expected_return_type, expr.ty
                    );
                }
                (Stmt::Return(expr), true)
            }
            Stmt::Block(stmts) => {
                self.scope_stack.push(HashMap::new());
                let mut terminates = false;
                let stmts = stmts
                    .iter()
                    .map(|s| {
                        let (s, t) = self.resolve_stmt(s);
                        if terminates {
                            panic!("Unreachable code after {s:?}");
                        }
                        terminates = t;
                        s
                    })
                    .collect();
                self.scope_stack.pop();
                (Stmt::Block(stmts), terminates)
            }
            Stmt::Expr(expr) => (Stmt::Expr(self.resolve_expr(expr, None)), false),
        }
    }
}

fn is_integral(ty: &ResolvedType) -> bool {
    use ResolvedType::*;
    matches!(ty, U8 | U16 | U32 | U64 | I8 | I16 | I32 | I64)
}

fn is_literal<T>(expr: &Expr<T>) -> bool {
    matches!(expr.kind, EKind::Int(_) | EKind::Str(_) | EKind::Bool(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser, types::*};

    #[test]
    fn test_resolve() {
        let src = br#"
fn bob(a: u8, b: **u8) {}

fn main(argc: u8, argv: **u8) {
    main = bob;
}

"#;
        println!("Source code:\n{}\n", str::from_utf8(src).unwrap());
        let tokens: Vec<TKind> = Lexer::new(src).map(|t| t.kind).collect();
        let parsed = Parser::new(&tokens).parse_program();
        let resolved = Resolver::new().resolve_program(&parsed);
        dbg!(&resolved);
    }
}
