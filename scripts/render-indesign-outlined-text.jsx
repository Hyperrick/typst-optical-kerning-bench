#target indesign

app.scriptPreferences.userInteractionLevel = UserInteractionLevels.NEVER_INTERACT;
app.scriptPreferences.measurementUnit = MeasurementUnits.POINTS;

function fail(message) {
  throw new Error(message);
}

function readTextFile(path) {
  var file = File(path);
  if (!file.exists) fail("Config file does not exist: " + path);
  file.encoding = "UTF-8";
  file.open("r");
  var text = file.read();
  file.close();
  return text;
}

function writeTextFile(path, text) {
  var file = File(path);
  file.encoding = "UTF-8";
  file.open("w");
  file.write(text);
  file.close();
}

function parseJson(text) {
  if (typeof JSON !== "undefined" && JSON.parse) return JSON.parse(text);
  return eval("(" + text + ")");
}

function jsonQuote(value) {
  return "\"" + String(value)
    .replace(/\\/g, "\\\\")
    .replace(/"/g, "\\\"")
    .replace(/\t/g, "\\t")
    .replace(/\r/g, "\\r")
    .replace(/\n/g, "\\n") + "\"";
}

function toJson(value) {
  if (value === null) return "null";
  if (value instanceof Array) {
    var parts = [];
    for (var i = 0; i < value.length; i++) parts.push(toJson(value[i]));
    return "[" + parts.join(",") + "]";
  }
  if (typeof value === "object") {
    var props = [];
    for (var key in value) {
      if (value.hasOwnProperty(key)) props.push(jsonQuote(key) + ":" + toJson(value[key]));
    }
    return "{" + props.join(",") + "}";
  }
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return jsonQuote(value);
}

function scriptArguments() {
  if (typeof arguments !== "undefined" && arguments.length > 0) return arguments;
  return [];
}

function setupDocument(doc) {
  doc.documentPreferences.facingPages = false;
  doc.documentPreferences.pageWidth = "210mm";
  doc.documentPreferences.pageHeight = "297mm";
  doc.viewPreferences.rulerOrigin = RulerOrigin.PAGE_ORIGIN;
  doc.viewPreferences.horizontalMeasurementUnits = MeasurementUnits.POINTS;
  doc.viewPreferences.verticalMeasurementUnits = MeasurementUnits.POINTS;
  doc.zeroPoint = [0, 0];
  try {
    doc.pages[0].marginPreferences.top = 0;
    doc.pages[0].marginPreferences.bottom = 0;
    doc.pages[0].marginPreferences.left = 0;
    doc.pages[0].marginPreferences.right = 0;
    doc.pages[0].marginPreferences.columnCount = 1;
    doc.pages[0].marginPreferences.columnGutter = 0;
  } catch (error) {}
}

function kerningMethod(name) {
  if (name === "metrics" || name === "metric") return "$ID/Metrics";
  if (name === "optical") return "$ID/Optical";
  if (name === "none") return "$ID/None";
  return name;
}

function fontCandidates(config) {
  var family = String(config.fontFamily || "");
  var style = String(config.fontStyle || "");
  var candidates = [];
  if (style.length > 0) {
    candidates.push(family + "\t" + style);
    candidates.push(family + " " + style);
  }
  candidates.push(family + "\tRegular");
  candidates.push(family + "\tRoman");
  candidates.push(family + "\tLight");
  candidates.push(family + "\tExtraLight");
  candidates.push(family);
  return candidates;
}

function fontFor(config) {
  var candidates = fontCandidates(config);
  for (var i = 0; i < candidates.length; i++) {
    try {
      var font = app.fonts.itemByName(candidates[i]);
      if (font.isValid) return font;
    } catch (error) {}
  }

  var familyNeedle = String(config.fontFamily || "").toLowerCase();
  for (var j = 0; j < app.fonts.length; j++) {
    try {
      var current = app.fonts[j];
      var haystack = String(current.name).toLowerCase() + " " +
        String(current.fontFamily).toLowerCase() + " " +
        String(current.fontStyleName).toLowerCase();
      if (haystack.indexOf(familyNeedle) >= 0) return current;
    } catch (error2) {}
  }
  fail("Font is not available in InDesign: " + config.fontFamily);
}

