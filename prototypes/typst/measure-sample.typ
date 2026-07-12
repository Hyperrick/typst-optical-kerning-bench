#let font = sys.inputs.at("font")
#let sample = sys.inputs.at("sample")
#let ligatures = sys.inputs.at("ligatures", default: "false") == "true"

#context [
  #let metric = measure(text(
    font: font,
    size: 100pt,
    kerning: true,
    ligatures: ligatures,
    sample,
  )).width
  #let optical = measure(text(
    font: font,
    size: 100pt,
    kerning: "optical",
    ligatures: ligatures,
    sample,
  )).width
  #metadata((
    font: font,
    sample: sample,
    metric_width: metric,
    optical_width: optical,
    correction: optical - metric,
  )) <optical-measurement>
]
