use std::collections::HashMap;

use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};
use swc_ecma_visit::VisitWith;

use crate::analyzer::stages::js::{JsResult, extractor::VarExtractor, usage::JsUsageAnalyzer};

pub fn run_js_analysis(filename: &str, js_code: &str) -> anyhow::Result<JsResult> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        FileName::Custom(filename.to_string()).into(),
        js_code.to_string(),
    );

    let lexer = Parser::new(
        Syntax::Es(EsSyntax {
            jsx: false,
            ..Default::default()
        }),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = lexer;
    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(err) => return Err(anyhow::anyhow!("Erro ao fazer parse: {:?}", err)),
    };

    let mut var_extractor = VarExtractor {
        vars: HashMap::new(),
        stack: Vec::new()
    };
    module.visit_with(&mut var_extractor);

    let mut usage_analyzer = JsUsageAnalyzer {
        result: JsResult::new(var_extractor.vars),
    };
    module.visit_with(&mut usage_analyzer);

    Ok(usage_analyzer.result)
}