function configureText(doc, story, config) {
  var font = fontFor(config);
  var ligatures = Boolean(config.ligatures);
  story.appliedFont = font;
  story.pointSize = Number(config.pointSize || 12);
  story.kerningMethod = kerningMethod(config.kerning || "optical");
  story.ligatures = ligatures;
  try {
    story.texts[0].ligatures = ligatures;
  } catch (error) {}
  try {
    story.characters.everyItem().ligatures = ligatures;
  } catch (error2) {}
  try {
    story.texts[0].opentypeFeatures = ligatures
      ? [["liga", 1], ["clig", 1]]
      : [["liga", 0], ["clig", 0]];
  } catch (error3) {}
  applyOpenTypeFlags(story, ligatures);
  try {
    applyOpenTypeFlags(story.texts[0], ligatures);
  } catch (error4) {}
  try {
    applyOpenTypeFlags(story.characters.everyItem(), ligatures);
  } catch (error5) {}
  if (!ligatures) {
    disableLigaturesViaMenu(story);
  }
  story.tracking = Number(config.tracking || 0);
  story.horizontalScale = 100;
  story.verticalScale = 100;
  story.hyphenation = false;
  story.justification = Justification.LEFT_ALIGN;
  applyBenchmarkCharacterStyle(doc, story, ligatures);
  return font;
}

function applyBenchmarkCharacterStyle(doc, story, ligatures) {
  try {
    var style = doc.characterStyles.add({name: "Optikern Character Settings"});
    style.ligatures = true;
    style.ligatures = ligatures;
    style.otfDiscretionaryLigature = false;
    style.otfContextualAlternate = ligatures;
    story.texts[0].appliedCharacterStyle = style;
    applyOpenTypeFlags(story.texts[0], ligatures);
    applyOpenTypeFlags(story.characters.everyItem(), ligatures);
  } catch (error) {}
}

function disableLigaturesViaMenu(story) {
  try {
    app.select(story.texts[0]);
    var action = app.menuActions.item("$ID/Ligatures");
    if (action.isValid && action.checked) action.invoke();
    app.select(null);
  } catch (error) {
    try {
      app.select(null);
    } catch (error2) {}
  }
}

function applyOpenTypeFlags(target, ligatures) {
  try {
    target.properties = {
      ligatures: ligatures,
      otfDiscretionaryLigature: false,
      otfContextualAlternate: ligatures
    };
  } catch (error) {}
}

function readFontProperty(font, name) {
  try {
    if (font && font[name] !== undefined) return String(font[name]);
  } catch (error) {}
  return "";
}

function fontInfo(font) {
  return {
    name: readFontProperty(font, "name"),
    fullName: readFontProperty(font, "fullName"),
    fontFamily: readFontProperty(font, "fontFamily"),
    fontStyleName: readFontProperty(font, "fontStyleName"),
    postscriptName: readFontProperty(font, "postscriptName"),
    fontType: readFontProperty(font, "fontType"),
    location: readFontProperty(font, "location")
  };
}

function unionBounds(a, b) {
  if (a === null) return [Number(b[0]), Number(b[1]), Number(b[2]), Number(b[3])];
  return [
    Math.min(Number(a[0]), Number(b[0])),
    Math.min(Number(a[1]), Number(b[1])),
    Math.max(Number(a[2]), Number(b[2])),
    Math.max(Number(a[3]), Number(b[3]))
  ];
}

function collectVisibleBounds(items) {
  var bounds = null;
  for (var i = 0; i < items.length; i++) {
    try {
      bounds = unionBounds(bounds, items[i].visibleBounds);
    } catch (error) {}
  }
  if (bounds === null) fail("No visible outline bounds found.");
  return bounds;
}

function moveItemsToOrigin(items, bounds, paddingPt) {
  var dx = paddingPt - Number(bounds[1]);
  var dy = paddingPt - Number(bounds[0]);
  for (var i = 0; i < items.length; i++) {
    try {
      items[i].move(undefined, [dx, dy]);
    } catch (error) {}
  }
}

