use std::collections::HashMap;

use swc_ecma_ast::{
    AssignTarget, BinExpr, BinaryOp, CallExpr, Expr, FnDecl, Lit, MemberExpr, MemberProp,
    ObjectLit, Pat, Prop, PropName, PropOrSpread, SimpleAssignTarget, UnaryExpr, VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::analyzer::stages::js::types::prop_as_keypub;

pub struct VarExtractor {
    pub vars: HashMap<String, String>,
    pub stack: Vec<String>,
}

impl VarExtractor {
    fn insert_var(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    fn extract_props(&mut self, expr: &Expr, prefix: String) {
        match expr {
            Expr::Lit(lit) => match lit {
                Lit::Str(s) => {
                    self.insert_var(prefix, s.value.to_string());
                }
                Lit::Bool(b) => {
                    self.insert_var(prefix, b.value.to_string());
                }
                Lit::Num(n) => {
                    self.insert_var(prefix, n.value.to_string());
                }
                _ => {}
            },

            Expr::Tpl(tpl) if tpl.exprs.is_empty() => {
                let raw = tpl
                    .quasis
                    .iter()
                    .map(|q| q.raw.to_string())
                    .collect::<String>();
                self.insert_var(prefix, raw);
            }

            Expr::Tpl(_) => {
                self.insert_var(prefix, "<interpolated>".to_string());
            }

            Expr::Ident(id) => {
                self.insert_var(prefix, id.sym.to_string());
            }

            Expr::Paren(p) => {
                self.extract_props(&p.expr, prefix);
            }

            Expr::Object(obj) => {
                for (key, value) in obj.props.iter().filter_map(prop_as_keypub) {
                    let key_str = match key {
                        PropName::Ident(i) => i.sym.to_string(),
                        PropName::Str(s) => s.value.to_string(),
                        _ => continue,
                    };
                    let full_key = format!("{}.{}", prefix, key_str);
                    self.extract_props(value, full_key);
                }
            }

            Expr::Array(arr) => {
                for (i, el) in arr.elems.iter().enumerate() {
                    if let Some(expr) = el {
                        let full_key = format!("{}[{}]", prefix, i);
                        self.extract_props(&expr.expr, full_key);
                    }
                }
            }

            Expr::Bin(BinExpr {
                op: BinaryOp::LogicalOr | BinaryOp::NullishCoalescing,
                left,
                right,
                ..
            }) => {
                self.extract_props(left, format!("{} (left)", prefix));
                self.extract_props(right, format!("{} (right)", prefix));
            }

            Expr::Bin(BinExpr {
                op: operator,
                left,
                right,
                ..
            }) => {
                let mut value = vec![];

                let left_val = get_expr_string(left);
                let right_val = get_expr_string(right);

                if let Some(l) = left_val {
                    value.push(l);
                }
                if let Some(r) = right_val {
                    value.push(r);
                }

                if !value.is_empty() {
                    self.insert_var(prefix, value.join(operator.as_str()));
                }
            }

            Expr::Unary(UnaryExpr { arg, op, .. }) => {
                if let Expr::Lit(Lit::Num(n)) = &**arg {
                    let value = match op {
                        swc_ecma_ast::UnaryOp::Minus => -n.value,
                        swc_ecma_ast::UnaryOp::Plus => n.value,
                        _ => n.value,
                    };
                    self.insert_var(prefix, value.to_string());
                } else if let Expr::Lit(Lit::Bool(b)) = &**arg {
                    let value = match op {
                        swc_ecma_ast::UnaryOp::Bang => (!b.value).to_string(),
                        _ => b.value.to_string(),
                    };
                    self.insert_var(prefix, value);
                }
            }

            Expr::Call(call) => {
                if let Some(first_arg) = call.args.first() {
                    self.extract_props(&first_arg.expr, format!("{}()", prefix));
                }
            }

            Expr::Member(member_expr) => {
                let mut name = String::new();

                if let Expr::Ident(obj) = &*member_expr.obj {
                    name.push_str(&obj.sym);
                }

                if let MemberProp::Ident(prop) = &member_expr.prop {
                    name.push('.');
                    name.push_str(&prop.sym);
                }

                self.insert_var(prefix, name);
            }
            _ => {}
        }
    }
}

impl Visit for VarExtractor {
    fn visit_fn_decl(&mut self, node: &FnDecl) {
        self.stack.push(node.ident.sym.to_string());
        node.function.visit_children_with(self);
        self.stack.pop();
    }

    fn visit_object_lit(&mut self, obj: &ObjectLit) {
        for prop in &obj.props {
            if let PropOrSpread::Prop(p) = prop {
                match &**p {
                    Prop::KeyValue(kv) => {
                        if let PropName::Ident(ident) = &kv.key {
                            self.stack.push(ident.sym.to_string());
                            kv.value.visit_with(self);
                            self.stack.pop();
                        }
                    }
                    Prop::Method(method) => {
                        if let PropName::Ident(ident) = &method.key {
                            self.stack.push(ident.sym.to_string());
                            prop.visit_with(self);
                            self.stack.pop();
                        }
                    }
                    
                    _ => {}
                }
            }
        }

        obj.visit_children_with(self);
    }

    fn visit_var_declarator(&mut self, node: &VarDeclarator) {
        if let Pat::Ident(ident) = &node.name {
            if let Some(expr) = &node.init {
                let key = format!("{}::{}", self.stack.join("::"), ident.sym.to_string());
                self.extract_props(expr, key);
            }
        }
        node.visit_children_with(self);
    }

    fn visit_expr(&mut self, node: &Expr) {
        match &*node {
            Expr::Assign(assign_expr) => match &assign_expr.left {
                AssignTarget::Simple(target) => match target {
                    SimpleAssignTarget::Member(member_expr) => {
                        let mut lhs_str = vec![];

                        match &*member_expr.obj {
                            Expr::Ident(obj) => lhs_str.push(obj.sym.to_string()),
                            Expr::This(_) => lhs_str.push("this".into()),
                            _ => {}
                        }

                        if let MemberProp::Ident(prop) = &member_expr.prop {
                            lhs_str.push(prop.sym.to_string());
                        }
                        self.extract_props(&*&assign_expr.right, lhs_str.join("."));
                    }
                    _ => {}
                },
                AssignTarget::Pat(_) => {}
            }
            _ => {}
        }
        node.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, node: &CallExpr) {
        if let Some(expr) = node.callee.as_expr() {
            if let Expr::Member(MemberExpr { .. }) = &**expr {
                if node.args.len() == 2 {
                    let name_arg = &node.args[0];
                    let value_arg = &node.args[1];

                    if let Expr::Lit(Lit::Str(name)) = &*name_arg.expr {
                        let prefix = name.value.to_string();

                        if let Expr::Object(..) = &*value_arg.expr {
                            self.extract_props(&value_arg.expr, prefix);
                        }
                    }
                }
            }
        }
        node.visit_children_with(self);
    }
}

fn get_expr_string(expr: &Box<Expr>) -> Option<String> {
    match &**expr {
        Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
        Expr::Lit(Lit::Num(n)) => Some(n.value.to_string()),
        Expr::Ident(id) => Some(id.sym.to_string()),
        Expr::Tpl(tpl) if tpl.exprs.is_empty() => Some(
            tpl.quasis
                .iter()
                .map(|q| q.raw.to_string())
                .collect::<String>(),
        ),
        Expr::Member(MemberExpr {
            obj,
            prop: MemberProp::Ident(prop_id),
            ..
        }) => {
            if let Expr::Ident(id) = &**obj {
                Some(format!(
                    "{}.{}",
                    id.sym.to_string(),
                    prop_id.sym.to_string()
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}
