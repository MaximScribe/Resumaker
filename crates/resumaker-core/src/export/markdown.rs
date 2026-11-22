use std::fmt::Write;

use super::view::View;

pub fn render(view: &View) -> String {
    let mut out = String::new();
    let h = &view.header;
    let _ = writeln!(out, "# {}\n", h.name);
    if let Some(label) = &h.label {
        let _ = writeln!(out, "**{label}**\n");
    }
    let contacts: Vec<String> = h
        .location
        .iter()
        .cloned()
        .chain(h.contacts.iter().map(|c| match &c.url {
            Some(url) => format!("[{}]({url})", c.text),
            None => c.text.clone(),
        }))
        .collect();
    if !contacts.is_empty() {
        let _ = writeln!(out, "{}\n", contacts.join(" · "));
    }
    if let Some(summary) = &h.summary {
        let _ = writeln!(out, "{summary}\n");
    }

    for section in &view.sections {
        let _ = writeln!(out, "## {}\n", section.title);
        for e in &section.entries {
            let _ = write!(out, "### {}", e.title);
            if let Some(sub) = &e.subtitle {
                let _ = write!(out, " — {sub}");
            }
            out.push_str("\n\n");
            let meta: Vec<&str> = [&e.dates, &e.location]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            if !meta.is_empty() {
                let _ = writeln!(out, "*{}*\n", meta.join(" · "));
            }
            if let Some(summary) = &e.summary {
                let _ = writeln!(out, "{summary}\n");
            }
            for b in &e.bullets {
                let _ = writeln!(out, "- {b}");
            }
            if !e.bullets.is_empty() {
                out.push('\n');
            }
        }
    }
    let trimmed = out.trim_end().len();
    out.truncate(trimmed);
    out.push('\n');
    out
}
