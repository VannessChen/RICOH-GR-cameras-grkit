//! Installation guides as Typst sources. Each package carries the guide of its
//! camera in one language as a PDF. The release compiles these sources with
//! Typst (`.github/scripts/build-guides.sh`).
//!
//! The converter accepts only the Markdown that the guides in `resources/` use:
//! headings, paragraphs, ordered and unordered lists, one image per line,
//! strong text, code spans, and links.

use anyhow::{bail, ensure, Result};

use crate::{
    camera::Camera,
    packaging::{guide_images, installation_guide, Language},
    Files,
};

/// The source file of a guide, next to the images that it shows.
pub const SOURCE: &str = "guide.typ";
/// The label of the recovery appendix. Links to the recovery guide point to it.
const RECOVERY_LABEL: &str = "recovery";

fn recovery_guide(language: Language) -> &'static str {
    match language {
        Language::English => include_str!("../../../resources/RECOVERY.md"),
        Language::SimplifiedChinese => include_str!("../../../resources/RECOVERY.zh-Hans.md"),
        Language::TraditionalChinese => include_str!("../../../resources/RECOVERY.zh-Hant.md"),
        Language::Japanese => include_str!("../../../resources/RECOVERY.ja.md"),
    }
}

/// The font and the text language of a guide. The Noto Sans CJK fonts also
/// cover Latin text, so one family sets the whole guide. They draw curly
/// quotes full width, so the guides keep straight quotes.
fn text_settings(language: Language) -> &'static str {
    match language {
        Language::English => r#"font: "Noto Sans SC", lang: "en""#,
        Language::SimplifiedChinese => r#"font: "Noto Sans SC", lang: "zh", region: "cn""#,
        Language::TraditionalChinese => r#"font: "Noto Sans TC", lang: "zh", region: "tw""#,
        Language::Japanese => r#"font: "Noto Sans JP", lang: "ja""#,
    }
}

/// The Typst source of the guide of `camera` in `language`, with the images
/// that it shows: `guide.typ` and `images/*`. The recovery guide follows as an
/// appendix on its own page.
pub fn source(camera: &Camera, language: Language) -> Result<Files> {
    let guide = String::from_utf8(installation_guide(camera, language))?;
    let title = language.guide_title();
    let mut typst = format!(
        "#set document(title: \"{title}\", author: \"GRKit\", date: none)\n\
         #set page(paper: \"a5\", margin: (x: 15mm, y: 16mm), numbering: \"1\")\n\
         #set text({}, size: 9.5pt)\n\
         #set par(leading: 0.75em, spacing: 1.1em)\n\
         #set list(indent: 0.6em)\n\
         #set enum(indent: 0.6em)\n\
         #set smartquote(enabled: false)\n\
         #show raw: set text(size: 1.15em)\n\
         #show heading: set block(above: 1.5em, below: 0.8em)\n\
         #show heading.where(level: 1): set text(size: 17pt)\n\
         #show heading.where(level: 2): set text(size: 12pt)\n\
         #show link: underline\n\n",
        text_settings(language)
    );
    typst.push_str(&markdown(&guide, None)?);
    typst.push_str("\n#pagebreak()\n\n");
    typst.push_str(&markdown(recovery_guide(language), Some(RECOVERY_LABEL))?);
    let mut files = Files::from([(SOURCE.to_owned(), typst.into_bytes())]);
    for (name, image) in guide_images(camera) {
        files.insert((*name).to_owned(), image.to_vec());
    }
    Ok(files)
}

/// Converts guide Markdown to Typst markup. `label` names the first heading.
pub fn markdown(text: &str, label: Option<&str>) -> Result<String> {
    let mut out = String::new();
    let mut paragraph = String::new();
    let mut label = label;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            flush(&mut out, &mut paragraph);
            out.push('\n');
            continue;
        }
        let level = line.bytes().take_while(|b| *b == b'#').count();
        if level > 0 && line.as_bytes().get(level) == Some(&b' ') {
            ensure!(level <= 3, "Unsupported heading level in {line:?}");
            flush(&mut out, &mut paragraph);
            out.push_str(&"=".repeat(level));
            out.push(' ');
            out.push_str(&inline(&line[level + 1..])?);
            if let Some(name) = label.take() {
                out.push_str(&format!(" <{name}>"));
            }
            out.push('\n');
        } else if let Some(item) = line.strip_prefix("- ") {
            flush(&mut out, &mut paragraph);
            out.push_str("- ");
            out.push_str(&inline(item)?);
            out.push('\n');
        } else if let Some(item) = ordered_item(line) {
            flush(&mut out, &mut paragraph);
            out.push_str("+ ");
            out.push_str(&inline(item)?);
            out.push('\n');
        } else if let Some((alt, path)) = image(line) {
            flush(&mut out, &mut paragraph);
            out.push_str(&format!(
                "#image({}, width: 100%, alt: {})\n",
                string(path),
                string(alt)
            ));
        } else {
            ensure!(
                !line.starts_with(['>', '|', '*', '+', ' ', '\t']),
                "Unsupported Markdown in {line:?}"
            );
            if !(paragraph.is_empty() || ends_wide(&paragraph) || starts_wide(line)) {
                paragraph.push(' ');
            }
            paragraph.push_str(line);
        }
    }
    flush(&mut out, &mut paragraph);
    Ok(out)
}

