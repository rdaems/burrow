//! A markdown file rendered (index.html's marked, `renderFile`), natively: headings, paragraphs, **bold**,
//! *emphasis*, ~~struck~~, `code`, links (the web's in your browser, another file of it in the tile), lists in lists,
//! quotes, fenced and indented code, rules, tables, pictures (from the machine the file is on). What marked would
//! pass through as HTML is not run, nor shown as a page: its tags go, its text stays, an <img> is a picture and a
//! <br> a break. Nothing in a file can run here; there is no page to run it in.
//!   The parser is CommonMark's common cases, not all of CommonMark (no nested emphasis rules, no HTML blocks as
//! such, no footnotes); the drawing is burrow.css's `.fview`.

use crate::kit::{self, col};
use gpui::{
    AnyElement, App, FontStyle, FontWeight, HighlightStyle, InteractiveText, IntoElement, ParentElement, StrikethroughStyle, StyledText, Styled, UnderlineStyle,
    Window, div, img, prelude::*, px, rgb,
};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// a stretch of text, as it is shown
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub em: bool,
    pub code: bool,
    pub strike: bool,
    pub link: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Span(Span),
    Image { alt: String, src: String, link: Option<String> },
    Break,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading(u8, Vec<Inline>),
    Para(Vec<Inline>),
    Code(String),
    Quote(Vec<Block>),
    /// ordered from its first number, or bullets
    List(Option<u64>, Vec<Vec<Block>>),
    Rule,
    Table(Vec<Vec<Inline>>, Vec<Vec<Vec<Inline>>>),
}

type Refs = HashMap<String, String>;

fn indent(l: &str) -> usize {
    let mut n = 0;
    for c in l.chars() {
        match c {
            ' ' => n += 1,
            '\t' => n += 4 - n % 4,
            _ => break,
        }
    }
    n
}
/// a line less `n` columns of its indentation
fn dedent(l: &str, n: usize) -> &str {
    let mut c = 0;
    for (i, ch) in l.char_indices() {
        if c >= n || !(ch == ' ' || ch == '\t') {
            return &l[i..];
        }
        c += if ch == '\t' { 4 - c % 4 } else { 1 };
    }
    ""
}
fn blank(l: &str) -> bool {
    l.trim().is_empty()
}
fn fence(l: &str) -> Option<(char, usize)> {
    if indent(l) > 3 {
        return None;
    }
    let t = l.trim_start();
    let c = t.chars().next()?;
    let n = t.chars().take_while(|x| *x == c).count();
    ((c == '`' || c == '~') && n >= 3 && !(c == '`' && t[n..].contains('`'))).then_some((c, n))
}
fn heading(l: &str) -> Option<(u8, &str)> {
    if indent(l) > 3 {
        return None;
    }
    let t = l.trim_start();
    let n = t.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&n) || !(t.len() == n || t[n..].starts_with([' ', '\t'])) {
        return None;
    }
    let s = t[n..].trim();
    // closing #s go
    let s = s.trim_end_matches('#');
    let s = if s.ends_with(' ') || s.is_empty() { s.trim_end() } else { t[n..].trim() };
    Some((n as u8, s))
}
fn rule(l: &str) -> bool {
    let t: String = l.chars().filter(|c| !c.is_whitespace()).collect();
    indent(l) < 4 && t.len() >= 3 && ["-", "*", "_"].iter().any(|c| t.chars().all(|x| x.to_string() == *c))
}
/// a list item's marker: ordered (its number), the marker's width with the spaces after it
fn marker(l: &str) -> Option<(Option<u64>, usize)> {
    let i = indent(l);
    let t = dedent(l, i);
    let (num, w) = if t.starts_with(['-', '*', '+']) {
        (None, 1)
    } else {
        let d = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if d == 0 || d > 9 || !t[d..].starts_with(['.', ')']) {
            return None;
        }
        (t[..d].parse().ok(), d + 1)
    };
    let rest = &t[w..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) || rule(l) && num.is_none() {
        return None;
    }
    let sp = (rest.len() - rest.trim_start().len()).clamp(1, 4);
    let sp = if rest.trim().is_empty() { 1 } else { sp };
    Some((num, i + w + sp))
}
/// an item's first line without its marker
fn item_text(l: &str) -> &str {
    let t = l.trim_start();
    let m = if t.starts_with(['-', '*', '+']) { 1 } else { t.chars().take_while(|c| c.is_ascii_digit()).count() + 1 };
    let rest = &t[m.min(t.len())..];
    let sp = rest.len() - rest.trim_start_matches([' ', '\t']).len();
    if sp > 4 { &rest[1..] } else { &rest[sp..] }
}
fn table_sep(l: &str) -> bool {
    let t = l.trim().trim_matches('|');
    !t.is_empty() && t.contains('-') && t.split('|').all(|c| {
        let c = c.trim();
        !c.is_empty() && c.trim_matches(':').chars().all(|x| x == '-')
    })
}
fn cells(l: &str) -> Vec<String> {
    let t = l.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    let mut out = vec![String::new()];
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\\' if it.peek() == Some(&'|') => out.last_mut().unwrap().push(it.next().unwrap()),
            '|' => out.push(String::new()),
            c => out.last_mut().unwrap().push(c),
        }
    }
    out.into_iter().map(|c| c.trim().to_string()).collect()
}
/// does this line start a block of its own (and so end a paragraph)
fn interrupts(l: &str) -> bool {
    // (a list interrupts only with a bullet, or with 1., and only with something in its item)
    heading(l).is_some() || fence(l).is_some() || rule(l) || l.trim_start().starts_with('>') || (indent(l) < 4 && marker(l).is_some_and(|(n, _)| n.is_none_or(|n| n == 1)) && !blank(item_text(l)))
}

