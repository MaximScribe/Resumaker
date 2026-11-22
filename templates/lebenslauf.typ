// Lebenslauf: the tabular German CV, dates in the left column.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let de = opts.lang == "de"
#let accent = rgb("#1e3a8a")
#let muted = luma(90)

#set document(title: header.name, author: header.name)
#set page(paper: "a4", margin: (x: 2cm, y: 1.8cm))
#set text(
  font: ("Source Sans 3", "Segoe UI", "Helvetica Neue", "Arial", "DejaVu Sans", "Libertinus Serif"),
  size: 10.5pt,
  lang: opts.lang,
)
#set par(leading: 0.55em, spacing: 0.6em)
#set list(indent: 0em, body-indent: 0.45em, spacing: 0.4em)
#show link: set text(fill: accent)

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }
#let left-col = 4cm

#text(size: 1.9em, weight: "bold", fill: accent, if de { "Lebenslauf" } else { "Curriculum Vitae" })
#line(length: 100%, stroke: 1pt + accent)

#let title(t) = block(above: 1.3em, below: 0.7em, sticky: true,
  text(size: 1.1em, weight: "bold", fill: accent, t))

#let row(left, right) = grid(
  columns: (left-col, 1fr),
  column-gutter: 0.6cm,
  text(fill: muted, left),
  right,
)

#title(if de { "Persönliche Daten" } else { "Personal details" })
#row(if de { "Name" } else { "Name" }, text(weight: "bold", header.name))
#if header.label != none { block(above: 0.45em, row(if de { "Profil" } else { "Profile" }, header.label)) }
#if header.location != none { block(above: 0.45em, row(if de { "Wohnort" } else { "Location" }, header.location)) }
#for (i, c) in header.contacts.enumerate() {
  let label = if i == 0 { if de { "Kontakt" } else { "Contact" } } else { [] }
  block(above: 0.45em, row(label, maybe-link(c.text, c.url)))
}
#if header.summary != none { block(above: 0.9em, row([], header.summary)) }

#let compact-kinds = ("skills", "languages", "interests")

#for section in view.sections {
  title(section.title)
  if section.kind in compact-kinds {
    for e in section.entries {
      let detail = (e.subtitle, e.summary).filter(x => x != none).join(" · ")
      block(above: 0.45em, row(e.title, if detail == none { [] } else { detail }))
    }
    continue
  }
  for e in section.entries {
    block(above: 0.75em, breakable: true, row(
      if e.dates != none { e.dates } else { [] },
      [
        #text(weight: "bold", maybe-link(e.title, e.url))
        #{
          let line = (e.subtitle, e.location).filter(x => x != none)
          if line.len() > 0 { linebreak(); line.join(", ") }
        }
        #if e.summary != none { block(above: 0.35em, e.summary) }
        #if e.bullets.len() > 0 { block(above: 0.35em, list(..e.bullets)) }
      ],
    ))
  }
}
