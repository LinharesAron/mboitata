use swc_ecma_ast::{Expr, KeyValueProp, Prop, PropName, PropOrSpread};

pub fn prop_as_keypub(prop_or_spread: &PropOrSpread) -> Option<(&PropName, &Box<Expr>)> {
    if let PropOrSpread::Prop(p) = prop_or_spread {
        if let Prop::KeyValue(KeyValueProp { key, value }) = &**p {
            return Some((key, value));
        }
    }
    None
}