/// a markdown file's blocks
pub fn parse(text: &str) -> Vec<Block> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<&str> = text.split('\n').collect();
    // the links' definitions, from anywhere: [name]: url
    let mut refs = Refs::new();
    for l in &lines {
        let t = l.trim();
        if let Some(rest) = t.strip_prefix('[')
            && let Some((name, url)) = rest.split_once("]:")
            && !name.is_empty()
        {
            let url = url.split_whitespace().next().unwrap_or("").trim_matches(['<', '>']);
            refs.insert(name.to_lowercase(), url.to_string());
        }
    }
    blocks(&lines, &refs)
}

fn blocks(lines: &[&str], refs: &Refs) -> Vec<Block> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if blank(l) {
            i += 1;
            continue;
        }
        if let Some((c, n)) = fence(l) {
            let ind = indent(l);
            let mut code = Vec::new();
            i += 1;
            while i < lines.len() {
                if fence(lines[i]).is_some_and(|(c2, n2)| c2 == c && n2 >= n) && lines[i].trim().chars().all(|x| x == c) {
                    i += 1;
                    break;
                }
                code.push(dedent(lines[i], ind));
                i += 1;
            }
            out.push(Block::Code(code.join("\n")));
            continue;
        }
        if indent(l) >= 4 {
            let mut code = Vec::new();
            while i < lines.len() && (indent(lines[i]) >= 4 || blank(lines[i])) {
                code.push(dedent(lines[i], 4));
                i += 1;
            }
            while code.last().is_some_and(|c| blank(c)) {
                code.pop();
            }
            out.push(Block::Code(code.join("\n")));
            continue;
        }
        if let Some((n, s)) = heading(l) {
            out.push(Block::Heading(n, inline(s, refs)));
            i += 1;
            continue;
        }
        if rule(l) {
            out.push(Block::Rule);
            i += 1;
            continue;
        }
        if l.trim_start().starts_with('>') {
            let mut inner = Vec::new();
            while i < lines.len() && !blank(lines[i]) {
                let t = lines[i].trim_start();
                match t.strip_prefix('>') {
                    Some(r) => inner.push(r.strip_prefix(' ').unwrap_or(r)),
                    None if !interrupts(lines[i]) => inner.push(lines[i]),
                    None => break,
                }
                i += 1;
            }
            out.push(Block::Quote(blocks(&inner, refs)));
            continue;
        }
        if let Some((num, w0)) = marker(l) {
            let ordered = num.is_some();
            let mut items: Vec<Vec<&str>> = Vec::new();
            let mut w = w0;
            // another item of this list: a marker of its kind, not indented under the item before
            let item = |li: &str, w: usize| marker(li).filter(|(n2, _)| n2.is_some() == ordered && indent(li) < w);
            while i < lines.len() {
                let li = lines[i];
                if let Some((_, w2)) = item(li, w) {
                    w = w2;
                    items.push(vec![item_text(li)]);
                    i += 1;
                    continue;
                }
                if blank(li) {
                    // a blank line: the list goes on if what follows is indented under the item, or is another item
                    match lines[i + 1..].iter().find(|x| !blank(x)) {
                        Some(nx) if indent(nx) >= w || item(nx, w).is_some() => {
                            items.last_mut().unwrap().push("");
                            i += 1;
                            continue;
                        }
                        _ => break,
                    }
                }
                if indent(li) >= w {
                    items.last_mut().unwrap().push(dedent(li, w));
                } else if !interrupts(li) && items.last().is_some_and(|it| it.last().is_some_and(|x| !blank(x))) {
                    items.last_mut().unwrap().push(li.trim_start()); // a lazy line of the paragraph
                } else {
                    break;
                }
                i += 1;
            }
            out.push(Block::List(num, items.iter().map(|it| blocks(it, refs)).collect()));
            continue;
        }
        if l.contains('|') && i + 1 < lines.len() && table_sep(lines[i + 1]) {
            let head = cells(l).iter().map(|c| inline(c, refs)).collect();
            i += 2;
            let mut rows = Vec::new();
            while i < lines.len() && !blank(lines[i]) && lines[i].contains('|') {
                rows.push(cells(lines[i]).iter().map(|c| inline(c, refs)).collect());
                i += 1;
            }
            out.push(Block::Table(head, rows));
            continue;
        }
        // a link's definition is not shown
        let t = l.trim();
        if t.starts_with('[') && t.split_once("]:").is_some_and(|(n, _)| !n[1..].is_empty() && !n.contains(']')) {
            i += 1;
            continue;
        }
        // a paragraph, to a blank line or a block of another kind; underlined with = or -, a heading
        let mut para = vec![l];
        i += 1;
        while i < lines.len() && !blank(lines[i]) {
            let t = lines[i].trim();
            if !t.is_empty() && (t.chars().all(|c| c == '=') || (t.chars().all(|c| c == '-') && para.len() >= 1)) && indent(lines[i]) < 4 {
                let n = if t.starts_with('=') { 1 } else { 2 };
                out.push(Block::Heading(n, inline(para.join("\n").trim(), refs)));
                para.clear();
                i += 1;
                break;
            }
            if interrupts(lines[i]) {
                break;
            }
            para.push(lines[i]);
            i += 1;
        }
        if !para.is_empty() {
            out.push(Block::Para(inline(para.iter().map(|x| x.trim_start()).collect::<Vec<_>>().join("\n").trim_end(), refs)));
        }
    }
    out
}

