use std::fmt::Write;

use super::view::View;

pub fn render(view: &View) -> String {
    let mut out = String::new();
    let h = &view.header;
    let _ = writeln!(out, "{}", h.name.to_uppercase());
    if let Some(label) = &h.label {
        let _ = writeln!(out, "{label}");
    }
    let contacts: Vec<&str> = h
        .location
        .iter()
        .map(String::as_str)
        .chain(h.contacts.iter().map(|c| c.text.as_str()))
        .collect();
    if !contacts.is_empty() {
        let _ = writeln!(out, "{}", contacts.join(" | "));
    }
    if let Some(summary) = &h.summary {
        let _ = writeln!(out, "\n{summary}");
    }

    for section in &view.sections {
        let title = section.title.to_uppercase();
        let _ = writeln!(out, "\n\n{title}\n{}", "=".repeat(title.chars().count()));
        for e in &section.entries {
            let _ = write!(out, "\n{}", e.title);
            if let Some(sub) = &e.subtitle {
                let _ = write!(out, ", {sub}");
            }
            out.push('\n');
            let meta: Vec<&str> = [&e.dates, &e.location]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            if !meta.is_empty() {
                let _ = writeln!(out, "{}", meta.join(" | "));
            }
            if let Some(summary) = &e.summary {
                let _ = writeln!(out, "{summary}");
            }
            for b in &e.bullets {
                let _ = writeln!(out, "  * {b}");
            }
        }
    }
    out
}
