use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_till, take_while1},
    character::complete::char,
    combinator::opt,
    multi::many0,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::MissingField;

/// Template AST node types
#[derive(Debug, Clone)]
enum TemplateNode {
    Literal(String),
    Variable {
        name: String,
        fallback: Option<String>,
        skip_if_empty: bool,
    },
    Conditional {
        name: String,
        content: Vec<TemplateNode>,
    },
}

/// Result of template resolution
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum TemplateResult {
    Path { directory: String, filename: String },
    MissingFields(Vec<MissingField>),
}

/// Path template parser using nom combinators
pub struct PathTemplate;

impl PathTemplate {
    pub fn resolve(template: &str, metadata: &HashMap<String, String>) -> TemplateResult {
        match Self::parse(template) {
            Ok(nodes) => Self::resolve_nodes(&nodes, metadata),
            Err(e) => TemplateResult::MissingFields(vec![MissingField {
                variable: "template".to_string(),
                description: format!("Template parse error: {}", e),
            }]),
        }
    }

    fn parse(input: &str) -> Result<Vec<TemplateNode>, String> {
        match Self::parse_template(input) {
            Ok(("", nodes)) => Ok(nodes),
            Ok((remaining, _)) => Err(format!("Unparsed input: '{}'", remaining)),
            Err(e) => Err(format!("Parse error: {:?}", e)),
        }
    }

    fn parse_template(input: &str) -> IResult<&str, Vec<TemplateNode>> {
        many0(|i| Self::parse_node(i)).parse(input)
    }

    fn parse_node(input: &str) -> IResult<&str, TemplateNode> {
        alt((
            |i| Self::parse_conditional(i),
            |i| Self::parse_variable(i),
            |i| Self::parse_literal(i),
        ))
        .parse(input)
    }