/// the closing `]` of a `[` at `i` (brackets nest; a backslash escapes)
fn close_bracket(s: &str, i: usize) -> Option<usize> {
    let mut depth = 0;
    let b = s.as_bytes();
    let mut j = i;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            b'`' => {
                // a code span inside: skip it
                let n = s[j..].chars().take_while(|c| *c == '`').count();
                if let Some(k) = s[j + n..].find(&"`".repeat(n)) {
                    j += n + k + n - 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    None
}

/// a link's destination after `](`, to its `)`: the url (a "title" after it dropped), and where it ends
fn destination(s: &str, i: usize) -> Option<(String, usize)> {
    let rest = &s[i..];
    if let Some(r) = rest.strip_prefix('<') {
        let k = r.find('>')?;
        let end = r[k..].find(')')? + k;
        return Some((r[..k].to_string(), i + 1 + end + 1));
    }
    let mut depth = 0;
    for (k, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' if depth == 0 => {
                let url = rest[..k].trim().split_whitespace().next().unwrap_or("").to_string();
                return Some((url, i + k + 1));
            }
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// an html tag's attribute
fn attr(tag: &str, name: &str) -> Option<String> {
    let low = tag.to_lowercase();
    let k = low.find(&format!("{name}="))? + name.len() + 1;
    let v = &tag[k..];
    let q = v.chars().next()?;
    if q == '"' || q == '\'' {
        Some(v[1..].split(q).next()?.to_string())
    } else {
        Some(v.split([' ', '>', '/']).next()?.to_string())
    }
}

#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    em: bool,
    strike: bool,
}

/// a paragraph's (or a heading's, a cell's) text: what it says and how
pub fn inline(s: &str, refs: &Refs) -> Vec<Inline> {
    let mut out = Vec::new();
    run(s, refs, None, &mut out);
    out
}

fn push(out: &mut Vec<Inline>, text: &str, st: Style, code: bool, link: &Option<String>) {
    if text.is_empty() {
        return;
    }
    if let Some(Inline::Span(last)) = out.last_mut()
        && last.bold == st.bold
        && last.em == st.em
        && last.strike == st.strike
        && last.code == code
        && last.link == *link
    {
        last.text.push_str(text);
        return;
    }
    out.push(Inline::Span(Span { text: text.to_string(), bold: st.bold, em: st.em, code, strike: st.strike, link: link.clone() }));
}

fn run(s: &str, refs: &Refs, link: Option<String>, out: &mut Vec<Inline>) {
    let mut st = Style::default();
    let mut text = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    let flush = |text: &mut String, out: &mut Vec<Inline>, st: Style| {
        push(out, text, st, false, &link);
        text.clear();
    };
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        // an escaped character: as it is
        if c == '\\' && rest.len() > 1 {
            let n = rest[1..].chars().next().unwrap();
            if n == '\n' {
                flush(&mut text, out, st);
                out.push(Inline::Break);
                i += 2;
                continue;
            }
            if n.is_ascii_punctuation() {
                text.push(n);
                i += 1 + n.len_utf8();
                continue;
            }
        }
        if c == '`' {
            let n = rest.chars().take_while(|x| *x == '`').count();
            if let Some(k) = rest[n..].find(&"`".repeat(n)).filter(|k| !rest[n + k + n..].starts_with('`')) {
                flush(&mut text, out, st);
                let code = rest[n..n + k].replace('\n', " ");
                let code = if code.len() > 2 && code.starts_with(' ') && code.ends_with(' ') { code[1..code.len() - 1].to_string() } else { code };
                push(out, &code, st, true, &link);
                i += n + k + n;
                continue;
            }
            text.push_str(&rest[..n]);
            i += n;
            continue;
        }
        if c == '!' && rest[1..].starts_with('[')
            && let Some(k) = close_bracket(s, i + 1)
        {
            let alt = &s[i + 2..k];
            let dest = if s[k + 1..].starts_with('(') { destination(s, k + 2) } else { None };
            let dest = dest.or_else(|| {
                let (name, end) = if s[k + 1..].starts_with('[') { s[k + 2..].find(']').map(|e| (&s[k + 2..k + 2 + e], k + 3 + e))? } else { (alt, k + 1) };
                refs.get(&(if name.is_empty() { alt } else { name }).to_lowercase()).map(|u| (u.clone(), end))
            });
            if let Some((src, end)) = dest {
                flush(&mut text, out, st);
                out.push(Inline::Image { alt: alt.to_string(), src, link: link.clone() });
                i = end;
                continue;
            }
        }
        if c == '['
            && link.is_none()
            && let Some(k) = close_bracket(s, i)
        {
            let label = &s[i + 1..k];
            let dest = if s[k + 1..].starts_with('(') { destination(s, k + 2) } else { None };
            let dest = dest.or_else(|| {
                let (name, end) = if s[k + 1..].starts_with('[') { s[k + 2..].find(']').map(|e| (&s[k + 2..k + 2 + e], k + 3 + e))? } else { (label, k + 1) };
                refs.get(&(if name.is_empty() { label } else { name }).to_lowercase()).map(|u| (u.clone(), end))
            });
            if let Some((url, end)) = dest {
                flush(&mut text, out, st);
                let mut inner = Vec::new();
                run(label, refs, Some(url), &mut inner);
                // (the link's text takes the style round it)
                for x in inner {
                    out.push(match x {
                        Inline::Span(mut sp) => {
                            sp.bold |= st.bold;
                            sp.em |= st.em;
                            sp.strike |= st.strike;
                            Inline::Span(sp)
                        }
                        x => x,
                    });
                }
                i = end;
                continue;
            }
        }
        if c == '<' {
            if let Some(k) = rest.find('>') {
                let inside = &rest[1..k];
                if (inside.starts_with("http://") || inside.starts_with("https://") || inside.starts_with("mailto:")) && !inside.contains(' ') {
                    flush(&mut text, out, st);
                    push(out, inside.trim_start_matches("mailto:"), st, false, &Some(inside.to_string()));
                    i += k + 1;
                    continue;
                }
                let name: String = inside.trim_start_matches('/').chars().take_while(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
                if !name.is_empty() && name.chars().next().unwrap().is_ascii_alphabetic() {
                    // html: not run, not shown; a picture is a picture, a break a break
                    if name == "img"
                        && let Some(src) = attr(inside, "src")
                    {
                        flush(&mut text, out, st);
                        out.push(Inline::Image { alt: attr(inside, "alt").unwrap_or_default(), src, link: link.clone() });
                    } else if name == "br" {
                        flush(&mut text, out, st);
                        out.push(Inline::Break);
                    }
                    i += k + 1;
                    continue;
                }
                if inside.starts_with("!--") {
                    i += rest.find("-->").map(|e| e + 3).unwrap_or(rest.len());
                    continue;
                }
            }
        }
        if link.is_none() && (rest.starts_with("https://") || rest.starts_with("http://")) && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
            let end = rest.find(|ch: char| ch.is_whitespace() || ch == '<').unwrap_or(rest.len());
            let url = rest[..end].trim_end_matches(['.', ',', ':', ';', '!', '?', ')', '\'', '"']);
            flush(&mut text, out, st);
            push(out, url, st, false, &Some(url.to_string()));
            i += url.len();
            continue;
        }
        // emphasis: a run of * or _ (or ~~) that opens where something follows it and closes where something came
        // before it, if it closes at all
        if c == '*' || c == '_' || rest.starts_with("~~") {
            let n = if c == '~' { 2 } else { rest.chars().take_while(|x| *x == c).count() };
            let before = s[..i].chars().next_back();
            let after = rest[n..].chars().next();
            let word = |x: Option<char>| x.is_some_and(|x| x.is_alphanumeric());
            let can_open = after.is_some_and(|a| !a.is_whitespace()) && !(c == '_' && word(before));
            let can_close = before.is_some_and(|x| !x.is_whitespace()) && !(c == '_' && word(after));
            let delim = &rest[..n];
            let (on, kind) = match (c, n) {
                ('~', _) => (st.strike, 2),
                (_, 1) => (st.em, 1),
                (_, 2) => (st.bold, 2),
                _ => (st.bold && st.em, 3),
            };
            let closes_later = rest[n..].contains(delim);
            if (on && can_close) || (!on && can_open && closes_later) {
                flush(&mut text, out, st);
                match (c, kind) {
                    ('~', _) => st.strike = !st.strike,
                    (_, 1) => st.em = !st.em,
                    (_, 2) => st.bold = !st.bold,
                    _ => {
                        st.bold = !st.bold;
                        st.em = !st.em;
                    }
                }
                i += n;
                continue;
            }
            text.push_str(delim);
            i += n;
            continue;
        }
        if c == '\n' {
            // two spaces before a line's end: a break; otherwise a space
            if text.ends_with("  ") {
                let t = text.trim_end().to_string();
                text = t;
                flush(&mut text, out, st);
                out.push(Inline::Break);
            } else {
                let t = text.trim_end().to_string();
                text = t;
                text.push(' ');
            }
            i += 1;
            continue;
        }
        text.push(c);
        i += c.len_utf8();
    }
    flush(&mut text, out, st);
}

// ---- drawing it

/// a picture asked of the machine: on its way, here, or not to be had
#[derive(Clone)]
pub enum Pic {
    Loading,
    Here(Arc<gpui::Image>),
    Missing,
}

/// what the drawing needs: the pictures so far (by their src), a link clicked, the width there is
pub struct Draw<'a> {
    pub pics: &'a HashMap<String, Pic>,
    pub on_link: Rc<dyn Fn(&str, &mut Window, &mut App)>,
    pub id: &'a str,
}

