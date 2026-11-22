// Classic: a single-column, serif resume.
//
// Data: /view.json (prepared sections), /profile.json (raw JSON Resume), /options.json.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let accent = rgb("#1f3a5f")
#let muted = luma(90)
#let compact = opts.at("pages", default: none) == 1

#set document(title: header.name, author: header.name)
#set page(paper: "a4", margin: (x: 1.7cm, y: 1.5cm))
#set text(font: ("Libertinus Serif", "New Computer Modern"), size: if compact { 10pt } else { 10.5pt }, lang: opts.lang)
#set par(leading: 0.55em, spacing: 0.6em)
#set list(indent: 0.3em, body-indent: 0.5em, spacing: 0.45em)
#show link: set text(fill: accent)

#let section-title(title) = block(
  above: 1.3em,
  below: 0.5em,
  width: 100%,
  inset: (bottom: 3pt),
  stroke: (bottom: 0.6pt + accent.lighten(40%)),
  sticky: true,
  text(size: 1.05em, weight: "bold", fill: accent, tracking: 0.06em, upper(title)),
)

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }

#let entry(e) = block(above: 0.8em, breakable: true)[
  #grid(
    columns: (1fr, auto),
    column-gutter: 1em,
    [
      #text(weight: "bold", maybe-link(e.title, e.url))
      #if e.subtitle != none [ #h(0.2em)#text(fill: muted)[· #e.subtitle] ]
    ],
    align(right, text(fill: muted, size: 0.95em, {
      (e.dates, e.location).filter(x => x != none).join(" · ")
    })),
  )
  #if e.summary != none { block(above: 0.4em, e.summary) }
  #if e.bullets.len() > 0 { block(above: 0.4em, list(..e.bullets)) }
]

// Skills, languages and interests read better as compact rows.
#let compact-row(e) = block(above: 0.45em)[
  #text(weight: "bold", e.title)#if e.subtitle != none [ (#e.subtitle)]#if e.summary != none [: #e.summary]
]

#align(center)[
  #text(size: 2.1em, weight: "bold", fill: accent, header.name)
  #if header.label != none { v(-0.4em); text(size: 1.15em, fill: muted, header.label) }
  #v(-0.2em)
  #text(size: 0.95em, {
    let items = ()
    if header.location != none { items.push(header.location) }
    for c in header.contacts { items.push(maybe-link(c.text, c.url)) }
    items.join(h(0.5em) + text(fill: muted, "·") + h(0.5em))
  })
]

#if header.summary != none {
  block(above: 1em, header.summary)
}

#for section in view.sections {
  section-title(section.title)
  if section.kind in ("skills", "languages", "interests") {
    for e in section.entries { compact-row(e) }
  } else {
    for e in section.entries { entry(e) }
  }
}
