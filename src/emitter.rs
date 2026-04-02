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

enum Ir {
    /* FUNCTIONS */
    /// get dst, index
    GetParam(VReg, u8),
    /// set src, index
    SetParam(VReg, u8),
    /// call dst, NAME
    Call(VReg, String),
    /// icall dst, src
    Icall(VReg, VReg),
    /// ret (src)
    Ret(Option<VReg>),

    /* DATA */
    /// imm dst, width, imm
    Imm(VReg, u8, u64),
    /// addr dst, index
    Addr(VReg, u64),
    /// load dst, width, src
    Load(VReg, u8, VReg),
    /// store src2, width, src1
    Store(VReg, u8, VReg),

    /* ARITHMETIC */
    /// add, dst, src1, src2
    Add(VReg, VReg, VReg),
    /// Sub, dst, src1, src2
    Sub(VReg, VReg, VReg),
    /// Mul, dst, src1, src2
    Mul(VReg, VReg, VReg),
    /// Sdiv, dst, src1, src2
    Sdiv(VReg, VReg, VReg),
    /// udiv, dst, src1, src2
    Udiv(VReg, VReg, VReg),

    /* ORDERING */
    /// sgt dst, src1, src2
    Sgt(VReg, VReg, VReg),
    /// sge dst, src1, src2
    Sge(VReg, VReg, VReg),
    /// ugt dst, src1, src2
    Ugt(VReg, VReg, VReg),
    /// uge dst, src1, src2
    Uge(VReg, VReg, VReg),
    /// slt dst, src1, src2
    Slt(VReg, VReg, VReg),
    /// sle dst, src1, src2
    Sle(VReg, VReg, VReg),
    /// ult dst, src1, src2
    Ult(VReg, VReg, VReg),
    /// ule dst, src1, src2
    Ule(VReg, VReg, VReg),

    /* EQUALITY */
    /// eq dst, src1, src2
    Eq(VReg, VReg, VReg),
    /// ne dst, src1, src2
    Ne(VReg, VReg, VReg),

    /* LOGIC */
    /// and dst, src1, src2
    And(VReg, VReg, VReg),
    /// or dst, src1, src2
    Or(VReg, VReg, VReg),
    /// xor dst, src1, src2
    Xor(VReg, VReg, VReg),

    /* CONTROL FLOW */
    /// beq src1, src2, LABEL
    Beq(VReg, VReg, String),
    /// bne src1, src2, LABEL
    Bne(VReg, VReg, String),
    Br(String),
}

