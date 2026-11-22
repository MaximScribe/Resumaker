// Base for templates generated from a reference resume. Expects `spec` to be defined
// above this line (Resumaker writes it). Data: /view.json, /options.json.

#let view = json("/view.json")
#let opts = json("/options.json")
#let header = view.header
#let accent = spec.accent
#let muted = luma(95)
#let size = (compact: 9.5pt, normal: 10.5pt, airy: 11pt).at(spec.density)
#let gap = (compact: 0.6em, normal: 0.85em, airy: 1.05em).at(spec.density)
#let fallback = if spec.serif {
  ("Libertinus Serif", "New Computer Modern")
} else {
  ("Inter", "Segoe UI", "Helvetica Neue", "Arial", "DejaVu Sans", "Libertinus Serif")
}
#let side-kinds = ("skills", "languages", "interests")

#set document(title: header.name, author: header.name)
#set page(paper: "a4", margin: (x: 1.6cm, y: 1.4cm))
#set text(font: spec.fonts + fallback, size: size, lang: opts.lang)
#set par(leading: 0.55em, spacing: 0.6em)
#set list(indent: 0.25em, body-indent: 0.45em, spacing: 0.42em)
#show link: set text(fill: accent)

#let maybe-link(body, url) = if url == none { body } else { link(url, body) }

#let section-title(title) = {
  let label = text(weight: "bold", fill: accent, tracking: 0.05em, upper(title))
  if spec.heading == "rule" {
    block(above: gap * 1.5, below: gap * 0.7, width: 100%, inset: (bottom: 3pt),
      stroke: (bottom: 0.6pt + accent.lighten(30%)), sticky: true, label)
  } else if spec.heading == "bar" {
    block(above: gap * 1.5, below: gap * 0.7, inset: (left: 6pt), sticky: true,
      stroke: (left: 2.5pt + accent), label)
  } else if spec.heading == "caps" {
    block(above: gap * 1.5, below: gap * 0.7, sticky: true, label)
  } else {
    block(above: gap * 1.5, below: gap * 0.7, sticky: true,
      text(size: 1.15em, weight: "bold", fill: accent, title))
  }
}

#let meta(e) = (e.dates, e.location).filter(x => x != none).join(" · ")

#let entry(e, compact-kind) = {
  if compact-kind {
    block(above: gap * 0.6)[
      #text(weight: "bold", e.title)#if e.subtitle != none [ (#e.subtitle)]#if e.summary != none [: #e.summary]
    ]
    return
  }
  let head = [
    #text(weight: "bold", maybe-link(e.title, e.url))
    #if e.subtitle != none [ #text(fill: muted)[· #e.subtitle]]
  ]
  let body = [
    #if e.summary != none { block(above: 0.35em, e.summary) }
    #if e.bullets.len() > 0 { block(above: 0.35em, list(..e.bullets)) }
  ]
  if spec.dates == "left" {
    block(above: gap, grid(columns: (3.2cm, 1fr), column-gutter: 0.5cm,
      text(fill: muted, if meta(e) == none { [] } else { meta(e) }),
      [#head #body]))
  } else {
    block(above: gap)[
      #grid(columns: (1fr, auto), column-gutter: 1em, head,
        align(right, text(fill: muted, if meta(e) == none { [] } else { meta(e) })))
      #body
    ]
  }
}

#let ordered = {
  let rank(kind) = {
    let i = spec.order.position(k => k == kind)
    if i == none { 100 } else { i }
  }
  view.sections.sorted(key: s => rank(s.kind))
}

#let contacts = {
  let items = ()
  if header.location != none { items.push(header.location) }
  for c in header.contacts { items.push(maybe-link(c.text, c.url)) }
  items
}

#let name-block(align-to) = align(align-to)[
  #text(size: 2em, weight: "bold", fill: accent, header.name)
  #if header.label != none { linebreak(); text(size: 1.1em, fill: muted, header.label) }
]

#if spec.layout == "sidebar" {
  grid(
    columns: (5.4cm, 1fr),
    column-gutter: 0.8cm,
    [
      #name-block(left)
      #v(gap)
      #set text(size: 0.92em)
      #for c in contacts { block(above: 0.4em, c) }
      #for s in ordered.filter(s => s.kind in side-kinds) {
        section-title(s.title)
        for e in s.entries { entry(e, true) }
      }
    ],
    [
      #if header.summary != none { block(header.summary) }
      #for s in ordered.filter(s => s.kind not in side-kinds) {
        section-title(s.title)
        for e in s.entries { entry(e, false) }
      }
    ],
  )
} else {
  let a = if spec.name-align == "center" { center } else { left }
  name-block(a)
  align(a, text(size: 0.95em, contacts.join(h(0.4em) + text(fill: muted, "·") + h(0.4em))))
  if header.summary != none { block(above: gap * 1.2, header.summary) }
  for s in ordered {
    section-title(s.title)
    for e in s.entries { entry(e, s.kind in side-kinds) }
  }
}
