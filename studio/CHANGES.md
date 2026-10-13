# muhandes: changes from upstream CADCraft

Required by the Apache License 2.0 (section 4b): files we changed carry this notice through
this list.

- Branding (user-visible only, applied by `studio/rebrand.py`): the app name muhandes, the bundle id
  `ps.creative.studio.muhandes`, links to muhandes.studio.creative.ps and this repository. Crate names,
  settings folders and the file format keep their upstream names.
- No Discord buttons; community links point to Creative Studio.
- App icon: the muhandes star on navy (`assets/app-icon/`).
- Removed upstream's ArtCraft brand assets (`docs/brand/`), as their brand terms require of forks.
- README (upstream's is `docs/UPSTREAM-README.md`) and NOTICE: the fork, its credits and the suite.
- Arabic interface: `crates/ui-egui/src/i18n/ar.tsv` (every catalog row; see `studio/ar-work/TODO.md`),
  `i18n/bidi.rs` (right-to-left text stored in visual order), and the Arabic font loaded from the
  system (not bundled; `theme::arabic_font`). `studio/arabic_names.py` puts the Arabic name مهندس in
  the Arabic catalogs.
- Right-to-left layout: `vendor/egui` (egui 0.36.2 with a right-to-left switch, marked `Creative Studio`,
  used through `[patch.crates-io]` in `Cargo.toml`), the switch in `crates/ui-egui/src/lib.rs` and the
  palettes' side in `crates/ui-egui/src/palettes.rs`, and `crates/ui-egui/tests/rtl.rs`. All applied by `studio/rebrand.py`.
- Windows installer: our own UpgradeCode, component GUIDs, registry keys and ProgId
  (`packaging/windows/cadcraft.wxs`), so muhandes installs next to CADCraft and never replaces it.
- Loading screen of the web version: the app's star (`studio/` splash, `apps/cadcraft-web/index.html`).
