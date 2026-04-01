use std::collections::HashMap;

use crate::types::*;

pub struct Emitter<'src> {
    vr_count: VReg,
    if_label: usize,
    loop_label: usize,
    program: Vec<String>,
    symbols: HashMap<&'src str, VReg>,
}

type VReg = usize;

impl<'src> Emitter<'src> {
    pub fn new() -> Self {
        Self {
            vr_count: 0,
            if_label: 0,
            loop_label: 0,
            program: Vec::new(),
            symbols: HashMap::new(),
        }
    }

    fn emit_raw(&mut self, next_program: &str) {
        if !next_program.is_empty() {
            self.program.push(format!("\t{next_program}\n"));
        }
    }

    fn emit_label(&mut self, label: &str) {
        if !label.is_empty() {
            self.program.push(format!("{label}:\n"));
        }
    }

    fn next_label(&mut self, stmt: &Stmt<'src, ResolvedType>) -> String {
        match &stmt.kind {
            SKind::While { .. } => {
                self.loop_label += 1;
                format!("L{}", self.loop_label)
            }
            SKind::If { .. } => {
                self.if_label += 1;
                format!("I{}", self.if_label)
            }
            _ => unreachable!(),
        }
    }

    fn next_vr(&mut self) -> VReg {
        self.vr_count += 1;
        self.vr_count
    }

    pub fn emit_program(mut self, objs: &[Object<'src, ResolvedType>]) -> Vec<String> {
        for obj in objs {
            self.emit_object(obj);
        }
        self.program
    }

    fn emit_object(&mut self, obj: &Object<'src, ResolvedType>) {
        match &obj.kind {
            OKind::Fn {
                name,
                returns,
                args,
                body,
            } => {
                let label = format!("Fn_{name}");
                self.emit_label(&label);
                self.symbols.clear();
                self.vr_count = 0;
                for (i, arg) in args.iter().enumerate() {
                    let target = self.next_vr();
                    self.emit_raw(&format!("%{target} = arg({i})"));
                    self.symbols.insert(arg.name, target);
                }
                self.emit_stmt(body);
                if *returns == ResolvedType::Void
                    && !self.program.last().unwrap().contains("\tret\n")
                {
                    self.emit_raw("ret");
                }
            }
            OKind::Global { .. } => todo!(),
            OKind::Struct { .. } => todo!(),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt<'src, ResolvedType>) {
        match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let target = self.next_vr();
                if self.symbols.insert(lhs.name, target).is_some() {
                    panic!("Duplicate symbol {:?}", lhs);
                }
                let rhs = self.emit_expr(rhs);
                let program = format!("%{target} = %{}", rhs);
                self.emit_raw(&program);
            }
            SKind::While { cond, body } => {
                let label = self.next_label(stmt);
                let start_label = label.clone() + "_loop";
                let end_label = label + "_end";
                let cond = self.emit_expr(cond);
                let target = self.next_vr();
                self.emit_raw(&format!("%{target} = int(0)"));
                self.emit_raw(&format!("bne %{target}, %{cond}, {end_label}"));
                self.emit_label(&start_label);
                self.emit_stmt(body);
                self.emit_raw(&format!("br {start_label}"));
                self.emit_label(&end_label);
            }
            // Find the current loop label
            SKind::Continue => {
                // Jump to start label
                self.emit_raw(&format!("br L{}_loop", self.if_label));
            }
            SKind::Break => {
                // Jump to end label
                self.emit_raw(&format!("br L{}_endloop", self.if_label));
            }
            SKind::If { cond, then_, else_ } => {
                let cond = self.emit_expr(cond);
                let label = self.next_label(stmt);
                let else_label = label.clone() + "_else";
                let end_label = label + "_end";
                let target = self.next_vr();
                self.emit_raw(&format!("%{target} = int(0)"));
                self.emit_raw(&format!("beq %{target}, %{cond}, {else_label}"));
                self.emit_stmt(then_);
                if let SKind::Block(v) = &else_.as_ref().kind
                    && !v.is_empty()
                {
                    self.emit_raw(&format!("br {end_label}"));
                    self.emit_label(&else_label);
                    self.emit_stmt(else_);
                }
                self.emit_label(&end_label);
            }
            SKind::Return(expr) => {
                if expr.ty != ResolvedType::Void {
                    let ret = self.emit_expr(expr);
                    self.emit_raw(&format!("ret %{ret}"));
                } else {
                    self.emit_raw(&format!("ret"));
                }
            }
            SKind::Block(stmts) => {
                for stmt in stmts {
                    self.emit_stmt(stmt);
                }
            }
            SKind::Expr(expr) => _ = self.emit_expr(expr),
        }
    }