    fn parse_literal(input: &str) -> IResult<&str, TemplateNode> {
        // Match text until we hit a { or end of string
        let (input, text) = take_till(|c| c == '{').parse(input)?;

        // Don't match empty strings - let other parsers handle it
        if text.is_empty() {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::TooLarge,
            )));
        }

        Ok((input, TemplateNode::Literal(text.to_string())))
    }

    fn parse_variable(input: &str) -> IResult<&str, TemplateNode> {
        // Parse: {name}, {name:fallback}, {name/}, {name:fallback/}
        let (input, _) = char('{').parse(input)?;

        // Ensure we're not parsing a conditional {?...}
        if input.starts_with('?') {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Alt,
            )));
        }

        let (input, name) = Self::parse_identifier(input)?;

        // Optional fallback
        let (input, fallback_opt) = opt(|i| {
            let (i, _) = char(':').parse(i)?;
            Self::parse_fallback_value(i)
        })
        .parse(input)?;

        // Optional skip_if_empty flag
        let (input, skip_opt) = opt(|i| char('/').parse(i)).parse(input)?;

        let (input, _) = char('}').parse(input)?;

        Ok((
            input,
            TemplateNode::Variable {
                name: name.to_string(),
                fallback: fallback_opt.map(|s: &str| s.to_string()),
                skip_if_empty: skip_opt.is_some(),
            },
        ))
    }

    fn parse_conditional(input: &str) -> IResult<&str, TemplateNode> {
        // Parse: {?name}...{/name}
        let (input, _) = tag("{?").parse(input)?;
        let (input, name) = Self::parse_identifier(input)?;
        let (input, _) = char('}').parse(input)?;

        // Find closing tag
        let close_tag = format!("{{/{}}}", name);
        let close_pos = input.find(&close_tag).ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag))
        })?;

        let content_str = &input[..close_pos];
        let remaining = &input[close_pos + close_tag.len()..];

        // Parse content inside conditional
        let (_, content) = Self::parse_template(content_str).map_err(|_| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
        })?;

        Ok((
            remaining,
            TemplateNode::Conditional {
                name: name.to_string(),
                content,
            },
        ))
    }

    fn parse_identifier(input: &str) -> IResult<&str, &str> {
        // Match valid identifier chars: alphanumeric and underscore
        take_while1(|c: char| c.is_alphanumeric() || c == '_').parse(input)
    }

    fn parse_fallback_value(input: &str) -> IResult<&str, &str> {
        // Match everything until } or /
        take_till(|c| c == '}' || c == '/').parse(input)
    }
    
    fn resolve_nodes(nodes: &[TemplateNode], metadata: &HashMap<String, String>) -> TemplateResult {
        let mut result = String::new();
        let mut missing = Vec::new();

        for node in nodes {
            if let Some(text) = Self::resolve_node(node, metadata, &mut missing) {
                result.push_str(&text);
            }
        }

        if !missing.is_empty() {
            return TemplateResult::MissingFields(missing);
        }

        let cleaned_path = Self::clean_path(&result);
        let (directory, filename) = Self::split_path(&cleaned_path);

        TemplateResult::Path {
            directory: directory.to_string(),
            filename: filename.to_string(),
        }
    }

    fn resolve_node(
        node: &TemplateNode,
        metadata: &HashMap<String, String>,
        missing: &mut Vec<MissingField>,
    ) -> Option<String> {
        match node {
            TemplateNode::Literal(s) => Some(s.clone()),
            TemplateNode::Variable {
                name,
                fallback,
                skip_if_empty,
            } => match metadata.get(name) {
                Some(val) if !val.is_empty() => Some(if *skip_if_empty {
                    format!("{}/", val)
                } else {
                    val.clone()
                }),
                _ => {
                    if let Some(fb) = fallback {
                        Some(if *skip_if_empty {
                            format!("{}/", fb)
                        } else {
                            fb.clone()
                        })
                    } else if *skip_if_empty {
                        Some(String::new())
                    } else {
                        missing.push(MissingField {
                            variable: name.clone(),
                            description: format!("Missing: {}", name),
                        });
                        None
                    }
                }
            },
            TemplateNode::Conditional { name, content } => {
                if let Some(val) = metadata.get(name) {
                    if !val.is_empty() {
                        let mut result = String::new();
                        for n in content {
                            if let Some(s) = Self::resolve_node(n, metadata, missing) {
                                result.push_str(&s);
                            }
                        }
                        return Some(result);
                    }
                }
                Some(String::new())
            }
        }
    }

    fn clean_path(path: &str) -> String {
        let mut result = String::new();
        let mut last_was_sep = false;

        for ch in path.chars() {
            if ch == '/' || ch == '\\' {
                if !last_was_sep {
                    result.push(ch);
                    last_was_sep = true;
                }
            } else {
                result.push(ch);
                last_was_sep = false;
            }
        }

        result
            .trim_end_matches('/')
            .trim_end_matches('\\')
            .to_string()
    }

    fn split_path(path: &str) -> (&str, &str) {
        // Find the last separator (/ or \)
        let last_sep_pos = path.rfind(|c| c == '/' || c == '\\');

        match last_sep_pos {
            Some(pos) => {
                let directory = &path[..pos];
                let filename = &path[pos + 1..];
                (directory, filename)
            }
            None => {
                // No separator found, the entire path is the filename
                ("", path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_metadata(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn assert_path(template: &str, metadata: &HashMap<String, String>, expected: &str) {
        let result = PathTemplate::resolve(template, metadata);
        let TemplateResult::Path {
            directory,
            filename,
        } = result
        else {
            panic!("Expected a path, got {:?}", result);
        };

        let full_path = if directory.is_empty() {
            filename
        } else {
            format!("{}/{}", directory, filename)
        };
        assert_eq!(full_path, expected);
    }

    #[test]
    fn test_simple_literal() {
        let metadata = create_metadata(&[]);
        assert_path("/home/user/books", &metadata, "/home/user/books");
    }

    #[test]
    fn test_single_variable() {
        let template = "/books/{author}";
        let metadata = create_metadata(&[("author", "Hobb")]);
        assert_path(template, &metadata, "/books/Hobb");
    }

    #[test]
    fn test_multiple_variables() {
        let template = "/books/{author}/{title}.{ext}";
        let metadata = create_metadata(&[
            ("author", "Hobb"),
            ("title", "Assassin's Apprentice"),
            ("ext", "epub"),
        ]);
        assert_path(
            template,
            &metadata,
            "/books/Hobb/Assassin's Apprentice.epub",
        );
    }

    #[test]
    fn test_missing_required_field() {
        let template = "/books/{author}/{title}.{ext}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Assassin's Apprentice")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::MissingFields(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].variable, "ext");
            }
            TemplateResult::Path { .. } => panic!("Should identify missing field"),
        }
    }

    #[test]
    fn test_fallback_value() {
        let template = "/books/{series:_oneshots}/{title}.{ext}";
        let metadata = create_metadata(&[("title", "Book"), ("ext", "epub")]);
        assert_path(template, &metadata, "/books/_oneshots/Book.epub");
    }

    #[test]
    fn test_fallback_with_existing_value() {
        let template = "/books/{series:_oneshots}/{title}.{ext}";
        let metadata = create_metadata(&[
            ("series", "Farseer Trilogy"),
            ("title", "Book"),
            ("ext", "epub"),
        ]);
        assert_path(template, &metadata, "/books/Farseer Trilogy/Book.epub");
    }

    #[test]
    fn test_skip_if_empty_present() {
        let template = "/books/{language/}{author}/{title}.{ext}";
        let metadata = create_metadata(&[
            ("language", "en"),
            ("author", "Hobb"),
            ("title", "Book"),
            ("ext", "epub"),
        ]);
        assert_path(template, &metadata, "/books/en/Hobb/Book.epub");
    }

    #[test]
    fn test_skip_if_empty_missing() {
        let template = "/books/{language/}{author}/{title}.{ext}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book"), ("ext", "epub")]);
        assert_path(template, &metadata, "/books/Hobb/Book.epub");
    }

    #[test]
    fn test_conditional_block_present() {
        let template =
            "/books/{author}/{?series}{series} - {series_number} - {/series}{title}.{ext}";
        let metadata = create_metadata(&[
            ("author", "Hobb"),
            ("series", "Farseer Trilogy"),
            ("series_number", "1"),
            ("title", "Assassin's Apprentice"),
            ("ext", "epub"),
        ]);
        assert_path(
            template,
            &metadata,
            "/books/Hobb/Farseer Trilogy - 1 - Assassin's Apprentice.epub",
        );
    }

    #[test]
    fn test_conditional_block_missing() {
        let template =
            "/books/{author}/{?series}{series} - {series_number} - {/series}{title}.{ext}";
        let metadata = create_metadata(&[
            ("author", "Hobb"),
            ("title", "Standalone Book"),
            ("ext", "epub"),
        ]);
        assert_path(template, &metadata, "/books/Hobb/Standalone Book.epub");
    }

    #[test]
    fn test_clean_path_double_slashes() {
        let template = "/books//{author}//{title}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book")]);
        assert_path(template, &metadata, "/books/Hobb/Book");
    }

    #[test]
    fn test_clean_path_trailing_slash() {
        let template = "/books/{author}/{title}/";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book")]);
        assert_path(template, &metadata, "/books/Hobb/Book");
    }

    #[test]
    fn test_complex_real_world_example() {
        let template = "/home/okno/Ebooks/{language}/{author}/{series:_oneshots}/{?series}{series} - {series_number} - {/series}{title}.{ext}";
        let metadata = create_metadata(&[
            ("language", "fr"),
            ("author", "Hobb"),
            ("series", "Farseer Trilogy"),
            ("series_number", "1"),
            ("title", "Assassin's Apprentice"),
            ("ext", "epub"),
        ]);
        assert_path(
            template,
            &metadata,
            "/home/okno/Ebooks/fr/Hobb/Farseer Trilogy/Farseer Trilogy - 1 - Assassin's Apprentice.epub",
        );
    }

    #[test]
    fn test_complex_standalone_book() {
        let template = "/home/okno/Ebooks/{language}/{author}/{series:_oneshots}/{?series}{series} - {series_number} - {/series}{title}.{ext}";
        let metadata = create_metadata(&[
            ("language", "en"),
            ("author", "Author"),
            ("title", "Standalone Book"),
            ("ext", "epub"),
        ]);
        assert_path(
            template,
            &metadata,
            "/home/okno/Ebooks/en/Author/_oneshots/Standalone Book.epub",
        );
    }

    #[test]
    fn test_empty_string_treated_as_missing() {
        let template = "/books/{author}/{title}.{ext}";
        let metadata = create_metadata(&[("author", ""), ("title", "Book"), ("ext", "epub")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::MissingFields(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].variable, "author");
            }
            TemplateResult::Path { .. } => panic!("Empty string should be treated as missing"),
        }
    }

    #[test]
    fn test_whitespace_in_values() {
        let template = "/books/{author}/{title}.{ext}";
        let metadata = create_metadata(&[
            ("author", "Hobb"),
            ("title", "Assassin's Apprentice"),
            ("ext", "epub"),
        ]);
        assert_path(
            template,
            &metadata,
            "/books/Hobb/Assassin's Apprentice.epub",
        );
    }

    #[test]
    fn test_multiple_missing_fields() {
        let template = "/books/{language}/{author}/{title}.{ext}";
        let metadata = create_metadata(&[("author", "Hobb")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::MissingFields(fields) => {
                assert_eq!(fields.len(), 3);
                let vars: Vec<&str> = fields.iter().map(|f| f.variable.as_str()).collect();
                assert!(vars.contains(&"language"));
                assert!(vars.contains(&"title"));
                assert!(vars.contains(&"ext"));
            }
            TemplateResult::Path { .. } => panic!("Should identify all missing fields"),
        }
    }

    #[test]
    fn test_parse_returns_ast() {
        let template = "/books/{author}/{title}.{ext}";

        match PathTemplate::parse(template) {
            Ok(nodes) => {
                assert!(!nodes.is_empty());
                assert!(matches!(nodes[0], TemplateNode::Literal(_)));
            }
            Err(e) => panic!("Parse should succeed: {}", e),
        }
    }

    #[test]
    fn test_conditional_parse() {
        let template = "{?series}{series} - {series_number}{/series}";

        match PathTemplate::parse(template) {
            Ok(nodes) => {
                assert_eq!(nodes.len(), 1);
                assert!(matches!(nodes[0], TemplateNode::Conditional { .. }));
            }
            Err(e) => panic!("Parse should succeed: {}", e),
        }
    }

    #[test]
    fn test_consecutive_literals() {
        let template = "prefix_{author}_suffix";
        let metadata = create_metadata(&[("author", "Hobb")]);
        assert_path(template, &metadata, "prefix_Hobb_suffix");
    }

    #[test]
    fn test_variable_at_start() {
        let template = "{author}/books/{title}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book")]);
        assert_path(template, &metadata, "Hobb/books/Book");
    }

    #[test]
    fn test_variable_at_end() {
        let template = "/books/{author}";
        let metadata = create_metadata(&[("author", "Hobb")]);
        assert_path(template, &metadata, "/books/Hobb");
    }

    #[test]
    fn test_consecutive_variables() {
        let template = "{language}-{author}";
        let metadata = create_metadata(&[("language", "en"), ("author", "Hobb")]);
        assert_path(template, &metadata, "en-Hobb");
    }

    #[test]
    fn test_special_characters_in_values() {
        let template = "/books/{author}/{title}";
        let metadata = create_metadata(&[
            ("author", "Hobb & Associates"),
            ("title", "Book (Special Edition)"),
        ]);
        assert_path(
            template,
            &metadata,
            "/books/Hobb & Associates/Book (Special Edition)",
        );
    }

    #[test]
    fn test_numeric_values() {
        let template = "/series/{series}/{series_number}";
        let metadata = create_metadata(&[("series", "Books"), ("series_number", "42")]);
        assert_path(template, &metadata, "/series/Books/42");
    }

    #[test]
    fn test_nested_skip_if_empty_flags() {
        let template = "/books/{language/}{series/}{title}";
        let metadata = create_metadata(&[("title", "Book")]);
        assert_path(template, &metadata, "/books/Book");
    }

    #[test]
    fn test_skip_if_empty_with_fallback() {
        let template = "/books/{language:en/}{author}";
        let metadata = create_metadata(&[("author", "Hobb")]);
        assert_path(template, &metadata, "/books/en/Hobb");
    }

    #[test]
    fn test_only_variables() {
        let template = "{author}/{title}/{ext}";
        let metadata = create_metadata(&[("author", "A"), ("title", "B"), ("ext", "c")]);
        assert_path(template, &metadata, "A/B/c");
    }

    #[test]
    fn test_conditional_with_multiple_variables_inside() {
        let template = "{?series}[{series} - {series_number}]{/series}";
        let metadata = create_metadata(&[("series", "Trilogy"), ("series_number", "1")]);
        assert_path(template, &metadata, "[Trilogy - 1]");
    }

    #[test]
    fn test_conditional_missing_one_inner_variable() {
        let template = "{?series}{series} - {series_number}{/series}";
        let metadata = create_metadata(&[("series", "Trilogy")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::MissingFields(fields) => {
                assert!(fields.iter().any(|f| f.variable == "series_number"));
            }
            TemplateResult::Path {
                directory,
                filename,
            } => {
                panic!(
                    "Should have missing series_number: directory={}, filename={}",
                    directory, filename
                );
            }
        }
    }

    #[test]
    fn test_backslash_separator() {
        let template = "C:\\Users\\{author}\\Books\\{title}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::Path {
                directory,
                filename,
            } => {
                let full_path = format!("{}\\{}", directory, filename);
                assert!(full_path.contains("Hobb"));
                assert!(full_path.contains("Book"));
            }
            TemplateResult::MissingFields(_) => panic!("Should handle backslashes"),
        }
    }

    #[test]
    fn test_fallback_with_special_chars() {
        let template = "/books/{series:No Series}/{title}";
        let metadata = create_metadata(&[("title", "Book")]);
        assert_path(template, &metadata, "/books/No Series/Book");
    }

    #[test]
    fn test_case_sensitive_variables() {
        let template = "/books/{Author}/{Title}";
        let metadata = create_metadata(&[("author", "Hobb"), ("title", "Book")]);

        match PathTemplate::resolve(template, &metadata) {
            TemplateResult::MissingFields(fields) => {
                assert_eq!(fields.len(), 2);
                let vars: Vec<&str> = fields.iter().map(|f| f.variable.as_str()).collect();
                assert!(vars.contains(&"Author"));
                assert!(vars.contains(&"Title"));
            }
            TemplateResult::Path { .. } => panic!("Should be case-sensitive"),
        }
    }
}
