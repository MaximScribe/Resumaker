use std::io::Cursor;

use docx_rs::{
    AlignmentType, Docx, LineSpacing, PageMargin, Paragraph, Run, RunFonts, SpecialIndentType,
};

use super::view::View;
use crate::Error;

// docx sizes are in half-points; spacing and indents in twentieths of a point.
const BODY: usize = 21;
const MUTED: &str = "57606A";

pub fn render(view: &View) -> Result<Vec<u8>, Error> {
    let h = &view.header;
    let mut doc = Docx::new()
        .default_fonts(RunFonts::new().ascii("Calibri").hi_ansi("Calibri"))
        .default_size(BODY)
        .page_margin(
            PageMargin::new()
                .top(1000)
                .bottom(1000)
                .left(1100)
                .right(1100),
        );

    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(&h.name).bold().size(40)));
    if let Some(label) = &h.label {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(label).size(26).color(MUTED)),
        );
    }
    let contacts: Vec<&str> = h
        .location
        .iter()
        .map(String::as_str)
        .chain(h.contacts.iter().map(|c| c.text.as_str()))
        .collect();
    if !contacts.is_empty() {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(contacts.join("  ·  ")).color(MUTED)),
        );
    }
    if let Some(summary) = &h.summary {
        doc = doc.add_paragraph(spaced(
            Paragraph::new().add_run(Run::new().add_text(summary)),
            120,
        ));
    }

    for section in &view.sections {
        doc = doc.add_paragraph(
            spaced(Paragraph::new(), 280).keep_next(true).add_run(
                Run::new()
                    .add_text(section.title.to_uppercase())
                    .bold()
                    .size(23),
            ),
        );
        for e in &section.entries {
            let mut head = Paragraph::new()
                .keep_next(true)
                .add_run(Run::new().add_text(&e.title).bold());
            if let Some(sub) = &e.subtitle {
                head = head.add_run(Run::new().add_text(format!(" · {sub}")).color(MUTED));
            }
            doc = doc.add_paragraph(spaced(head, 100));

            let meta: Vec<&str> = [&e.dates, &e.location]
                .into_iter()
                .flatten()
                .map(String::as_str)
                .collect();
            if !meta.is_empty() {
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text(meta.join(" · ")).italic().color(MUTED)),
                );
            }
            if let Some(summary) = &e.summary {
                doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(summary)));
            }
            for b in &e.bullets {
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .align(AlignmentType::Left)
                        .indent(Some(360), Some(SpecialIndentType::Hanging(220)), None, None)
                        .add_run(Run::new().add_text(format!("•\u{a0}{b}"))),
                );
            }
        }
    }

    let mut buf = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut buf)
        .map_err(|e| Error::Render(format!("docx: {e}")))?;
    Ok(buf.into_inner())
}

fn spaced(p: Paragraph, before: u32) -> Paragraph {
    p.line_spacing(LineSpacing::new().before(before))
}