function outputInddPath(config) {
  if (config.outputIndd) return String(config.outputIndd);
  if (config.outputPdf) return String(config.outputPdf).replace(/\.pdf$/i, ".indd");
  fail("Config needs outputPdf or outputIndd.");
}

function closeDocumentAtPath(path) {
  var target = File(path).fsName;
  for (var i = app.documents.length - 1; i >= 0; i--) {
    try {
      var doc = app.documents[i];
      if (doc.saved && doc.fullName && File(doc.fullName).fsName === target) {
        doc.close(SaveOptions.NO);
      }
    } catch (error) {}
  }
}

function build(config) {
  if (!config.text) fail("Config needs text.");
  if (!config.fontFamily) fail("Config needs fontFamily.");

  var inddPath = outputInddPath(config);
  var pdfPath = config.outputPdf ? String(config.outputPdf) : inddPath.replace(/\.indd$/i, ".pdf");
  var jsonPath = config.outputJson ? String(config.outputJson) : pdfPath.replace(/\.pdf$/i, ".json");
  var paddingPt = Number(config.paddingPt || 0);

  closeDocumentAtPath(inddPath);
  var doc = app.documents.add();
  setupDocument(doc);
  doc.save(File(inddPath));
  doc.close(SaveOptions.YES);

  doc = app.open(File(inddPath));
  setupDocument(doc);

  var page = doc.pages[0];
  var frame = page.textFrames.add();
  var frameHeight = Math.max(240, Number(config.pointSize || 12) * 4);
  var frameWidth = Math.max(1440, String(config.text).length * Number(config.pointSize || 12) * 2);
  frame.geometricBounds = ["0pt", "0pt", frameHeight + "pt", frameWidth + "pt"];
  frame.contents = String(config.text);
  var appliedFont = configureText(doc, frame.parentStory, config);
  try {
    frame.parentStory.recompose();
  } catch (error) {}
  try {
    doc.recompose();
  } catch (error2) {}

  var outlined = frame.createOutlines(true);
  var items = outlined instanceof Array ? outlined : [outlined];
  var bounds = collectVisibleBounds(items);
  moveItemsToOrigin(items, bounds, paddingPt);

  var widthPt = Number(bounds[3]) - Number(bounds[1]) + paddingPt * 2;
  var heightPt = Number(bounds[2]) - Number(bounds[0]) + paddingPt * 2;
  page.resize(
    CoordinateSpaces.INNER_COORDINATES,
    AnchorPoint.TOP_LEFT_ANCHOR,
    ResizeMethods.REPLACING_CURRENT_DIMENSIONS_WITH,
    [widthPt, heightPt]
  );

  doc.exportFile(ExportFormat.PDF_TYPE, File(pdfPath), false);
  doc.save();

  var sidecar = {
    schemaVersion: 1,
    renderer: "indesign",
    text: String(config.text),
    fontFamily: String(config.fontFamily),
    fontStyle: String(config.fontStyle || ""),
    appliedFont: fontInfo(appliedFont),
    pointSize: Number(config.pointSize || 12),
    kerning: String(config.kerning || "optical"),
    ligatures: Boolean(config.ligatures),
    paddingPt: paddingPt,
    pageBoundsPt: [0, 0, heightPt, widthPt],
    inkBoundsPt: [paddingPt, paddingPt, heightPt - paddingPt, widthPt - paddingPt],
    outputPdf: pdfPath,
    outputIndd: inddPath
  };
  writeTextFile(jsonPath, toJson(sidecar));
  doc.close(SaveOptions.YES);
}

var argv = scriptArguments();
var configPath = typeof OPTIKERN_CONFIG_PATH !== "undefined" ? OPTIKERN_CONFIG_PATH : null;
if (configPath === null && argv.length > 0) configPath = String(argv[0]);
if (configPath === null) fail("Pass config JSON path as first script argument.");
build(parseJson(readTextFile(configPath)));