/// every src of the pictures in it, in order (to ask the machine for)
pub fn pictures(blocks: &[Block]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(bs: &[Block], out: &mut Vec<String>) {
        let inl = |xs: &[Inline], out: &mut Vec<String>| {
            for x in xs {
                if let Inline::Image { src, .. } = x {
                    out.push(src.clone());
                }
            }
        };
        for b in bs {
            match b {
                Block::Heading(_, xs) | Block::Para(xs) => inl(xs, out),
                Block::Quote(bs) => walk(bs, out),
                Block::List(_, items) => items.iter().for_each(|it| walk(it, out)),
                Block::Table(h, rows) => {
                    h.iter().for_each(|c| inl(c, out));
                    rows.iter().flatten().for_each(|c| inl(c, out));
                }
                _ => {}
            }
        }
    }
    walk(blocks, &mut out);
    out
}

const TEXT: u32 = 0xdfe2ec;
const SIZE: f32 = 15.0;

/// the file rendered
pub fn render(blocks: &[Block], d: &Draw) -> AnyElement {
    let mut n = 0usize;
    div().flex().flex_col().font_family(kit::SANS).text_size(px(SIZE)).line_height(px(SIZE * 1.6)).text_color(rgb(TEXT)).children(blocks.iter().map(|b| block(b, d, &mut n, 0))).into_any_element()
}

