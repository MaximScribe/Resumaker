// Modern: sans-serif, with a sidebar for contact details, skills and languages.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let accent = rgb("#0f766e")
#let muted = luma(95)
#let side-kinds = ("skills", "languages", "interests")

#set document(title: header.name, author: header.name)
#let sidebar = 6.2cm
#set page(
  paper: "a4",
  margin: (left: 0cm, right: 1.4cm, y: 1.3cm),
  background: place(left + top, rect(width: sidebar, height: 100%, fill: accent.lighten(92%))),
)
#set text(
  font: ("Inter", "Segoe UI", "Helvetica Neue", "Arial", "DejaVu Sans", "Libertinus Serif"),
  size: if opts.at("pages", default: none) == 1 { 9.5pt } else { 10pt },
  lang: opts.lang,
)
#set par(leading: 0.55em, spacing: 0.6em)
#set list(indent: 0.2em, body-indent: 0.45em, spacing: 0.4em, marker: text(fill: accent, "▸"))
#show link: set text(fill: accent)

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }
#let heading-bar(title) = block(above: 1.1em, below: 0.55em, sticky: true,
  text(size: 0.95em, weight: "bold", fill: accent, tracking: 0.08em, upper(title)))

#let entry(e) = block(above: 0.75em)[
  #text(weight: "bold", maybe-link(e.title, e.url))
  #if e.subtitle != none [ #text(fill: muted)[· #e.subtitle]]
  #let meta = (e.dates, e.location).filter(x => x != none)
  #if meta.len() > 0 { linebreak(); text(size: 0.9em, fill: muted, meta.join(" · ")) }
  #if e.summary != none { block(above: 0.35em, e.summary) }
  #if e.bullets.len() > 0 { block(above: 0.35em, list(..e.bullets)) }
]

#let side-entry(e) = block(above: 0.55em)[
  #text(weight: "bold", e.title)
  #if e.subtitle != none { linebreak(); text(size: 0.9em, fill: muted, e.subtitle) }
  #if e.summary != none { linebreak(); text(size: 0.92em, e.summary) }
]

#grid(
  columns: (sidebar, 1fr),
  column-gutter: 0.9cm,
  block(inset: (left: 1.2cm, right: 0.6cm), width: 100%)[
    #text(size: 1.9em, weight: "bold", fill: accent, header.name)
    #if header.label != none { block(above: 0.4em, text(size: 1.05em, header.label)) }
    #v(0.8em)
    #set text(size: 0.9em)
    #if header.location != none { block(above: 0.4em, header.location) }
    #for c in header.contacts { block(above: 0.4em, maybe-link(c.text, c.url)) }
    #for section in view.sections.filter(s => s.kind in side-kinds) {
      heading-bar(section.title)
      for e in section.entries { side-entry(e) }
    }
  ],
  [
    #if header.summary != none {
      heading-bar(if opts.lang == "de" { "Profil" } else { "Profile" })
      header.summary
    }
    #for section in view.sections.filter(s => s.kind not in side-kinds) {
      heading-bar(section.title)
      for e in section.entries { entry(e) }
    }
  ],
)
