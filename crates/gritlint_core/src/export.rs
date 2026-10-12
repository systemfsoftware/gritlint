use crate::domain::{ExportRefusal, ParsedRule, RuleKind};
use grit_util::{Ast as _, AstNode as _, Order, traverse};
use marzano_language::grit_parser::MarzanoGritParser;
use marzano_util::node_with_source::NodeWithSource;
use std::collections::BTreeSet;
use std::fmt::Write as _;

const LANGUAGES: [&str; 3] = ["css", "js", "json"];
const JS_FLAVORS: [&str; 2] = ["typescript", "jsx"];
const WRAPPER: &str = "gritlint_exported_rule";
const MATCH: &str = "$gritlint_match";

pub fn find<'a>(rules: &[&'a ParsedRule], id: &str) -> Result<&'a ParsedRule, ExportRefusal> {
    rules
        .iter()
        .copied()
        .find(|rule| rule.id().as_string() == id)
        .ok_or_else(|| {
            ExportRefusal::new(
                id.to_owned(),
                "export-unknown-rule",
                "no enabled rule has this id.".to_owned(),
                "run `gritlint export biome` without `--rule` to list the enabled rule ids.",
            )
        })
}

pub fn biome(rule: &ParsedRule, all: &[&ParsedRule]) -> Result<String, ExportRefusal> {
    let refuse = |reason_code, detail: String, next_action| {
        ExportRefusal::new(rule.id().as_string(), reason_code, detail, next_action)
    };
    if rule.kind() == RuleKind::MultiFile {
        return Err(refuse(
            "export-multifile",
            "the rule is multifile, and a Biome plugin sees one file at a time.".to_owned(),
            "keep this rule in `gritlint check`; only a single-file rule exports.",
        ));
    }
    let tree = MarzanoGritParser::new()
        .and_then(|mut parser| parser.parse_file(rule.body(), None))
        .map_err(|error| {
            refuse(
                "export-construct",
                format!("the rule body does not parse as GritQL: {error}."),
                "report the rule to gritlint; `gritlint check` compiled it.",
            )
        })?;
    let root = tree.root_node();
    let body = Body::read(&root);
    if let Some(language) = body.unsupported_language() {
        return Err(refuse(
            "export-language",
            format!(
                "`language {language}` is not a Biome plugin language: Biome plugins take css, js and json, and js only without a flavor or as typescript or jsx."
            ),
            "keep this rule in `gritlint check`, or write it for a language Biome plugins support.",
        ));
    }
    if body.foreign_function {
        return Err(refuse(
            "export-construct",
            "the rule defines a JavaScript function (`function … js { … }`), which Biome does not compile.".to_owned(),
            "rewrite the function as a GritQL pattern or predicate, or keep this rule in `gritlint check`.",
        ));
    }
    if body.rewrite {
        return Err(refuse(
            "export-rewrite",
            "the rule rewrites (`=>`), and gritlint never edits files.".to_owned(),
            "remove the rewrite to export a rule that only reports.",
        ));
    }
    let scope = rule.id().scope();
    let siblings: BTreeSet<&str> = all
        .iter()
        .filter(|other| other.id() != rule.id() && other.id().scope() == scope)
        .map(|other| other.name().as_str())
        .collect();
    if let Some(called) = body
        .calls
        .difference(&body.defined)
        .find(|name| siblings.contains(name.as_str()))
    {
        return Err(refuse(
            "export-library",
            format!(
                "the rule calls `{called}`, another rule of its pack, which a plugin cannot reach."
            ),
            "inline the called rule's pattern into this rule, or keep this rule in `gritlint check`.",
        ));
    }
    Ok(render(rule, &body))
}

#[must_use]
pub fn listing(rules: &[&ParsedRule]) -> String {
    let mut sorted: Vec<&ParsedRule> = rules.to_vec();
    sorted.sort_by_cached_key(|rule| rule.id().as_string());
    sorted.iter().fold(String::new(), |mut out, rule| {
        let line = match biome(rule, rules) {
            Ok(_) => format!("{} exportable", rule.id()),
            Err(refusal) => format!(
                "{} refused {}: {}",
                rule.id(),
                refusal.reason_code(),
                refusal.next_action()
            ),
        };
        let _ = writeln!(out, "{line}");
        out
    })
}