fn block(b: &Block, d: &Draw, n: &mut usize, depth: usize) -> AnyElement {
    *n += 1;
    let id = format!("{}-{}", d.id, n);
    match b {
        Block::Heading(level, xs) => {
            let (size, rule) = match level {
                1 => (1.7, false),
                2 => (1.35, true),
                3 => (1.12, false),
                _ => (1.0, false),
            };
            let size = SIZE * size;
            div()
                .mt(px(size * 1.0))
                .mb(px(size * 0.5))
                .text_size(px(size))
                .line_height(px(size * 1.25))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(col::STEEL4))
                .when(rule, |e| e.pb(px(size * 0.2)).border_b_1().border_color(rgb(col::LINE)))
                .child(para(xs, d, &id))
                .into_any_element()
        }
        Block::Para(xs) => div().my(px(SIZE * 0.6)).child(para(xs, d, &id)).into_any_element(),
        Block::Code(code) => div()
            .id(gpui::ElementId::Name(id.into()))
            .my(px(SIZE * 0.6))
            .bg(rgb(col::CODE))
            .rounded(px(6.))
            .px(px(14.))
            .py(px(12.))
            .font_family(kit::MONO)
            .text_size(px(13.))
            .line_height(px(13. * 1.5))
            .overflow_x_scroll()
            .children(code.split('\n').map(|l| div().whitespace_nowrap().child(if l.is_empty() { " ".to_string() } else { l.replace('\t', "    ") })))
            .into_any_element(),
        Block::Quote(bs) => div()
            .my(px(SIZE))
            .px(px(SIZE))
            .border_l_3()
            .border_color(rgb(col::LINE))
            .text_color(rgb(col::STEEL2))
            .children(bs.iter().map(|b| block(b, d, n, depth)))
            .into_any_element(),
        Block::List(start, items) => {
            let mut list = div().my(px(SIZE * 0.6)).flex().flex_col();
            for (k, it) in items.iter().enumerate() {
                let mark = match start {
                    Some(s) => format!("{}.", s + k as u64),
                    None => ["•", "◦", "▪"][depth % 3].to_string(),
                };
                // a tight item: its paragraph without the paragraph's margins
                let body = div().flex_1().min_w(px(0.)).flex().flex_col().children(it.iter().map(|b| match b {
                    Block::Para(xs) => {
                        *n += 1;
                        div().child(para(xs, d, &format!("{}-{}", d.id, n))).into_any_element()
                    }
                    b => block(b, d, n, depth + 1),
                }));
                list = list.child(div().flex().gap(px(8.)).child(div().flex_none().w(px(22.)).flex().justify_end().text_color(rgb(col::STEEL2)).child(mark)).child(body));
            }
            list.into_any_element()
        }
        Block::Rule => div().my(px(SIZE)).h(px(1.)).bg(rgb(col::LINE)).into_any_element(),
        Block::Table(head, rows) => {
            let cols = head.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
            let cell = |xs: Option<&Vec<Inline>>, key: String, bold: bool| {
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .px(px(10.))
                    .py(px(4.))
                    .border_1()
                    .border_color(rgb(col::LINE))
                    .when(bold, |e| e.font_weight(FontWeight::BOLD))
                    .child(xs.map(|xs| para(xs, d, &key)).unwrap_or_else(|| div().into_any_element()))
            };
            let row = |r: &Vec<Vec<Inline>>, k: usize, bold: bool| div().flex().children((0..cols).map(|c| cell(r.get(c), format!("{id}-{k}-{c}"), bold)));
            div()
                .my(px(SIZE * 0.6))
                .flex()
                .flex_col()
                .child(row(head, 0, true))
                .children(rows.iter().enumerate().map(|(k, r)| row(r, k + 1, false)))
                .into_any_element()
        }
    }
}

