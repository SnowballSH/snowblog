use typst_syntax::{SyntaxKind, SyntaxNode, ast};

use super::ImportError;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frontmatter {
    pub title: String,
    pub description: Option<String>,
    pub date: Option<String>,
    pub tags: Vec<String>,
    pub draft: bool,
    pub hidden: bool,
    pub chinese_source: Option<String>,
}

pub fn parse_frontmatter(source: &str) -> Result<(Frontmatter, String), ImportError> {
    let root = typst_syntax::parse(source);
    let (dict_node, span_range) =
        find_metadata_block(&root, source).ok_or(ImportError::MissingFrontmatter)?;
    let frontmatter = read_dict(dict_node)?;
    if frontmatter.title.is_empty() {
        return Err(ImportError::InvalidFrontmatter("missing title".into()));
    }
    let mut remaining = String::with_capacity(source.len());
    remaining.push_str(&source[..span_range.0]);
    remaining.push_str(&source[span_range.1..]);
    Ok((frontmatter, remaining.trim_start().to_string()))
}

fn find_metadata_block<'a>(
    root: &'a SyntaxNode,
    source: &str,
) -> Option<(&'a SyntaxNode, (usize, usize))> {
    let mut offset = 0;
    let mut call: Option<(&SyntaxNode, usize, usize)> = None;
    for child in root.children() {
        let len = child.len();
        let start = offset;
        offset += len;
        match child.kind() {
            SyntaxKind::FuncCall if is_metadata_call(child) => {
                call = Some((child, start, offset));
            }
            SyntaxKind::Label if call.is_some() => {
                let text = &source[start..offset];
                if text == "<frontmatter>" {
                    let (node, call_start, _) = call.unwrap();
                    let dict = find_dict(node)?;
                    return Some((dict, (call_start.saturating_sub(1), offset)));
                }
                call = None;
            }
            SyntaxKind::Space | SyntaxKind::Parbreak => {}
            _ => call = None,
        }
    }
    None
}

fn is_metadata_call(node: &SyntaxNode) -> bool {
    node.children().next().is_some_and(|callee| {
        callee.kind() == SyntaxKind::Ident && callee.leaf_text() == "metadata"
    })
}

fn find_dict(node: &SyntaxNode) -> Option<&SyntaxNode> {
    if node.kind() == SyntaxKind::Dict {
        return Some(node);
    }
    node.children().find_map(find_dict)
}

fn read_dict(dict_node: &SyntaxNode) -> Result<Frontmatter, ImportError> {
    let dict: ast::Dict = dict_node
        .cast()
        .ok_or_else(|| ImportError::InvalidFrontmatter("metadata is not a dictionary".into()))?;
    let mut frontmatter = Frontmatter::default();
    for item in dict.items() {
        let ast::DictItem::Named(named) = item else {
            continue;
        };
        let key = named.name().as_str().to_string();
        match key.as_str() {
            "title" => frontmatter.title = expect_str(named.expr(), &key)?,
            "description" => frontmatter.description = Some(expect_str(named.expr(), &key)?),
            "date" => frontmatter.date = Some(expect_str(named.expr(), &key)?),
            "chineseSource" => frontmatter.chinese_source = Some(expect_str(named.expr(), &key)?),
            "draft" => frontmatter.draft = expect_bool(named.expr(), &key)?,
            "hidden" => frontmatter.hidden = expect_bool(named.expr(), &key)?,
            "tags" => frontmatter.tags = expect_str_array(named.expr(), &key)?,
            _ => {}
        }
    }
    Ok(frontmatter)
}

fn expect_str(expr: ast::Expr, key: &str) -> Result<String, ImportError> {
    match expr {
        ast::Expr::Str(value) => Ok(value.get().to_string()),
        _ => Err(ImportError::InvalidFrontmatter(format!(
            "{key} must be a string literal"
        ))),
    }
}

fn expect_bool(expr: ast::Expr, key: &str) -> Result<bool, ImportError> {
    match expr {
        ast::Expr::Bool(value) => Ok(value.get()),
        _ => Err(ImportError::InvalidFrontmatter(format!(
            "{key} must be a boolean literal"
        ))),
    }
}

fn expect_str_array(expr: ast::Expr, key: &str) -> Result<Vec<String>, ImportError> {
    let ast::Expr::Array(array) = expr else {
        return Err(ImportError::InvalidFrontmatter(format!(
            "{key} must be an array of strings"
        )));
    };
    array
        .items()
        .map(|item| match item {
            ast::ArrayItem::Pos(ast::Expr::Str(value)) => Ok(value.get().to_string()),
            _ => Err(ImportError::InvalidFrontmatter(format!(
                "{key} must contain only string literals"
            ))),
        })
        .collect()
}
