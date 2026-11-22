// Academic: a serif CV that leads with education and publications.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let order = ("education", "publications", "work", "projects", "awards", "certificates",
  "volunteer", "skills", "languages", "interests")

#set document(title: header.name, author: header.name)
#set page(paper: "a4", margin: (x: 2.2cm, y: 2cm), numbering: "1", number-align: right)
#set text(font: ("Libertinus Serif", "New Computer Modern"), size: 11pt, lang: opts.lang)
#set par(justify: true, leading: 0.6em, spacing: 0.7em)
#set list(indent: 0.4em, spacing: 0.5em)
#show link: set text(fill: rgb("#7c2d12"))

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }

#align(center)[
  #text(size: 1.8em, smallcaps(header.name))
  #if header.label != none { linebreak(); text(style: "italic", header.label) }
  #linebreak()
  #text(size: 0.92em, {
    let items = ()
    if header.location != none { items.push(header.location) }
    for c in header.contacts { items.push(maybe-link(c.text, c.url)) }
    items.join("  |  ")
  })
]

#if header.summary != none { block(above: 1.2em, header.summary) }

#let sections = order.map(k => view.sections.filter(s => s.kind == k)).flatten()

#for section in sections {
  block(above: 1.4em, below: 0.7em, sticky: true, text(size: 1.1em, smallcaps(section.title)))
  for e in section.entries {
    block(above: 0.7em)[
      #grid(
        columns: (1fr, auto),
        column-gutter: 1em,
        [*#maybe-link(e.title, e.url)*#if e.subtitle != none [, #emph(e.subtitle)]],
        (e.dates, e.location).filter(x => x != none).join(", "),
      )
      #if e.summary != none { block(above: 0.35em, e.summary) }
      #if e.bullets.len() > 0 { block(above: 0.35em, list(..e.bullets)) }
    ]
  }
}
