use std::fmt::Write;

use super::view::View;

const STYLE: &str = "\
body{font:15px/1.5 system-ui,-apple-system,'Segoe UI',sans-serif;color:#1f2328;max-width:800px;margin:40px auto;padding:0 16px}\
h1{margin:0;font-size:2em}.label{margin:.2em 0;font-size:1.15em;color:#57606a}\
.contacts{margin:.4em 0 1em;color:#57606a}.contacts a{color:inherit}\
h2{margin:1.6em 0 .6em;font-size:1em;letter-spacing:.08em;text-transform:uppercase;border-bottom:1px solid #d0d7de;padding-bottom:.2em}\
.entry{margin:0 0 1em}.head{display:flex;justify-content:space-between;gap:1em;flex-wrap:wrap}\
h3{margin:0;font-size:1em}.sub{font-weight:normal;color:#57606a}.meta{color:#57606a;white-space:nowrap}\
p{margin:.25em 0}ul{margin:.25em 0;padding-left:1.2em}\
@media print{body{margin:0}}";

pub fn render(view: &View) -> String {
    let h = &view.header;
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"{}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<header>\n<h1>{}</h1>\n",
        view.lang.code(),
        esc(&h.name),
        esc(&h.name)
    );
    if let Some(label) = &h.label {
        let _ = writeln!(out, "<p class=\"label\">{}</p>", esc(label));
    }
    let contacts: Vec<String> = h
        .location
        .iter()
        .map(|l| esc(l))
        .chain(h.contacts.iter().map(|c| match &c.url {
            Some(url) => format!("<a href=\"{}\">{}</a>", esc(url), esc(&c.text)),
            None => esc(&c.text),
        }))
        .collect();
    if !contacts.is_empty() {
        let _ = writeln!(out, "<p class=\"contacts\">{}</p>", contacts.join(" · "));
    }
    if let Some(summary) = &h.summary {
        let _ = writeln!(out, "<p>{}</p>", esc(summary));
    }
    out.push_str("</header>\n");

    for section in &view.sections {
        let _ = writeln!(out, "<section>\n<h2>{}</h2>", esc(&section.title));
        for e in &section.entries {
            out.push_str("<div class=\"entry\">\n<div class=\"head\"><h3>");
            match &e.url {
                Some(url) => {
                    let _ = write!(out, "<a href=\"{}\">{}</a>", esc(url), esc(&e.title));
                }
                None => out.push_str(&esc(&e.title)),
            }
            if let Some(sub) = &e.subtitle {
                let _ = write!(out, " <span class=\"sub\">· {}</span>", esc(sub));
            }
            out.push_str("</h3>");
            let meta: Vec<String> = [&e.dates, &e.location]
                .into_iter()
                .flatten()
                .map(|m| esc(m))
                .collect();
            if !meta.is_empty() {
                let _ = write!(out, "<span class=\"meta\">{}</span>", meta.join(" · "));
            }
            out.push_str("</div>\n");
            if let Some(summary) = &e.summary {
                let _ = writeln!(out, "<p>{}</p>", esc(summary));
            }
            if !e.bullets.is_empty() {
                out.push_str("<ul>\n");
                for b in &e.bullets {
                    let _ = writeln!(out, "<li>{}</li>", esc(b));
                }
                out.push_str("</ul>\n");
            }
            out.push_str("</div>\n");
        }
        out.push_str("</section>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_markup() {
        assert_eq!(
            super::esc("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }
}
