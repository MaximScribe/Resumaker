// Compact: dense one-page layout with section titles in a margin column.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let muted = luma(100)

#set document(title: header.name, author: header.name)
#set page(paper: "a4", margin: (x: 1.3cm, y: 1.1cm))
#set text(
  font: ("Inter", "Segoe UI", "Helvetica Neue", "Arial", "DejaVu Sans", "Libertinus Serif"),
  size: 9pt,
  lang: opts.lang,
)
#set par(leading: 0.5em, spacing: 0.5em)
#set list(indent: 0em, body-indent: 0.4em, spacing: 0.32em)
#show link: set text(fill: rgb("#1d4ed8"))

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }

#grid(
  columns: (1fr, auto),
  align: (left + bottom, right + bottom),
  [
    #text(size: 1.9em, weight: "bold", header.name)
    #if header.label != none [ #h(0.6em)#text(size: 1.15em, fill: muted, header.label)]
  ],
  text(size: 0.95em, {
    let items = ()
    if header.location != none { items.push(header.location) }
    for c in header.contacts { items.push(maybe-link(c.text, c.url)) }
    items.join(" · ")
  }),
)
#line(length: 100%, stroke: 0.8pt)
#if header.summary != none { block(above: 0.6em, header.summary) }

#let row(title, body) = grid(
  columns: (2.6cm, 1fr),
  column-gutter: 0.4cm,
  text(weight: "bold", size: 0.95em, upper(title)),
  body,
)

#for section in view.sections {
  block(above: 0.9em, breakable: true, row(section.title, {
    for e in section.entries {
      block(above: 0.5em, below: 0.3em)[
        #grid(
          columns: (1fr, auto),
          column-gutter: 0.6em,
          [
            #text(weight: "bold", maybe-link(e.title, e.url))
            #if e.subtitle != none [, #e.subtitle]
            #if section.kind in ("skills", "languages", "interests") and e.summary != none [: #e.summary]
          ],
          text(fill: muted, (e.dates, e.location).filter(x => x != none).join(" · ")),
        )
        #if e.summary != none and section.kind not in ("skills", "languages", "interests") {
          block(above: 0.3em, e.summary)
        }
        #if e.bullets.len() > 0 { block(above: 0.3em, list(..e.bullets)) }
      ]
    }
  }))
}
