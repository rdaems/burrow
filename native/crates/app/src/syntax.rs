//! What the code in a file is, for the editor's colours (index.html's `editorColours`, which CodeMirror's parsers
//! feed): no parser here, a line at a time by the file's kind: comments, strings, numbers, keywords, a function's
//! name before its "(", a Type, a markdown heading, link or `code`. Block comments are followed across lines.

use std::ops::Range;

/// what a stretch of a line is
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Plain,
    Keyword,
    Str,
    Num,
    Comment,
    Func,
    Type,
    Heading,
    Link,
    Meta,
}

impl Tok {
    /// the room's palette for it (editorColours), and whether it is bold or italic
    pub fn look(self) -> (u32, bool, bool) {
        match self {
            Tok::Plain => (0xe4e7f0, false, false),
            Tok::Keyword => (0xe88aa8, false, false),
            Tok::Str => (0x7fdcb0, false, false),
            Tok::Num => (0xffe27a, false, false),
            Tok::Comment => (0x6c7590, false, true),
            Tok::Func => (0x8fc0e8, false, false),
            Tok::Type => (0xf2a531, false, false),
            Tok::Heading => (0xf6c784, true, false),
            Tok::Link => (0x8fc0e8, false, false),
            Tok::Meta => (0x8b93ad, false, false),
        }
    }
}

/// a kind of file: its comments, strings, keywords
#[derive(Clone, Debug, PartialEq)]
pub struct Lang {
    line: &'static [&'static str],
    block: Option<(&'static str, &'static str)>,
    quotes: &'static [char],
    keywords: &'static [&'static str],
    md: bool,
}

const C_LIKE: &[&str] = &[
    "if", "else", "for", "while", "do", "return", "break", "continue", "switch", "case", "default", "function", "const", "let", "var", "class",
    "new", "this", "import", "export", "from", "as", "async", "await", "try", "catch", "finally", "throw", "typeof", "instanceof", "in", "of",
    "true", "false", "null", "undefined", "void", "delete", "yield", "extends", "static", "public", "private", "protected", "interface",
    "type", "enum", "struct", "fn", "pub", "use", "mod", "impl", "trait", "match", "mut", "ref", "where", "loop", "self", "Self", "super",
    "crate", "dyn", "move", "unsafe", "extern", "int", "char", "float", "double", "long", "short", "unsigned", "bool", "func", "package",
    "go", "defer", "chan", "map", "range", "nil", "var", "val", "fun", "when", "object", "override", "sizeof", "include", "define",
];
const PY: &[&str] = &[
    "def", "class", "if", "elif", "else", "for", "while", "return", "import", "from", "as", "with", "try", "except", "finally", "raise", "lambda",
    "yield", "pass", "break", "continue", "in", "is", "not", "and", "or", "None", "True", "False", "global", "nonlocal", "assert", "del",
    "async", "await", "self",
];
const SH: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "in", "function", "return", "export", "local",
    "set", "unset", "echo", "exit", "source", "cd",
];
const RB: &[&str] = &["def", "end", "class", "module", "if", "elsif", "else", "unless", "do", "while", "return", "require", "nil", "true", "false", "self", "yield", "begin", "rescue"];
const SQL: &[&str] = &["select", "from", "where", "insert", "into", "update", "delete", "create", "table", "join", "on", "and", "or", "not", "null", "group", "by", "order", "values", "set", "as"];
const NONE: &[&str] = &[];

/// the kind of a file by its name (CodeMirror's `language(t.rel)`); None: plain text
pub fn lang_of(path: &str) -> Option<Lang> {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or(if name == "makefile" || name == "dockerfile" { "sh" } else { "" });
    let l = |line, block, quotes, keywords| Some(Lang { line, block, quotes, keywords, md: false });
    match ext {
        "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "rs" | "go" | "c" | "h" | "cc" | "cpp" | "hpp" | "java" | "kt" | "swift" | "scala" | "dart" | "cs" => {
            l(&["//"], Some(("/*", "*/")), &['"', '\'', '`'], C_LIKE)
        }
        "css" | "scss" | "less" => l(&[], Some(("/*", "*/")), &['"', '\''], NONE),
        "json" | "jsonc" => l(&["//"], None, &['"'], &["true", "false", "null"]),
        "py" | "pyi" => l(&["#"], None, &['"', '\''], PY),
        "sh" | "bash" | "zsh" | "fish" | "toml" | "yaml" | "yml" | "ini" | "conf" | "env" | "r" => l(&["#"], None, &['"', '\''], SH),
        "rb" => l(&["#"], None, &['"', '\''], RB),
        "sql" => l(&["--"], Some(("/*", "*/")), &['\''], SQL),
        "lua" | "hs" => l(&["--"], None, &['"', '\''], NONE),
        "html" | "htm" | "xml" | "svg" | "vue" => l(&[], Some(("<!--", "-->")), &['"', '\''], NONE),
        "md" | "markdown" => Some(Lang { line: &[], block: None, quotes: &[], keywords: NONE, md: true }),
        _ => None,
    }
}