/// a paragraph: its text with its styles and links, its pictures between
fn para(xs: &[Inline], d: &Draw, id: &str) -> AnyElement {
    let mut out = div().flex().flex_wrap().items_end();
    let mut text = String::new();
    let mut highs: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
    let mut links: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut k = 0;
    let mut lines: Vec<AnyElement> = Vec::new();
    let flush = |text: &mut String, highs: &mut Vec<_>, links: &mut Vec<(std::ops::Range<usize>, String)>, k: &mut usize, lines: &mut Vec<AnyElement>| {
        if text.is_empty() {
            return;
        }
        *k += 1;
        let styled = StyledText::new(std::mem::take(text)).with_highlights(std::mem::take(highs));
        let ls = std::mem::take(links);
        let el = if ls.is_empty() {
            styled.into_any_element()
        } else {
            let on = d.on_link.clone();
            let urls: Vec<String> = ls.iter().map(|(_, u)| u.clone()).collect();
            InteractiveText::new(gpui::ElementId::Name(format!("{id}-t{k}").into()), styled)
                .on_click(ls.into_iter().map(|(r, _)| r).collect(), move |i, window, cx| on(&urls[i], window, cx))
                .into_any_element()
        };
        lines.push(div().w_full().child(el).into_any_element());
    };
    for x in xs {
        match x {
            Inline::Span(sp) => {
                let a = text.len();
                text.push_str(&sp.text);
                let mut h = HighlightStyle::default();
                if sp.bold {
                    h.font_weight = Some(FontWeight::BOLD);
                }
                if sp.em {
                    h.font_style = Some(FontStyle::Italic);
                }
                if sp.strike {
                    h.strikethrough = Some(StrikethroughStyle { thickness: px(1.), color: None });
                }
                if sp.code {
                    h.background_color = Some(kit::hsla(col::CODE));
                    h.color = Some(kit::hsla(0xe4e7f0));
                }
                if let Some(u) = &sp.link {
                    h.color = Some(kit::hsla(col::PALE0));
                    h.underline = Some(UnderlineStyle { thickness: px(1.), color: Some(kit::hsla(col::PALE0)), wavy: false });
                    links.push((a..text.len(), u.clone()));
                }
                if h != HighlightStyle::default() {
                    highs.push((a..text.len(), h));
                }
            }
            Inline::Break => text.push('\n'),
            Inline::Image { alt, src, link } => {
                let pic = d.pics.get(src).cloned();
                match pic {
                    Some(Pic::Here(im)) => {
                        flush(&mut text, &mut highs, &mut links, &mut k, &mut lines);
                        let on = d.on_link.clone();
                        let to = link.clone();
                        k += 1;
                        let el = div().id(gpui::ElementId::Name(format!("{id}-i{k}").into())).max_w_full().child(img(im).max_w_full()).when_some(to, |e, to| {
                            e.cursor_pointer().on_click(move |_, window, cx| on(&to, window, cx))
                        });
                        lines.push(el.into_any_element());
                    }
                    // not (yet) here: what it shows, in words
                    _ => {
                        let a = text.len();
                        text.push_str(if alt.is_empty() { "[picture]" } else { alt });
                        let to = link.clone().or_else(|| (src.starts_with("http://") || src.starts_with("https://")).then(|| src.clone()));
                        let mut h = HighlightStyle { color: Some(kit::hsla(col::STEEL2)), font_style: Some(FontStyle::Italic), ..Default::default() };
                        if let Some(u) = to {
                            h.color = Some(kit::hsla(col::PALE0));
                            links.push((a..text.len(), u));
                        }
                        highs.push((a..text.len(), h));
                    }
                }
            }
        }
    }
    flush(&mut text, &mut highs, &mut links, &mut k, &mut lines);
    // (text and pictures each a row of its own: a picture among words is rare outside badges)
    out = out.children(lines);
    out.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spans(xs: &[Inline]) -> Vec<(String, &'static str)> {
        xs.iter()
            .map(|x| match x {
                Inline::Span(s) => (
                    s.text.clone(),
                    match (s.bold, s.em, s.code, s.strike, s.link.is_some()) {
                        (_, _, true, _, _) => "code",
                        (_, _, _, _, true) => "link",
                        (true, true, ..) => "both",
                        (true, ..) => "bold",
                        (_, true, ..) => "em",
                        (.., true, _) => "strike",
                        _ => "",
                    },
                ),
                Inline::Image { src, .. } => (src.clone(), "img"),
                Inline::Break => ("\n".into(), "br"),
            })
            .collect()
    }

    #[test]
    fn inlines() {
        let r = Refs::from([("ref".to_string(), "https://r.example".to_string())]);
        let s = spans(&inline("a **b** *c* `d*` ~~e~~ [f](g.md) ![h](i.png) <https://j> [k][ref] 2 * 3 snake_case_name", &r));
        assert_eq!(
            s,
            [
                ("a ".into(), ""),
                ("b".into(), "bold"),
                (" ".into(), ""),
                ("c".into(), "em"),
                (" ".into(), ""),
                ("d*".into(), "code"),
                (" ".into(), ""),
                ("e".into(), "strike"),
                (" ".into(), ""),
                ("f".into(), "link"),
                (" ".into(), ""),
                ("i.png".into(), "img"),
                (" ".into(), ""),
                ("https://j".into(), "link"),
                (" ".into(), ""),
                ("k".into(), "link"),
                (" 2 * 3 snake_case_name".into(), ""),
            ]
        );
        let s = spans(&inline("see https://x.org/a. then\nmore  \nbroken<br>html <b>bold?</b> <img src=\"p.png\" alt=\"P\">", &Refs::new()));
        assert_eq!(s[1], ("https://x.org/a".into(), "link"));
        assert!(s.iter().any(|x| x.1 == "br"));
        assert!(s.iter().any(|x| x == &("p.png".into(), "img")), "{s:?}");
        assert!(!s.iter().any(|x| x.0.contains('<')), "tags go: {s:?}");
    }

    #[test]
    fn blocks_of_a_readme() {
        let md = "# kite\n\nA made-up **project**.\n\n- the room\n- the robots\n  - nested\n\n1. one\n2. two\n\n```js\nconst kite = fly();\n```\n\n> quoted\n> on\n\n---\n\n| a | b |\n|---|:-:|\n| 1 | 2 |\n\nTitle\n=====\n\n    indented\n";
        let b = parse(md);
        assert!(matches!(&b[0], Block::Heading(1, _)));
        assert!(matches!(&b[1], Block::Para(_)));
        let Block::List(None, items) = &b[2] else { panic!("{:?}", b[2]) };
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1].last(), Some(Block::List(None, _))), "a list in a list: {:?}", items[1]);
        assert!(matches!(&b[3], Block::List(Some(1), it) if it.len() == 2));
        assert_eq!(b[4], Block::Code("const kite = fly();".into()));
        assert!(matches!(&b[5], Block::Quote(q) if matches!(&q[0], Block::Para(_))));
        assert_eq!(b[6], Block::Rule);
        assert!(matches!(&b[7], Block::Table(h, rows) if h.len() == 2 && rows.len() == 1));
        assert!(matches!(&b[8], Block::Heading(1, _)));
        assert_eq!(b[9], Block::Code("indented".into()));
        assert_eq!(b.len(), 10);
        assert_eq!(pictures(&parse("![a](x.png)\n\n- ![b](y.gif)")), ["x.png", "y.gif"]);
    }

    #[test]
    fn a_paragraph_ends_where_another_block_starts() {
        let b = parse("text\n# head\nmore\n- item\n[r]: https://x\n");
        assert_eq!(b.len(), 4, "{b:?}");
        assert!(matches!(&b[1], Block::Heading(1, _)));
        assert!(matches!(&b[3], Block::List(None, _)));
    }
}