/// Writes the pending paragraph. Markdown joins its lines, so the paragraph is
/// one line of Typst markup.
fn flush(out: &mut String, paragraph: &mut String) {
    if paragraph.is_empty() {
        return;
    }
    let text = inline(paragraph).expect("paragraph text was checked line by line");
    // A paragraph must not start a heading, a list item, or an enumeration.
    if text.starts_with(['=', '-', '+']) || ordered_item(&text).is_some() {
        out.push('\\');
    }
    out.push_str(&text);
    out.push('\n');
    paragraph.clear();
}

/// Chinese text joins its source lines without a space.
fn wide(c: char) -> bool {
    c >= '\u{2e80}'
}

fn ends_wide(text: &str) -> bool {
    text.chars().next_back().is_some_and(wide)
}

fn starts_wide(text: &str) -> bool {
    text.chars().next().is_some_and(wide)
}

fn ordered_item(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    (digits > 0 && line[digits..].starts_with(". ")).then(|| &line[digits + 2..])
}

/// `![alt](path)` on a line of its own.
fn image(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("![")?;
    let (alt, rest) = rest.split_once("](")?;
    let path = rest.strip_suffix(')')?;
    (!path.contains([' ', ')'])).then_some((alt, path))
}

/// A Typst string literal.
fn string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Converts strong text, code spans, links, and bare URLs, and escapes the
/// other characters that Typst markup reads as syntax.
fn inline(text: &str) -> Result<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("**") {
            out.push('*');
            rest = after;
        } else if c == '`' {
            let Some(end) = rest[1..].find('`') else {
                bail!("Unclosed code span in {text:?}");
            };
            out.push_str(&rest[..end + 2]);
            rest = &rest[end + 2..];
        } else if let Some((label, url, after)) = link(rest) {
            let target = if url.contains("RECOVERY") {
                format!("<{RECOVERY_LABEL}>")
            } else {
                string(url)
            };
            out.push_str(&format!("#link({target})[{}]", inline(label)?));
            rest = after;
        } else if rest.starts_with("https://") || rest.starts_with("http://") {
            // Typst links a bare URL by itself.
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
        } else {
            if matches!(
                c,
                '\\' | '#' | '$' | '*' | '_' | '`' | '@' | '<' | '>' | '[' | ']' | '~' | '/'
            ) {
                out.push('\\');
            }
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(out)
}

/// `[label](url)` at the start of `text`, and the text after it.
fn link(text: &str) -> Option<(&str, &str, &str)> {
    let rest = text.strip_prefix('[')?;
    let (label, rest) = rest.split_once("](")?;
    let (url, after) = rest.split_once(')')?;
    (!label.contains('[') && !url.contains(char::is_whitespace)).then_some((label, url, after))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_blocks_become_typst_blocks() {
        let text = "# GRKit\n\nFor GR III.\nSecond line.\n\n## Install\n\n1. Turn the camera off.\n2. Copy `SD_ROOT`.\n\n- **Grain:** select [Strong].\n\n![The page.](images/a.png)\n";
        assert_eq!(
            markdown(text, Some("top")).unwrap(),
            "= GRKit <top>\n\nFor GR III. Second line.\n\n== Install\n\n+ Turn the camera off.\n+ Copy `SD_ROOT`.\n\n- *Grain:* select \\[Strong\\].\n\n#image(\"images/a.png\", width: 100%, alt: \"The page.\")\n"
        );
    }

    #[test]
    fn chinese_lines_join_without_a_space() {
        assert_eq!(
            markdown("第一行。\n第二行。\n", None).unwrap(),
            "第一行。第二行。\n"
        );
    }

    #[test]
    fn links_to_the_recovery_guide_point_to_the_appendix() {
        let text = "See the [recovery guide](https://example.com/RECOVERY.md). Code: https://github.com/ahonn/grkit\n";
        assert_eq!(
            markdown(text, None).unwrap(),
            "See the #link(<recovery>)[recovery guide]. Code: https://github.com/ahonn/grkit\n"
        );
    }

    #[test]
    fn markup_characters_are_escaped() {
        assert_eq!(
            inline("a_b #1 $2 @x <y> ~z c/d").unwrap(),
            "a\\_b \\#1 \\$2 \\@x \\<y\\> \\~z c\\/d"
        );
        assert!(markdown("> quote\n", None).is_err());
        assert!(markdown("#### Deep\n", None).is_err());
        assert!(inline("`open").is_err());
    }
}
