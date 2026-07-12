#let mode = sys.inputs.at("mode", default: "metric")
#let heading-kerning = if mode == "metric" { true } else { "optical" }
#let body-kerning = if mode == "all" { "optical" } else { true }

#set page(paper: "a4", margin: 16mm)
#set text(font: "Inter", size: 10pt, kerning: body-kerning)
#set par(leading: 0.65em)

#for index in range(120) [
  #text(size: 34pt, kerning: heading-kerning)[AVATAR OpenType 10.000]

  #lorem(55)

  #text(size: 20pt, font: "EB Garamond", kerning: heading-kerning)[
    Goldfish ToTaL WAYFINDER
  ]

  #lorem(35)
  #if index < 119 { pagebreak() }
]