/// for each line, whether it starts inside a block comment (or a fenced code block, in markdown)
pub fn block_state(text: &str, lang: &Lang) -> Vec<bool> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.split('\n') {
        out.push(inside);
        if lang.md {
            if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
                inside = !inside;
            }
            continue;
        }
        let Some((open, close)) = lang.block else { continue };
        let mut i = 0;
        while i < line.len() {
            if inside {
                match line[i..].find(close) {
                    Some(j) => {
                        inside = false;
                        i += j + close.len();
                    }
                    None => break,
                }
            } else {
                // (a block opener inside a string or after a line comment does not count)
                let rest = &line[i..];
                let c = rest.chars().next().unwrap();
                if lang.line.iter().any(|p| rest.starts_with(p)) {
                    break;
                } else if lang.quotes.contains(&c) {
                    i += c.len_utf8() + rest[c.len_utf8()..].find(c).map(|j| j + c.len_utf8()).unwrap_or(rest.len() - c.len_utf8());
                } else if rest.starts_with(open) {
                    inside = true;
                    i += open.len();
                } else {
                    i += c.len_utf8();
                }
            }
        }
    }
    out
}

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// the stretches of one line that are not plain, in order (byte ranges)
pub fn spans(line: &str, lang: &Lang, in_block: bool) -> Vec<(Range<usize>, Tok)> {
    let mut out = Vec::new();
    if lang.md {
        return md_spans(line, in_block);
    }
    let mut i = 0;
    if in_block && let Some((_, close)) = lang.block {
        match line.find(close) {
            Some(j) => {
                out.push((0..j + close.len(), Tok::Comment));
                i = j + close.len();
            }
            None => return vec![(0..line.len(), Tok::Comment)],
        }
    }
    while i < line.len() {
        let rest = &line[i..];
        let c = rest.chars().next().unwrap();
        if lang.line.iter().any(|p| rest.starts_with(p)) {
            out.push((i..line.len(), Tok::Comment));
            break;
        }
        if let Some((open, close)) = lang.block
            && rest.starts_with(open)
        {
            let end = rest[open.len()..].find(close).map(|j| i + open.len() + j + close.len()).unwrap_or(line.len());
            out.push((i..end, Tok::Comment));
            i = end;
            continue;
        }
        if lang.quotes.contains(&c) {
            // to the closing quote, a backslash escaping the next
            let mut j = i + 1;
            let b = line.as_bytes();
            while j < line.len() && b[j] != c as u8 {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(line.len());
            let end = (end..=line.len()).find(|&k| line.is_char_boundary(k)).unwrap_or(line.len());
            out.push((i..end, Tok::Str));
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            let end = i + rest.find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '_')).unwrap_or(rest.len());
            out.push((i..end, Tok::Num));
            i = end;
            continue;
        }
        if word_char(c) {
            let end = i + rest.find(|ch: char| !word_char(ch)).unwrap_or(rest.len());
            let w = &line[i..end];
            // (SQL's keywords in any case)
            let sql = std::ptr::eq(lang.keywords, SQL);
            let tok = if lang.keywords.iter().any(|k| if sql { k.eq_ignore_ascii_case(w) } else { *k == w }) {
                Tok::Keyword
            } else if line[end..].starts_with('(') || line[end..].starts_with("!(") {
                Tok::Func
            } else if w.chars().next().is_some_and(|ch| ch.is_uppercase()) && w.chars().any(|ch| ch.is_lowercase()) {
                Tok::Type
            } else {
                Tok::Plain
            };
            if tok != Tok::Plain {
                out.push((i..end, tok));
            }
            i = end;
            continue;
        }
        i += c.len_utf8();
    }
    out
}

/// markdown's: headings, `code`, [links](…), fences and what is in them
fn md_spans(line: &str, in_fence: bool) -> Vec<(Range<usize>, Tok)> {
    let t = line.trim_start();
    if in_fence || t.starts_with("```") || t.starts_with("~~~") {
        return vec![(0..line.len(), Tok::Meta)];
    }
    if t.starts_with('#') && t.trim_start_matches('#').starts_with(' ') {
        return vec![(0..line.len(), Tok::Heading)];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];
        if rest.starts_with('`')
            && let Some(j) = rest[1..].find('`')
        {
            out.push((i..i + j + 2, Tok::Str));
            i += j + 2;
            continue;
        }
        if (rest.starts_with('[') || rest.starts_with("!["))
            && let Some(j) = rest.find("](")
            && let Some(k) = rest[j..].find(')')
        {
            out.push((i..i + j + k + 1, Tok::Link));
            i += j + k + 1;
            continue;
        }
        i += rest.chars().next().unwrap().len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(line: &str, path: &str) -> Vec<(&'static str, Tok)> {
        let l = lang_of(path).unwrap();
        spans(line, &l, false).into_iter().map(|(r, t)| (&*Box::leak(line[r].to_string().into_boxed_str()), t)).collect()
    }

    #[test]
    fn code_by_its_kind() {
        assert_eq!(
            toks("const x = fly(\"up\", 42); // go", "a.js"),
            [("const", Tok::Keyword), ("fly", Tok::Func), ("\"up\"", Tok::Str), ("42", Tok::Num), ("// go", Tok::Comment)]
        );
        assert_eq!(toks("def run(self): # x", "a.py"), [("def", Tok::Keyword), ("run", Tok::Func), ("self", Tok::Keyword), ("# x", Tok::Comment)]);
        assert_eq!(toks("let s: String = \"a\\\"b\";", "m.rs"), [("let", Tok::Keyword), ("String", Tok::Type), ("\"a\\\"b\"", Tok::Str)]);
        assert!(lang_of("notes.txt").is_none());
    }

    #[test]
    fn block_comments_across_lines() {
        let l = lang_of("a.rs").unwrap();
        let text = "a /* one\ntwo\nthree */ b\n\"/*\" c";
        assert_eq!(block_state(text, &l), [false, true, true, false]);
        assert_eq!(spans("three */ b", &l, true), [(0..8, Tok::Comment)]);
    }

    #[test]
    fn markdown() {
        let l = lang_of("README.md").unwrap();
        assert_eq!(spans("## Title", &l, false), [(0..8, Tok::Heading)]);
        assert_eq!(spans("see `x` and [a](b.md)", &l, false), [(4..7, Tok::Str), (12..21, Tok::Link)]);
        assert_eq!(block_state("```\ncode\n```\nafter", &l), [false, true, true, false]);
    }
}