    fn emit_expr(&mut self, Expr { kind, .. }: &Expr<'src, ResolvedType>) -> VReg {
        let target;
        let template = match kind {
            EKind::Symbol(symbol) => {
                target = *self.symbols.get(symbol.name).unwrap();
                format!("")
            }
            EKind::Int(x) => {
                target = self.next_vr();
                format!("%{target} = int({x})")
            }
            EKind::Bool(x) => {
                target = self.next_vr();
                format!("%{target} = int({})", *x as u8)
            }
            EKind::Nothing => {
                target = self.vr_count;
                format!("")
            }
            EKind::Str(_) => todo!(),
            EKind::Call { callee, args } => {
                target = self.next_vr();
                let mut nodes: Vec<_> = args.iter().map(|arg| self.emit_expr(arg)).collect();
                let call_instr = match &callee.kind {
                    EKind::Symbol(symbol) => {
                        format!("%{target} = call Fn_{}, {}", symbol.name, args.len())
                    }
                    _ => {
                        let callee = self.emit_expr(callee);
                        let program = format!("%{target} = icall %{}, {}", callee, args.len());
                        nodes.push(callee);
                        program
                    }
                };
                let program = nodes
                    .iter()
                    .map(|param| format!("param %{}\n\t", param))
                    .collect::<Vec<_>>()
                    .join("")
                    + &call_instr;
                program
            }
            EKind::Unary { op, rhs } => match op {
                UnOp::Negate => {
                    let rhs = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = neg %{rhs}")
                }
                UnOp::Not => {
                    let rhs = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = xor %{rhs}, 1")
                }
                UnOp::Deref => {
                    let rhs = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = load %{rhs}")
                }
                UnOp::AddrOf => {
                    target = self.next_vr();
                    match &rhs.kind {
                        EKind::Symbol(symbol) => format!("%{target} = lea(\"{}\")", symbol.name),
                        x => panic!("Haven't done {x:?}"),
                    }
                }
            },
            EKind::Bin { op, lhs, rhs } => match op {
                BinOp::Assign => match &lhs.kind {
                    EKind::Unary {
                        op: UnOp::Deref,
                        rhs: lrhs, // Left right hand side (deref's inner)
                    } => {
                        target = self.emit_expr(lrhs); // This will be derived from AddrOf
                        let rhs = self.emit_expr(rhs);
                        format!("store %{target}, %{rhs}")
                    }
                    _ => {
                        target = self.emit_expr(lhs);
                        let rhs = self.emit_expr(rhs);
                        format!("%{target} = %{}", rhs)
                    }
                },
                op => {
                    let lhs = self.emit_expr(lhs);
                    let rhs = self.emit_expr(rhs);
                    let op_str = match op {
                        BinOp::Add => "add",
                        BinOp::Sub => "sub",
                        BinOp::Mul => "mul",
                        BinOp::Div => "div",
                        BinOp::LogOr => "or",
                        BinOp::LogAnd => "and",
                        BinOp::Gt => "gt",
                        BinOp::Ge => "ge",
                        BinOp::Lt => "lt",
                        BinOp::Le => "le",
                        BinOp::Eq => "eq",
                        BinOp::Ne => "ne",
                        _ => unreachable!(),
                    };
                    target = self.next_vr();
                    format!("%{target} = {op_str} %{}, %{}", lhs, rhs)
                }
            },
            EKind::FieldAccess { .. } => todo!(),
            EKind::Index { .. } => todo!(),
        };
        self.emit_raw(&template);
        target
    }
}
