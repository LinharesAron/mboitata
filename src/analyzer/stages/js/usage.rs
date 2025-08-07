use std::collections::HashMap;
use swc_ecma_ast::{
    BinaryOp, CallExpr, Callee, Expr, ExprOrSpread, Lit, MemberExpr, MemberProp, PropName, Tpl,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::analyzer::stages::js::{HttpCall, JsResult, types::prop_as_keypub};

pub struct JsUsageAnalyzer {
    pub result: JsResult,
}

impl JsUsageAnalyzer {
    fn handle_http_call(&mut self, method: &str, args: &[ExprOrSpread]) {
        let url = args
            .get(0)
            .map(|arg| resolve_expr(&arg.expr, &self.result.vars))
            .unwrap_or_else(|| "[unknown]".to_string());

        if !is_likely_url(&url) {
            return;
        }

        let authorization = extract_authorization_from_args(args, &self.result.vars);

        self.result.calls.push(HttpCall {
            method: method.to_string(),
            url,
            authorization,
        });
    }
}

impl Visit for JsUsageAnalyzer {
    fn visit_call_expr(&mut self, call: &CallExpr) {
        if let Callee::Expr(expr) = &call.callee {
            match &**expr {
                Expr::Member(MemberExpr {
                    prop: MemberProp::Ident(method),
                    ..
                }) => {
                    if is_http_method(&method.sym) {
                        self.handle_http_call(&method.sym, &call.args);
                    }
                }

                Expr::Ident(ident) if is_http_client_name(&ident.sym) => {
                    self.handle_http_call(&ident.sym, &call.args);
                }

                _ => {}
            }
        }

        call.visit_children_with(self);
    }
}

fn is_http_method(name: &str) -> bool {
    matches!(name, "get" | "post" | "put" | "delete" | "patch" | "fetch")
}

fn is_http_client_name(name: &str) -> bool {
    name.contains("http")
        || name.contains("fetch")
        || name.contains("axios")
        || name.contains("client")
        || name.contains("api")
}

fn is_likely_url(url: &str) -> bool {
    url.contains("/")
        || url.starts_with("http")
        || url.starts_with("./")
        || url.starts_with("../")
        || url.contains('?')
        || url.contains("api")
        || url.contains("${")
}

pub fn resolve_expr(expr: &Expr, vars: &HashMap<String, String>) -> String {
    match expr {
        Expr::Bin(bin) if bin.op == BinaryOp::Add => {
            let left = resolve_expr(&bin.left, vars);
            let right = resolve_expr(&bin.right, vars);
            format!("{left}{right}")
        }

        Expr::Lit(Lit::Str(s)) => s.value.to_string(),
        Expr::Lit(Lit::Num(n)) => n.value.to_string(),
        Expr::Lit(Lit::Bool(b)) => b.value.to_string(),
        Expr::Lit(Lit::Null(_)) => "null".to_string(),

        Expr::Ident(ident) => vars
            .get(&ident.sym.to_string())
            .cloned()
            .unwrap_or_else(|| format!("${{{}}}", ident.sym)),

        Expr::Tpl(Tpl { quasis, exprs, .. }) => {
            let mut result = String::new();
            for (i, quasi) in quasis.iter().enumerate() {
                result += &quasi.raw;

                if let Some(expr) = exprs.get(i) {
                    result += &format!("${{{}}}", stringify_expr(expr, vars));
                }
            }
            result
        }

        Expr::Member(MemberExpr { obj, prop, .. }) => {
            let obj_str = resolve_expr(&obj, vars);
            let prop_str = match prop {
                MemberProp::Ident(id) => {
                    if prop.is_computed() {
                        format!("[{}]", id.sym)
                    } else {
                        format!(".{}", id.sym)
                    }
                }
                MemberProp::Computed(expr) => {
                    let inner = resolve_expr(&expr.expr, vars);
                    format!("[{inner}]")
                }
                _ => "[computed]".into(),
            };
            format!("{obj_str}{prop_str}")
        }

        _ => "[expr]".to_string(),
    }
}

fn stringify_expr(expr: &Expr, vars: &HashMap<String, String>) -> String {
    match expr {
        Expr::Ident(id) => vars
            .get(&id.sym.to_string())
            .cloned()
            .unwrap_or_else(|| id.sym.to_string()),
        Expr::Lit(Lit::Str(s)) => s.value.to_string(),
        Expr::Lit(Lit::Num(n)) => n.value.to_string(),
        Expr::Member(_) => resolve_expr(expr, vars),
        _ => "[expr]".to_string(),
    }
}

pub fn extract_authorization_from_args(
    args: &[ExprOrSpread],
    vars: &HashMap<String, String>,
) -> Option<String> {
    for arg in args.iter().skip(1) {
        if let Expr::Object(obj) = &*arg.expr {
            for (key, value) in obj.props.iter().filter_map(prop_as_keypub) {
                if is_prop_named(key, "headers") {
                    if let Expr::Object(header_obj) = &**value {
                        for (key, value) in header_obj.props.iter().filter_map(prop_as_keypub) {
                            if is_prop_named(key, "authorization") {
                                return Some(resolve_expr(&value, vars));
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn is_prop_named(key: &PropName, name: &str) -> bool {
    match key {
        PropName::Ident(ident) => ident.sym.to_ascii_lowercase() == name.to_ascii_lowercase(),
        PropName::Str(str) => str.value.to_ascii_lowercase() == name.to_ascii_lowercase(),
        _ => false,
    }
}
