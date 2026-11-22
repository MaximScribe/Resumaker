// Cover letter. Data: /letter.json ({ letter, sender, date, lang }).

#let data = json("/letter.json")
#let letter = data.letter
#let sender = data.sender
#let accent = rgb("#1f3a5f")
#let muted = luma(90)

#set document(title: letter.subject, author: sender.name)
#set page(paper: "a4", margin: (x: 2.4cm, top: 2cm, bottom: 2.2cm))
#set text(font: ("Libertinus Serif", "New Computer Modern"), size: 11pt, lang: data.lang)
#set par(justify: true, leading: 0.65em, spacing: 1.1em)

#align(right)[
  #text(size: 1.3em, weight: "bold", fill: accent, sender.name) \
  #text(fill: muted, sender.lines.join(linebreak()))
]

#v(1.2cm)
#let r = letter.recipient
#{
  let lines = (r.at("company", default: none), r.at("contact", default: none), r.at("address", default: none))
    .filter(x => x != none and x != "")
  lines.join(linebreak())
}

#v(0.8cm)
#align(right, data.date)

#v(0.6cm)
#text(weight: "bold", letter.subject)

#letter.salutation

#for p in letter.paragraphs [
  #p

]

#letter.closing

#v(1.2cm)
#sender.name
