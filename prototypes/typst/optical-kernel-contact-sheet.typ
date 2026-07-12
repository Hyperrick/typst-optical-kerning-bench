#set page(paper: "a4", flipped: true, margin: 12mm)
#set text(font: "Optikern Inter NoLiga", size: 9pt)
#set par(leading: 0.55em)

#let fonts = (
  (label: "EB Garamond", family: "Optikern EB Garamond NoLiga"),
  (label: "Libre Baskerville", family: "Optikern Libre Baskerville NoLiga"),
  (label: "Inter", family: "Optikern Inter NoLiga"),
  (label: "Pacifico", family: "Optikern Pacifico NoLiga"),
  (label: "Comic Neue", family: "Optikern Comic Neue NoLiga"),
)

#let samples = (
  "Goldfish",
  "AVATAR",
  "WAVY",
  "ToTaL",
  "OpenType",
  "10.000",
)

#let specimen(font, sample, mode) = text(
  font: font,
  size: 34pt,
  kerning: mode,
  ligatures: false,
  sample,
)

#for sample in samples {
  block(width: 100%, height: 100%, breakable: false)[
    #text(size: 18pt, weight: "bold", sample)
    #v(3mm)

    #grid(
      columns: (38mm, 1fr, 1fr),
      column-gutter: 6mm,
      [*Font*],
      [*Typst Metric*],
      [*Typst Optical Prototype*],
    )
    #line(length: 100%, stroke: 0.4pt + rgb("b8b8b8"))
    #v(2mm)

    #for font in fonts {
      block(height: 29mm, width: 100%)[
        #grid(
          columns: (38mm, 1fr, 1fr),
          column-gutter: 6mm,
          align(horizon, font.label),
          align(left + horizon, specimen(font.family, sample, true)),
          align(left + horizon, specimen(font.family, sample, "optical")),
        )
      ]
    }
  ]
}