impl std::fmt::Display for Ir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[rustfmt::skip]
        let s = match self {
            Ir::GetParam(dst, index)     => format!("getp %{dst}, {index}"),
            Ir::SetParam(src, index)     => format!("setp %{src}, {index}"),
            Ir::Call(dst, name)          => format!("call %{dst}, {name}"),
            Ir::Icall(dst, src)          => format!("icall %{dst}, %{src}"),
            Ir::Ret(src)                 => format!("ret {}", src.map(|r| format!("%{r}")).unwrap_or("".to_string())),
            Ir::Imm(dst, width, imm)     => format!("imm %{dst}, {width}, {imm}"),
            Ir::Addr(dst, index)         => format!("addr %{dst}, {index}"),
            Ir::Load(dst, width, src)    => format!("load %{dst}, {width}, %{src}"),
            Ir::Store(src2, width, src1) => format!("store %{src1}, {width}, %{src2}"),
            Ir::Add(dst, src1, src2)     => format!("add %{dst}, %{src1}, %{src2}"),
            Ir::Sub(dst, src1, src2)     => format!("sub %{dst}, %{src1}, %{src2}"),
            Ir::Mul(dst, src1, src2)     => format!("mul %{dst}, %{src1}, %{src2}"),
            Ir::Sdiv(dst, src1, src2)    => format!("sdiv %{dst}, %{src1}, %{src2}"),
            Ir::Udiv(dst, src1, src2)    => format!("udiv %{dst}, %{src1}, %{src2}"),
            Ir::Sgt(dst, src1, src2)     => format!("sgt %{dst}, %{src1}, %{src2}"),
            Ir::Sge(dst, src1, src2)     => format!("sge %{dst}, %{src1}, %{src2}"),
            Ir::Ugt(dst, src1, src2)     => format!("ugt %{dst}, %{src1}, %{src2}"),
            Ir::Uge(dst, src1, src2)     => format!("uge %{dst}, %{src1}, %{src2}"),
            Ir::Slt(dst, src1, src2)     => format!("slt %{dst}, %{src1}, %{src2}"),
            Ir::Sle(dst, src1, src2)     => format!("sle %{dst}, %{src1}, %{src2}"),
            Ir::Ult(dst, src1, src2)     => format!("ult %{dst}, %{src1}, %{src2}"),
            Ir::Ule(dst, src1, src2)     => format!("ule %{dst}, %{src1}, %{src2}"),
            Ir::Eq(dst, src1, src2)      => format!("eq %{dst}, %{src1}, %{src2}"),
            Ir::Ne(dst, src1, src2)      => format!("ne %{dst}, %{src1}, %{src2}"),
            Ir::And(dst, src1, src2)     => format!("and %{dst}, %{src1}, %{src2}"),
            Ir::Or(dst, src1, src2)      => format!("or %{dst}, %{src1}, %{src2}"),
            Ir::Xor(dst, src1, src2)     => format!("xor %{dst}, %{src1}, %{src2}"),
            Ir::Beq(src1, src2, label)   => format!("beq %{src1}, %{src2}, {label}"),
            Ir::Bne(src1, src2, label)   => format!("bne %{src1}, %{src2}, {label}"),
            Ir::Br(label)                => format!("br {label}"),
        };
        f.write_str(&s)
    }
}

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

    fn next_label(&mut self, stmt: &Stmt<'src, Resolved>) -> String {
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

    pub fn emit_program(mut self, objs: &[Object<'src, Resolved>]) -> Vec<String> {
        for obj in objs {
            self.emit_object(obj);
        }
        self.program
    }

    fn emit_object(&mut self, obj: &Object<'src, Resolved>) {
        match &obj.kind {
            OKind::Fn {
                name,
                returns,
                args,
                body,
            } => {
                // START FUNCTION
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
                // END FUNCTION
                if *returns == Resolved::Void && !self.program.last().unwrap().contains("\tret\n") {
                    self.emit_raw("ret");
                }
                self.emit_label(&format!("FnEnd_{name}"));
            }
            OKind::Global { .. } => todo!(),
            OKind::Struct { .. } => todo!(),
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt<'src, Resolved>) {
        match &stmt.kind {
            SKind::Let { lhs, rhs } => {
                let target = self.next_vr();
                if self.symbols.insert(lhs.name, target).is_some() {
                    panic!("Duplicate symbol {:?}", lhs);
                }
                self.emit_raw(&format!(
                    "%{target} = local({}, \"{}\")",
                    lhs.ty.width(),
                    lhs.name
                ));
                let rhs_vr = self.emit_expr(rhs);
                let program = format!("%{target} = %{}", rhs_vr);
                self.emit_raw(&program);
            }
            SKind::While { cond, body } => {
                let label = self.next_label(stmt);
                let start_label = label.clone() + "_loop";
                let end_label = label + "_end";
                let cond = self.emit_expr(cond);
                let target = self.next_vr();
                self.emit_raw(&format!("%{target} = int({}, 0)", Resolved::Bool.width()));
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
                self.emit_raw(&format!("%{target} = int({}, 0)", Resolved::Bool.width()));
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
                if expr.ty != Resolved::Void {
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

    fn emit_expr(&mut self, Expr { kind, ty, .. }: &Expr<'src, Resolved>) -> VReg {
        let target;
        let template = match kind {
            EKind::Symbol(symbol) => {
                target = *self.symbols.get(symbol.name).unwrap();
                format!("")
            }
            EKind::Int(x) => {
                target = self.next_vr();
                format!("%{target} = int({}, {x})", ty.width())
            }
            EKind::Bool(x) => {
                target = self.next_vr();
                format!("%{target} = int(u32, {})", *x as u8)
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
                    let rhs_vr = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = neg %{rhs_vr}")
                }
                UnOp::Not => {
                    let rhs_vr = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = xor %{rhs_vr}, 1")
                }
                UnOp::Deref => {
                    let rhs_vr = self.emit_expr(rhs);
                    target = self.next_vr();
                    format!("%{target} = load %{rhs_vr}")
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
                        let rhs_vr = self.emit_expr(rhs);
                        format!("store %{target}, %{rhs_vr}")
                    }
                    _ => {
                        target = self.emit_expr(lhs);
                        let rhs_vr = self.emit_expr(rhs);
                        format!("%{target} = %{rhs_vr}")
                    }
                },
                op => {
                    let lhs_vr = self.emit_expr(lhs);
                    let rhs_vr = self.emit_expr(rhs);
                    #[rustfmt::skip]
                    let op_str = match op {
                        BinOp::LogOr  => "or",
                        BinOp::LogAnd => "and",
                        BinOp::Add    => "add",
                        BinOp::Sub    => "sub",
                        BinOp::Mul    => "mul",
                        BinOp::Eq     => "eq",
                        BinOp::Ne     => "ne",
                        // The following instructions have unsigned counterparts
                        BinOp::Div => if lhs.ty.signed() { "sdiv" } else { "udiv" }
                        BinOp::Gt  => if lhs.ty.signed() { "sgt"  } else { "ugt"  }
                        BinOp::Ge  => if lhs.ty.signed() { "sge"  } else { "uge"  }
                        BinOp::Lt  => if lhs.ty.signed() { "slt"  } else { "ult"  }
                        BinOp::Le  => if lhs.ty.signed() { "sle"  } else { "ule"  }
                        _ => unreachable!(),
                    };
                    target = self.next_vr();
                    format!("%{target} = {op_str} %{}, %{}", lhs_vr, rhs_vr)
                }
            },
            EKind::FieldAccess { .. } => todo!(),
            EKind::Index { .. } => todo!(),
        };
        self.emit_raw(&template);
        target
    }
}
