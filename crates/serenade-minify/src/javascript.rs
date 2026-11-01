use oxc_allocator::Allocator;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_mangler::MangleOptions;
use oxc_minifier::{CompressOptions, Minifier, MinifierOptions};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::MinifyError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptMode {
    Global,
    Module,
}

pub fn script_mode(attributes: &str) -> Option<ScriptMode> {
    let mut attributes = attributes.trim_end_matches('>').trim_start();
    if attributes.ends_with('/') {
        attributes = attributes[..attributes.len() - 1].trim_end();
    }
    let mut module = false;
    let mut script_type = None;

    while !attributes.is_empty() {
        attributes = attributes.trim_start();
        if attributes.is_empty() {
            break;
        }
        let name_end = attributes
            .find(|character: char| character.is_whitespace() || character == '=')
            .unwrap_or(attributes.len());
        if name_end == 0 {
            return None;
        }
        let name = &attributes[..name_end];
        attributes = &attributes[name_end..];
        let mut value = "";
        if let Some(rest) = attributes.strip_prefix('=') {
            attributes = rest.trim_start();
            if let Some(quote) = attributes
                .chars()
                .next()
                .filter(|ch| matches!(ch, '\'' | '"'))
            {
                attributes = &attributes[quote.len_utf8()..];
                let end = attributes.find(quote)?;
                value = &attributes[..end];
                attributes = &attributes[end + quote.len_utf8()..];
            } else {
                let end = attributes
                    .find(char::is_whitespace)
                    .unwrap_or(attributes.len());
                value = &attributes[..end];
                attributes = &attributes[end..];
            }
        }
        if name.eq_ignore_ascii_case("src") {
            return None;
        }
        if name.eq_ignore_ascii_case("type") {
            module = value.eq_ignore_ascii_case("module");
            script_type = Some(
                value
                    .split(';')
                    .next()
                    .unwrap_or(value)
                    .trim()
                    .to_ascii_lowercase(),
            );
        }
    }

    if module {
        return Some(ScriptMode::Module);
    }
    match script_type.as_deref() {
        None
        | Some(
            "text/javascript"
            | "application/javascript"
            | "text/ecmascript"
            | "application/ecmascript"
            | "application/x-javascript",
        ) => Some(ScriptMode::Global),
        _ => None,
    }
}

pub fn minify_script(source: &str, mode: ScriptMode) -> Result<String, MinifyError> {
    let allocator = Allocator::default();
    let source_type = match mode {
        ScriptMode::Global => SourceType::script(),
        ScriptMode::Module => SourceType::mjs(),
    };
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        let details = parsed
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(MinifyError::JavaScript(details));
    }

    let mut program = parsed.program;
    let options = MinifierOptions {
        mangle: Some(MangleOptions::default()),
        compress: Some(CompressOptions::default()),
        ..MinifierOptions::default()
    };
    let minified = Minifier::new(options).minify(&allocator, &mut program);
    let output = Codegen::new()
        .with_options(CodegenOptions::minify())
        .with_scoping(minified.scoping)
        .build(&program)
        .code;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_classic_and_module_script_types() {
        for attributes in [
            "",
            " defer",
            " type='text/javascript'",
            r#" type="application/javascript; charset=utf-8""#,
            " type=text/ecmascript",
            " type=application/ecmascript",
            " type=application/x-javascript",
        ] {
            assert!(matches!(script_mode(attributes), Some(ScriptMode::Global)));
        }

        assert!(matches!(
            script_mode(" type=module"),
            Some(ScriptMode::Module)
        ));
        assert!(matches!(
            script_mode(" type='MODULE'"),
            Some(ScriptMode::Module)
        ));
    }

    #[test]
    fn skips_external_data_and_malformed_script_attributes() {
        assert_eq!(script_mode(" type=module   "), Some(ScriptMode::Module));
        assert!(script_mode(" src=/app.js").is_none());
        assert!(script_mode(" type=application/json").is_none());
        assert!(script_mode(" type='unterminated").is_none());
        assert!(script_mode(" =broken").is_none());
        assert!(script_mode(" type=module src=/app.js").is_none());
        assert!(matches!(
            script_mode(" type=module/"),
            Some(ScriptMode::Module)
        ));
    }

    #[test]
    fn parses_classic_and_module_sources_and_reports_invalid_javascript() {
        let classic = "function greet () { return 1 + 2; }";
        assert_ne!(
            minify_script(classic, ScriptMode::Global).expect("classic script parses"),
            classic
        );
        assert!(minify_script("await Promise.resolve(1);", ScriptMode::Module).is_ok());
        assert!(matches!(
            minify_script("const = ;", ScriptMode::Global),
            Err(MinifyError::JavaScript(_))
        ));
    }
}
