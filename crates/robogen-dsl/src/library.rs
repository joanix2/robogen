use super::{lex, parse, Declaration, TokenKind};
use robogen_domain::{Diagnostic, SourceSpan};
use std::collections::{BTreeMap, BTreeSet};

pub fn component_library(
    source: &str,
    prefix: &str,
    entrypoint: &str,
) -> Result<String, Vec<Diagnostic>> {
    let identifier = |name: &str| {
        !name.is_empty()
            && name.bytes().enumerate().all(|(index, byte)| {
                byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
            })
    };
    if !identifier(prefix) || !identifier(entrypoint) {
        return Err(vec![Diagnostic::error(
            "E330",
            "invalid library identifier",
            SourceSpan::default(),
        )]);
    }
    let parsed = parse(source);
    if !parsed.diagnostics.is_empty() {
        return Err(parsed.diagnostics);
    }
    let Some(module) = parsed.module else {
        return Err(vec![Diagnostic::error(
            "E330",
            "missing library module",
            SourceSpan::default(),
        )]);
    };
    let mut renames = BTreeMap::new();
    let mut has_entrypoint = false;
    for declaration in &module.declarations {
        let name = match declaration {
            Declaration::Parameter(value) => &value.name,
            Declaration::Material(value) => &value.name,
            Declaration::Component(value) => {
                has_entrypoint |= value.name == entrypoint;
                &value.name
            }
            Declaration::Part(value) => &value.name,
            Declaration::Sketch(value) => {
                return Err(vec![Diagnostic::error(
                    "E330",
                    "sketch libraries are not supported",
                    value.span,
                )])
            }
        };
        renames.insert(
            name.clone(),
            if matches!(declaration, Declaration::Component(_)) && name == entrypoint {
                name.clone()
            } else {
                format!("{prefix}{name}")
            },
        );
    }
    let rewrite = |span: SourceSpan, locals: &BTreeSet<String>| {
        let text = &source[span.start..span.end];
        let (tokens, _) = lex(text);
        let mut result = text.to_owned();
        for (index, token) in tokens.iter().enumerate().rev() {
            let TokenKind::Ident(name) = &token.kind else {
                continue;
            };
            let Some(replacement) = renames.get(name) else {
                continue;
            };
            let next = tokens.get(index + 1).map(|token| &token.kind);
            if next == Some(&TokenKind::Symbol(':'))
                || (locals.contains(name) && next != Some(&TokenKind::Symbol('(')))
            {
                continue;
            }
            result.replace_range(token.span.start..token.span.end, replacement);
        }
        result
    };
    let mut result = String::new();
    let mut parts = Vec::new();
    for declaration in &module.declarations {
        let empty = BTreeSet::new();
        let text = match declaration {
            Declaration::Parameter(value) => rewrite(value.span, &empty),
            Declaration::Material(value) => rewrite(value.span, &empty),
            Declaration::Component(value) => {
                let locals = value
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .chain(value.bindings.iter().map(|binding| binding.name.clone()))
                    .collect();
                rewrite(value.span, &locals)
            }
            Declaration::Part(value) if !has_entrypoint => {
                if value.features.is_empty()
                    || value
                        .features
                        .iter()
                        .any(|feature| feature.operation == "extrude")
                {
                    return Err(vec![Diagnostic::error(
                        "E330",
                        "library parts must contain solid features",
                        value.span,
                    )]);
                }
                let name = &renames[&value.name];
                parts.push(format!("{name}()"));
                let locals = value
                    .features
                    .iter()
                    .map(|feature| feature.name.clone())
                    .collect();
                let mut text = format!("component {name}() -> Solid {{\n");
                for feature in &value.features {
                    text.push_str(&rewrite(feature.span, &locals));
                    text.push('\n');
                }
                text.push_str(&format!(
                    "return compound({});\n}}",
                    value
                        .features
                        .iter()
                        .map(|feature| feature.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                text
            }
            Declaration::Part(_) => continue,
            Declaration::Sketch(_) => continue,
        };
        result.push_str(&text);
        result.push_str("\n\n");
    }
    if !has_entrypoint {
        if parts.is_empty() {
            return Err(vec![Diagnostic::error(
                "E330",
                "library has no solid parts",
                module.span,
            )]);
        }
        result.push_str(&format!(
            "component {entrypoint}() -> Solid {{ return compound({}); }}\n",
            parts.join(", ")
        ));
    }
    let check = parse(&format!("module library;\n{result}"));
    if !check.diagnostics.is_empty() {
        return Err(check.diagnostics);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_rewrites_global_names_but_preserves_local_names_and_comments(
    ) -> Result<(), Vec<Diagnostic>> {
        let source = "module demo;
parameter WIDTH = 2 mm;
material Plastic { density: 1000 kg/m3; young: 1 GPa; poisson: 0.3; yield_strength: 10 MPa; }
component cube(WIDTH: Length = 3 mm) -> Solid { return box([WIDTH, WIDTH, WIDTH]); }
component shape() -> Solid { // WIDTH stays in this comment
return cube(WIDTH: WIDTH); }
part Item { material: Plastic; body = shape(); }";
        let result = component_library(source, "lib_", "shape")?;
        assert!(result.contains("parameter lib_WIDTH"));
        assert!(result.contains("component lib_cube(WIDTH: Length"));
        assert!(result.contains("box([WIDTH, WIDTH, WIDTH])"));
        assert!(result.contains("lib_cube(WIDTH: lib_WIDTH)"));
        assert!(result.contains("// WIDTH stays in this comment"));
        assert!(!result.contains("part Item"));
        Ok(())
    }
}
