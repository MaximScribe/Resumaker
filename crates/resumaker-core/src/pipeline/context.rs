//! Job context: free text, a job-posting file or a web page.

use std::path::Path;

use crate::Error;
use crate::sources::{FileKind, extract};

/// Longest context kept per item; job postings are rarely longer.
const MAX_CHARS: usize = 20_000;

/// Where a context item came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextOrigin {
    Text,
    File(String),
    Url(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub origin: ContextOrigin,
    pub text: String,
}

/// Interprets a `--context` value: an http(s) URL is fetched, an existing file is
/// read, anything else is taken as text.
pub fn load(item: &str) -> Result<Context, Error> {
    let trimmed = item.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        let html = fetch(trimmed)?;
        return Ok(Context {
            origin: ContextOrigin::Url(trimmed.to_owned()),
            text: limit(html_to_text(&html)),
        });
    }
    let path = Path::new(trimmed);
    if path.is_file() {
        let kind = FileKind::from_path(path).ok_or_else(|| Error::Unreadable {
            path: path.to_path_buf(),
            reason: "unsupported file type for job context".into(),
        })?;
        return Ok(Context {
            origin: ContextOrigin::File(trimmed.to_owned()),
            text: limit(extract::extract(path, kind)?),
        });
    }
    Ok(Context {
        origin: ContextOrigin::Text,
        text: limit(trimmed.to_owned()),
    })
}

fn fetch(url: &str) -> Result<String, Error> {
    crate::llm::http::get_text(url)
}

fn limit(text: String) -> String {
    match text.char_indices().nth(MAX_CHARS) {
        Some((i, _)) => text[..i].to_owned(),
        None => text,
    }
}

/// Readable text from an HTML page: drops scripts, styles and tags, keeps block
/// structure as line breaks and decodes common entities.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 3);
    let lower = html.to_ascii_lowercase();
    let mut i = 0;
    while i < html.len() {
        let rest = &lower[i..];
        if rest.starts_with("<script")
            || rest.starts_with("<style")
            || rest.starts_with("<noscript")
        {
            let tag = if rest.starts_with("<script") {
                "</script>"
            } else if rest.starts_with("<style") {
                "</style>"
            } else {
                "</noscript>"
            };
            i = rest
                .find(tag)
                .map(|p| i + p + tag.len())
                .unwrap_or(html.len());
            continue;
        }
        if rest.starts_with("<!--") {
            i = rest.find("-->").map(|p| i + p + 3).unwrap_or(html.len());
            continue;
        }
        if let Some(tag) = rest.strip_prefix('<') {
            let end = rest.find('>').map(|p| i + p + 1).unwrap_or(html.len());
            let name: String = tag
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if matches!(
                name.as_str(),
                "p" | "div"
                    | "br"
                    | "li"
                    | "ul"
                    | "ol"
                    | "tr"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
                    | "section"
                    | "article"
                    | "header"
                    | "footer"
                    | "table"
            ) {
                out.push('\n');
                if name == "li" && !rest.starts_with("</") {
                    out.push_str("- ");
                }
            }
            i = end;
            continue;
        }
        let next = rest.find('<').map(|p| i + p).unwrap_or(html.len());
        out.push_str(&decode_entities(&html[i..next]));
        i = next;
    }
    let lines: Vec<String> = out
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty() && l != "-")
        .collect();
    lines.join("\n")
}

fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            "ndash" => Some('–'),
            "mdash" => Some('—'),
            _ => entity
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_stays_text() {
        let c = load("Backend developer, Rust").unwrap();
        assert_eq!(c.origin, ContextOrigin::Text);
        assert_eq!(c.text, "Backend developer, Rust");
    }

    #[test]
    fn strips_html() {
        let html = r#"<html><head><style>p{}</style><script>var x="<p>";</script></head>
            <body><h1>Rust&nbsp;Engineer</h1><!-- c --><p>Build &amp; ship</p>
            <ul><li>Tokio</li><li>Postgres &#x2013; 5y</li></ul></body></html>"#;
        assert_eq!(
            html_to_text(html),
            "Rust Engineer\nBuild & ship\n- Tokio\n- Postgres – 5y"
        );
    }

    #[test]
    fn keeps_unknown_entities() {
        assert_eq!(decode_entities("a & b &bogus; &#65;"), "a & b &bogus; A");
    }
}
