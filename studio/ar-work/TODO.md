# Arabic: work left

Done (crates/ui-egui/src/i18n/ar.tsv): every catalog row (about 1230, with the core menus and tools
translated first). Plural rows use the two-form English rule (`plural_en`). Terms follow the
Arabic CAD usage of the other catalogs: layer = طبقة, block = كتلة, dimension = قياس, attribute = سمة,
layout = تخطيط, viewport = منفذ عرض, hatch = تظليل.

Done (right to left, vendor/egui, see CHANGES.md): when the language is Arabic, `egui::set_rtl` turns on
for the frame, so rows, grids, columns and menus run from the right. The palettes (right side) move to
the left, with their tab icons and collapse button mirrored. The Arabic font is the system font
(`theme::arabic_font`: GeezaPro on macOS, Arial, Noto Sans Arabic or DejaVu); no font is bundled.

Left (not checked on screen in every state):
- The canvas-drawn parts: the status bar, the file tabs and the start page are not mirrored.
- The toolbar and command line are not mirrored individually; check them with a screenshot.
- Dialogs (Options, Plot, Layers, Blocks, Parameters) keep their fixed-coordinate parts
  (buttons placed by hand) on the left.
- Plural rows (`@plural`) follow the English rule; Arabic has six plural forms in CLDR, so
  `count()` would need a plural function for `ar` before a count reads correctly for 3–10.
- Arabic words are stored in visual order (`i18n/bidi.rs`), so a new Arabic string must go through the
  catalog, not be typed into code.
