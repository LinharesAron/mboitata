use std::collections::HashMap;

pub mod analyzer;
pub mod extractor;
pub mod usage;
pub mod types;

#[derive(Default)]
pub struct JsResult {
    pub calls: Vec<HttpCall>,
    pub vars: HashMap<String, String>
}

impl JsResult {
    fn new(vars: HashMap<String, String>) -> Self {
        Self {
            calls: Vec::new(),
            vars,
        }
    }
}

#[derive(Debug)]
pub struct HttpCall {
    pub method: String,
    pub url: String,
    pub authorization: Option<String>,
}