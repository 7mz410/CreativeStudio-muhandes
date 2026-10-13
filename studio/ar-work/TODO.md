# Arabic catalog: work left

Done (crates/ui-egui/src/i18n/ar.tsv): the 11 menu headings, the menu items and core tools of the
Draw, Modify and Insert menus (about 84 rows), the About and Quit items, and the app name (muhandes = مهندس).

Left in English, to translate next (the shipped catalogs have about 1000 rows each):
- Command-line prompts and their keywords, and the dialogs (Options, Plot, Layers, Blocks, Parameters).
- Status-bar and tooltip text, error messages, the Command Reference, About text, and the Help menu.
- Plural rows (`@plural`): none in ar.tsv yet, so the plural rule is still the English one.
- Layout is still left to right. Arabic words are stored in visual order (see bidi.rs), so they read
  correctly, but panels, menus and toolbars do not mirror.
