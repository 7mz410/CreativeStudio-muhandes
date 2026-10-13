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
- Arabic interface (partial): `crates/ui-egui/src/i18n/ar.tsv` (core menus and tools; the rest shows in English,
  see `studio/ar-work/TODO.md`), `i18n/bidi.rs` (right-to-left text stored in visual order), and the Arabic
  font loaded from the system (not bundled; `theme::arabic_font`). `studio/arabic_names.py` puts the Arabic
  name مهندس in the Arabic catalogs.
- Loading screen of the web version: the app's star (`studio/` splash, `apps/cadcraft-web/index.html`).
