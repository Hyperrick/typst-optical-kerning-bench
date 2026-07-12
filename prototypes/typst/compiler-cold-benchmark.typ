#let mode = sys.inputs.at("mode", default: "metric")
#let heading-kerning = if mode == "metric" { true } else { "optical" }
#let body-kerning = if mode == "all" { "optical" } else { true }

#set page(paper: "a4", margin: 16mm)
#set text(font: "Inter", size: 10pt, kerning: body-kerning)

#text(size: 34pt, kerning: heading-kerning)[AVATAR OpenType 10.000]

#lorem(100)

#text(size: 20pt, font: "EB Garamond", kerning: heading-kerning)[
  Goldfish ToTaL WAYFINDER
]

#lorem(100)