#[derive(Default)]
struct Body {
    language: String,
    language_name: String,
    flavors: Vec<String>,
    definitions: Vec<String>,
    pattern: String,
    foreign_function: bool,
    rewrite: bool,
    calls: BTreeSet<String>,
    defined: BTreeSet<String>,
}

impl Body {
    fn read(root: &NodeWithSource<'_>) -> Self {
        let mut body = Self::default();
        if let Some(language) = root.child_by_field_name("language") {
            body.language = text(&language);
            body.language_name = language
                .child_by_field_name("name")
                .map(|name| text(&name))
                .unwrap_or_default();
            body.flavors = language
                .children_by_field_name("flavor")
                .map(|flavor| text(&flavor).trim_matches(['(', ')']).trim().to_owned())
                .filter(|flavor| !flavor.is_empty())
                .collect();
        }
        body.definitions = root
            .named_children_by_field_name("definitions")
            .map(|definition| text(&definition))
            .collect();
        body.pattern = pattern_text(root);
        traverse(root.walk(), Order::Pre).for_each(|node| body.visit(&node));
        body
    }

    fn visit(&mut self, node: &NodeWithSource<'_>) {
        match node.node.kind().as_ref() {
            "foreignFunctionDefinition" => self.foreign_function = true,
            "rewrite" | "predicateRewrite" => self.rewrite = true,
            "nodeLike" | "predicateCall" => {
                self.calls
                    .extend(node.child_by_field_name("name").map(|name| text(&name)));
            }
            "patternDefinition" | "predicateDefinition" | "functionDefinition" => {
                self.defined
                    .extend(node.child_by_field_name("name").map(|name| text(&name)));
            }
            _ => {}
        }
    }

    fn unsupported_language(&self) -> Option<&str> {
        let name_supported = LANGUAGES.contains(&self.language_name.as_str());
        let flavors_supported = self.language_name != "js"
            || self
                .flavors
                .iter()
                .all(|flavor| JS_FLAVORS.contains(&flavor.as_str()));
        (!(name_supported && flavors_supported)).then(|| {
            self.language
                .strip_prefix("language")
                .unwrap_or(&self.language)
                .trim()
        })
    }
}

fn pattern_text(root: &NodeWithSource<'_>) -> String {
    let pieces: Vec<NodeWithSource<'_>> = root.children_by_field_name("pattern").collect();
    let (Some(first), Some(last)) = (pieces.first(), pieces.last()) else {
        return String::new();
    };
    let start = first.range().start_byte;
    let end = last.range().end_byte;
    usize::try_from(start)
        .ok()
        .zip(usize::try_from(end).ok())
        .and_then(|(start, end)| root.source.get(start..end))
        .unwrap_or_default()
        .to_owned()
}

fn text(node: &NodeWithSource<'_>) -> String {
    node.text()
        .map(|text| text.trim().to_owned())
        .unwrap_or_default()
}

fn render(rule: &ParsedRule, body: &Body) -> String {
    let definitions: Vec<String> = rule
        .engine_parameters()
        .into_iter()
        .map(|(name, value)| crate::engine::parameter_definition(&name, &value))
        .chain(body.definitions.iter().cloned())
        .collect();
    let sections = [
        body.language.clone(),
        definitions.join("\n"),
        format!("pattern {WRAPPER}() {{\n  {}\n}}", body.pattern),
        format!(
            "{WRAPPER}() as {MATCH} where {{\n  register_diagnostic(span = {MATCH}, message = \"{}\", severity = \"error\")\n}}",
            escape(rule.message())
        ),
    ];
    let mut plugin = sections
        .into_iter()
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    plugin.push('\n');
    plugin
}

fn escape(message: &str) -> String {
    message.chars().fold(
        String::with_capacity(message.len()),
        |mut out, character| {
            match character {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                other => out.push(other),
            }
            out
        },
    )
}
