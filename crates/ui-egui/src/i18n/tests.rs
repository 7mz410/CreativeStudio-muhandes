use std::collections::BTreeSet;

use super::*;
use crate::{CadApp, Services};
use cadcraft_engine::Session;
use serde_json::json;

fn placeholders(text: &str) -> BTreeSet<&str> {
    text.split('{').skip(1).filter_map(|s| s.split_once('}').map(|(key, _)| key)).collect()
}

/// Languages with a catalog (every one but English).
fn translated() -> impl Iterator<Item = &'static Language> {
    LANGUAGES.iter().filter(|l| !l.source.is_empty())
}

#[test]
fn catalog_is_valid_and_preserves_placeholders_and_ellipses() {
    for language in &LANGUAGES {
        let (catalog, errors) = parse(language.source);
        assert!(errors.is_empty(), "{}: {errors:?}", language.code);
        for ((context, key), value) in catalog {
            assert!(["", "menu", "@plural"].contains(&context), "unknown context: {context}");
            let forms: Vec<_> = if context == "@plural" { value.split('|').collect() } else { vec![value] };
            if context == "@plural" {
                assert_eq!(forms.len(), language.plural_forms, "{}: {key}", language.code);
            }
            for form in forms {
                assert_eq!(placeholders(key), placeholders(form), "{key}");
                for suffix in ["...", "…"] {
                    assert_eq!(key.ends_with(suffix), form.ends_with(suffix), "{key}");
                }
            }
        }
        assert_eq!(tr(language.code, "a future untranslated label"), "a future untranslated label");
    }
    let (parsed, errors) = parse("broken\n\tOpen\tВідкрити\n\tOpen\tВідкрити\n\tEmpty\t\n");
    assert_eq!(parsed.len(), 1);
    assert_eq!(errors.len(), 3);
    assert_eq!(tr("unknown", "Open"), "Open");
}

/// What each language shows in English for now. Missing translations never fail a build:
/// `cargo xtask i18n`, a step of `cargo xtask ci`, runs this to print the list.
#[test]
#[ignore = "a report, printed by `cargo xtask i18n`"]
fn translation_gaps() {
    for language in translated() {
        let mut missing: BTreeSet<String> = missing_commands_menus_tools_and_snap_labels(language.code).into_iter().collect();
        missing.extend(missing_marked_ui_literals(language.code));
        if missing.is_empty() {
            println!("{}: nothing missing", language.code);
            continue;
        }
        println!("{}: {} strings fall back to English", language.code, missing.len());
        for source in &missing {
            println!("  {source}");
        }
    }
}

fn missing_commands_menus_tools_and_snap_labels(language: &str) -> Vec<String> {
    let catalog = catalog(language).unwrap();
    let mut sources = BTreeSet::new();
    sources.extend(crate::menus::MENUS.iter().copied());
    for c in cadcraft_engine::command_specs() {
        sources.insert(c.label);
        sources.extend(c.menu.iter().copied());
    }
    for (_, label, path, _) in crate::menus::UI_COMMANDS {
        sources.insert(*label);
        sources.extend(path.iter().copied());
    }
    for groups in [crate::palettes::DRAFTING, crate::palettes::MODELING] {
        for (name, large, small) in groups {
            sources.insert(*name);
            sources.extend(large.iter().chain(*small).map(|(_, _, label)| *label));
        }
    }
    sources.extend(cadcraft_engine::snap::mode::ALL.iter().map(|(_, name)| *name));
    sources.into_iter().filter(|s| !s.is_empty() && !catalog.contains_key(&("", *s))).map(str::to_string).collect()
}

fn missing_marked_ui_literals(language: &str) -> BTreeSet<String> {
    let catalog = catalog(language).unwrap();
    let sources = [
        include_str!("../xrefs.rs"),
        include_str!("../canvas.rs"),
        include_str!("../chrome.rs"),
        include_str!("../cmdline.rs"),
        include_str!("../credits.rs"),
        include_str!("../palettes/quickcalc.rs"),
        include_str!("../palettes/patterns.rs"),
        include_str!("../dialogs.rs"),
        include_str!("../hatchdlg.rs"),
        include_str!("../aliasdlg.rs"),
        include_str!("../mtext_editor.rs"),
        include_str!("../layers.rs"),
        include_str!("../canvas/compare.rs"),
        include_str!("../canvas/blockedit.rs"),
        include_str!("../bedit.rs"),
        include_str!("../palettes.rs"),
        include_str!("../parametric.rs"),
        include_str!("../quick.rs"),
        include_str!("../vpscale.rs"),
        include_str!("../options/dialog.rs"),
    ];
    let mut missing = BTreeSet::new();
    for source in sources {
        for marker in ["crate::tl!(", "crate::tf!("] {
            for rest in source.split(marker).skip(1) {
                // These macros require a quoted literal. Use JSON's string decoding for escapes.
                let rest = rest.trim_start();
                let mut escape = false;
                let end = rest
                    .char_indices()
                    .skip(1)
                    .find_map(|(i, c)| {
                        if escape {
                            escape = false;
                            None
                        } else if c == '\\' {
                            escape = true;
                            None
                        } else if c == '"' {
                            Some(i + 1)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                let key: String = serde_json::from_str(&rest[..end]).unwrap();
                // Coordinate axes, glyph buttons and product names are not translated.
                if !["X", "Y", "B", "CADCraft", "muhandes"].contains(&key.as_str()) && !catalog.contains_key(&("", key.as_str())) {
                    missing.insert(key);
                }
            }
        }
    }
    missing
}

#[test]
fn locale_and_saved_preference_resolution() {
    for tag in ["uk", "uk-UA", "uk_UA.UTF-8", "UK-ua", " uk-UA ", "uk_UA@euro"] {
        assert_eq!(locale_language(tag), Some("uk"), "{tag}");
    }
    for tag in ["de", "de-DE", "de_DE.UTF-8", "DE-at", "de-CH"] {
        assert_eq!(locale_language(tag), Some("de"), "{tag}");
    }
    for (tag, code) in [("ru", "ru"), ("ru-RU", "ru"), ("ru_RU.UTF-8", "ru"), ("fr", "fr"), ("fr-FR", "fr"), ("fr_CA.UTF-8", "fr")] {
        assert_eq!(locale_language(tag), Some(code), "{tag}");
    }
    for tag in ["pt", "pt-BR", "pt_BR.UTF-8", "PT-br", "pt-PT"] {
        assert_eq!(locale_language(tag), Some("pt-BR"), "{tag}");
    }
    for tag in ["es", "es-ES", "es_MX.UTF-8", "ES-ar"] {
        assert_eq!(locale_language(tag), Some("es"), "{tag}");
    }
    for (code, tags) in [
        ("cs", &["cs", "cs-CZ", "cs_CZ.UTF-8", "CS-cz", " cs-CZ ", "cs_CZ@euro"][..]),
        ("it", &["it", "it-IT", "it_IT.UTF-8", "IT-ch", " it-CH ", "it_IT@euro"]),
        ("ko", &["ko", "ko-KR", "ko_KR.UTF-8", "KO-kr", " ko-KR ", "ko_KR@euro"]),
        ("sv", &["sv", "sv-SE", "sv_SE.UTF-8", "SV-se", " sv-SE ", "sv-FI"]),
        ("hu", &["hu", "hu-HU", "hu_HU.UTF-8", "HU-hu", " hu-HU "]),
        ("tr", &["tr", "tr-TR", "tr_TR.UTF-8", "TR-tr", " tr-TR ", "tr-CY"]),
        ("el", &["el", "el-GR", "el_GR.UTF-8", "EL-cy"]),
        ("zh", &["zh", "zh-CN"]),
    ] {
        for tag in tags {
            assert_eq!(locale_language(tag), Some(code), "{tag}");
        }
    }
    // Look-alike codes, three-letter codes and endonyms pick nothing: `se` is Northern Sami.
    let look_alikes = ["ua", "ukulele", "pl-PL", "", "garbage", "💙", "ptolemy", "cz", "csharp", "csomething", "italic"];
    let others = ["kr", "korean", "kotlin", "se", "swe", "svenska", "hun", "magyar", "tur", "türkçe", "elvish"];
    for tag in look_alikes.into_iter().chain(others) {
        assert_eq!(locale_language(tag), None, "{tag}");
    }
    assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), "pt-BR".into()]), "pt-BR");
    assert_eq!(Preference::Auto.resolve(&["fr-FR".into(), "pt-BR".into()]), "fr");
    assert_eq!(Preference::Auto.resolve(&["es-ES".into(), "pt-BR".into()]), "es");
    let locales = vec!["pl-PL".into(), "uk-UA".into(), "en-US".into()];
    assert_eq!(Preference::Auto.resolve(&locales), "uk");
    assert_eq!(Preference::English.resolve(&locales), "en");
    assert_eq!(Preference::Auto.resolve(&["en-US".into(), "uk-UA".into()]), "en");
    assert_eq!(Preference::Auto.resolve(&[]), "en");
    assert_eq!(Preference::restore(None), Preference::Auto);
    assert_eq!(Preference::restore(Some("obsolete")), Preference::Auto);
    for preference in Preference::ALL {
        assert_eq!(Preference::restore(Some(preference.code())), preference);
        let saved = serde_json::to_string(&preference).unwrap();
        assert_eq!(saved, format!("\"{}\"", preference.code()));
        assert_eq!(serde_json::from_str::<Preference>(&saved).unwrap(), preference);
        if preference == Preference::Auto {
            continue;
        }
        assert_eq!(Preference::parse(preference.code()), Some(preference));
        // A chosen language wins over the system's; Auto takes the first system language there is.
        assert_eq!(preference.resolve(&["uk-UA".into()]), preference.code());
        if preference != Preference::English {
            assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), preference.code().into(), "en-US".into()]), preference.code());
        }
        // Saved with the host's other preferences; a bad saved value keeps the current choice.
        let mut a = CadApp::new(Session::new(), Services::default());
        a.ui.interface_language = preference;
        let mut b = CadApp::new(Session::new(), Services::default());
        b.load_prefs(&a.prefs_json());
        assert_eq!(b.ui.interface_language, preference);
        b.load_prefs(r#"{"interfaceLanguage":"klingon"}"#);
        assert_eq!(b.ui.interface_language, preference);
    }
    assert!(serde_json::from_str::<Preference>("\"unsupported\"").is_err());
    assert_eq!(serde_json::from_value::<crate::UiState>(json!({})).unwrap().interface_language, Preference::Auto);
}

#[test]
fn plural_rules_pick_the_cldr_form() {
    let uk = [0, 1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111, 114, u64::MAX].map(plural_uk);
    assert_eq!(uk, [2, 0, 1, 1, 2, 2, 2, 2, 0, 1, 2, 0, 2, 2, 2]);
    assert_eq!([0, 1, 2, 5, 21, u64::MAX].map(plural_pt), [0, 0, 1, 1, 1, 1]);
    assert_eq!([0, 1, 2, 4, 5, 11, 21, 22, 25, u64::MAX].map(plural_cs), [2, 0, 1, 1, 2, 2, 2, 2, 2, 2]);
    for language in &LANGUAGES {
        for n in [0, 1, 2, 3, 5, 11, 21, 22, 101, 111, u64::MAX] {
            assert!((language.plural)(n) < language.plural_forms, "{}: {n}", language.code);
        }
    }
}

#[test]
fn counts_and_templates_follow_each_catalog_and_keep_inserted_text() {
    for language in &LANGUAGES {
        with_language(language.code, || {
            for (one, other) in [("{n} contributor", "{n} contributors"), ("{n} commit", "{n} commits")] {
                for n in [0, 1, 2, 3, 5, 11, 21, 22, 101, u64::MAX] {
                    let form =
                        catalog(language.code).and_then(|c| c.get(&("@plural", one))).and_then(|forms| forms.split('|').nth((language.plural)(n)));
                    let expected = form.unwrap_or(if n == 1 { one } else { other }).replace("{n}", &n.to_string());
                    assert_eq!(count(n, one, other), expected, "{}: {n}", language.code);
                }
            }
            // Inserted values are never looked up or substituted again.
            assert_eq!(format("Current: {color}", &[("color", "{n} Open".into())]), t("Current: {color}").replace("{color}", "{n} Open"));
            assert_eq!(format("{b}/{a}", &[("a", "А".into()), ("b", "Б".into())]), "Б/А");
        });
    }
    with_language("uk", || {
        let layers = t("Layers");
        with_language("en", || assert_eq!(t("Layers"), "Layers"));
        assert_eq!(t("Layers"), layers);
    });
    assert_eq!(t("Layers"), "Layers", "language must not leak out of a render scope");
    assert_eq!(count(2, "{n} contributor", "{n} contributors"), "2 contributors");
}

#[test]
fn the_wcs_pill_draws_its_arrow_instead_of_a_glyph_a_font_may_lack() {
    // #1541: "WCS ⌄" showed a box where the UI font had no "⌄"; the chevron is now drawn as
    // lines, so no catalog and no language paints the glyph.
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    for language in &LANGUAGES {
        assert!(!language.source.contains('\u{2304}'), "{}.tsv still has the \u{2304} glyph", language.code);
        app.run("ui.language", json!({ "lang": language.code })).unwrap();
        app.logic(&ctx);
        let label = tr(app.language(), "WCS");
        let text = render(&mut app, &ctx);
        assert!(text.lines().any(|l| l == label), "{}: the pill reads {label}", language.code);
        assert!(!text.contains('\u{2304}'), "{}: no \u{2304} glyph is painted", language.code);
    }
}

fn painted_text(shapes: &[egui::epaint::ClippedShape]) -> String {
    fn collect(shape: &egui::Shape, text: &mut String) {
        match shape {
            egui::Shape::Text(s) => {
                text.push_str(s.galley.text());
                text.push('\n');
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, text);
                }
            }
            _ => {}
        }
    }
    let mut text = String::new();
    for shape in shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

fn render(app: &mut CadApp, ctx: &egui::Context) -> String {
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
    // Windows fade in; a second pass makes their text visible without a time dependency.
    let mut text = String::new();
    for _ in 0..2 {
        let mut output = ctx.run_ui(input.clone(), |ui| app.ui(ui));
        text = painted_text(&output.shapes);
        output.textures_delta.clear();
    }
    text
}

#[test]
fn live_switch_translates_the_shell_without_mutating_drawing_or_automation() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.in_window_menu = true;
    app.session.state_mut().unwrap().title = "Open".into();
    let ctx = egui::Context::default();
    app.logic(&ctx);
    assert!(render(&mut app, &ctx).contains("Properties"));
    let commands = crate::control::all_commands(&app);
    let (menu_request, _) = crate::ControlRequest::new("ui.menu.list", json!({}));
    let crate::control::Outcome::Done(menu_before) = crate::control::handle(&mut app, &ctx, &menu_request) else { panic!("expected menu reply") };
    app.start("line");
    let prompt = crate::control::cmdline_state(&app);
    for language in translated() {
        app.run("ui.language", json!({ "lang": language.code })).unwrap();
        app.logic(&ctx);
        // Without a font for its script, a language stays chosen and the interface shows English.
        let shown = app.language();
        assert_eq!(shown, if crate::theme::can_show(language.code) { language.code } else { "en" });
        let text = render(&mut app, &ctx);
        // Fonts are a frame-only concern: the render above installs them.
        assert_eq!(app.fonts_language, Some(shown));
        for source in ["Properties", "File"] {
            assert!(text.contains(tr(shown, source)), "{}: {source}: {text}", language.code);
        }
        assert!(text.contains("Open"), "the drawing title must remain unchanged");
        assert_eq!(crate::control::cmdline_state(&app), prompt);
        assert_eq!(crate::control::all_commands(&app), commands);
        let crate::control::Outcome::Done(menu_after) = crate::control::handle(&mut app, &ctx, &menu_request) else { panic!("expected menu reply") };
        assert_eq!(menu_before, menu_after);
        assert_eq!(app.session.state().unwrap().title, "Open");
        assert_eq!(t("Properties"), "Properties", "language must not leak out of the render scope");
        assert!(app.run("ui.language", json!({"lang":"bad"})).is_err());
        app.run("ui.resetpalettes", json!({})).unwrap();
        assert_eq!(app.language(), shown);
    }
    // The general ui.set route saves the same preference and installs the same fonts.
    for code in ["ko", "zh", "en"] {
        let (request, _) = crate::ControlRequest::new("ui.set", json!({ "interfaceLanguage": code }));
        let _ = crate::control::handle(&mut app, &ctx, &request);
        assert_eq!(app.ui.interface_language, Preference::parse(code).unwrap());
        app.logic(&ctx);
        render(&mut app, &ctx);
        assert_eq!(app.fonts_language, Some(app.language()));
    }
    assert!(render(&mut app, &ctx).contains("Properties"));
    // The menu tree used by the control channel keeps its original labels and IDs.
    assert_eq!(crate::menus::tree(&app).first().unwrap().0, "File");
}

#[test]
fn dialogs_render_in_every_language_and_keep_their_source_ids() {
    for language in translated() {
        for (dialog, heading) in [
            ("dsettings", "Drafting Settings"),
            ("about", "About muhandes"),
            ("commands", "Command Reference"),
            ("blocks", "Blocks"),
            ("layers", "Layer Properties Manager"),
            ("qselect", "Quick Select"),
            ("parameters", "Parameters Manager"),
            ("language", "Interface Language"),
            ("hatch", "Hatch and Gradient"),
            ("options:preferences", "User Preferences"),
        ] {
            let mut app = CadApp::new(Session::new(), Services::default());
            app.ui.interface_language = Preference::parse(language.code).unwrap();
            app.ui.dialog = Some(dialog.into());
            let ctx = egui::Context::default();
            app.logic(&ctx);
            let text = render(&mut app, &ctx);
            assert!(text.contains(tr(app.language(), heading)), "{}: {dialog}: {text}", language.code);
            assert_eq!(app.ui.dialog.as_deref(), Some(dialog));
        }
    }
}

/// The letters of every catalog have glyphs in both UI families. Latin, Cyrillic and Greek come
/// from egui's bundled fonts alone, as in the web build; the scripts they lack need a font that
/// `install_fonts` finds, and are checked only where one is there.
#[test]
fn interface_fonts_cover_every_letter_in_each_catalog() {
    for language in translated() {
        let extra_font = ["ko", "zh", "ar"].contains(&language.code);
        if extra_font && !crate::theme::can_show(language.code) {
            continue;
        }
        let ctx = egui::Context::default();
        if extra_font {
            crate::theme::install_fonts(&ctx, language.code);
        }
        let mut output = ctx.run_ui(Default::default(), |_| {});
        output.textures_delta.clear();
        let letters: BTreeSet<char> = language
            .source
            .lines()
            .filter(|line| !line.starts_with('#'))
            .filter_map(|line| line.split('\t').nth(2))
            .flat_map(str::chars)
            .filter(|c| !c.is_ascii() && c.is_alphabetic())
            .collect();
        let missing = ctx.fonts_mut(|view| {
            let mut missing = Vec::new();
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                // In egui 0.36, has_glyph can report a false negative for glyphs in the same
                // face as the replacement character. Check the family's actual character maps.
                let chars = view.characters(&family);
                for c in &letters {
                    if !chars.contains_key(c) {
                        missing.push(format!("{family:?}: {c} (U+{:04X})", u32::from(*c)));
                    }
                }
            }
            missing
        });
        assert!(missing.is_empty(), "{}: missing glyphs: {}", language.code, missing.join(", "));
    }
    for tag in ["cz", "csharp", "csomething"] {
        assert_eq!(locale_language(tag), None, "{tag}");
    }
    assert_eq!(Preference::parse("cs"), Some(Preference::Czech));
    assert_eq!(Preference::Czech.name(), "Čeština");
    assert_eq!(Preference::Czech.resolve(&["en-US".into()]), "cs");
    assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), "cs-CZ".into(), "en-US".into()]), "cs");
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.interface_language = Preference::Czech;
    let mut restored = CadApp::new(Session::new(), Services::default());
    restored.load_prefs(&app.prefs_json());
    assert_eq!(restored.ui.interface_language, Preference::Czech);
    with_language("cs", || {
        for (n, form) in [(0, 2), (1, 0), (2, 1), (4, 1), (5, 2), (11, 2), (21, 2), (22, 2), (25, 2), (u64::MAX, 2)] {
            assert_eq!(plural_cs(n), form, "{n}");
            let contributor = ["přispěvatel", "přispěvatelé", "přispěvatelů"][form];
            let commit = ["commit", "commity", "commitů"][form];
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} {contributor}"));
            assert_eq!(count(n, "{n} commit", "{n} commits"), format!("{n} {commit}"));
        }
        assert_eq!(format("Current: {color}", &[("color", "{n} Open".into())]), "Aktuální: {n} Open");
    });
}

#[test]
fn czech_menu_headings_keep_object_labels_and_other_catalogs() {
    with_language("cs", || {
        assert_eq!(menu("View"), "Zobrazení");
        assert_eq!(t("View"), "Pohled");
        assert_eq!(menu("Dimension"), "Kótování");
        assert_eq!(t("Dimension"), "Kóta");
        assert_eq!(menu("Draw"), "Kreslení");
        assert_eq!(menu("Modify"), "Modifikace");
        assert_eq!(menu("Edit"), "Úpravy");
        assert_eq!(menu("File"), "Soubor");
    });
    for language in ["en", "uk", "de", "es", "pt-BR", "unknown"] {
        for source in crate::menus::MENUS {
            assert_eq!(tr_menu(language, source), tr(language, source));
        }
    }
}

#[test]
fn live_czech_switch_preserves_drawing_command_and_saved_choice() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.in_window_menu = true;
    app.session.state_mut().unwrap().title = "Open".into();
    let ctx = egui::Context::default();
    app.logic(&ctx);
    assert!(render(&mut app, &ctx).contains("Properties"));
    app.start("line");
    let prompt = crate::control::cmdline_state(&app);
    let commands = crate::control::all_commands(&app);
    app.run("ui.language", json!({"lang":"cs"})).unwrap();
    let text = render(&mut app, &ctx);
    for label in ["Vlastnosti", "Soubor", "Zobrazení", "Kreslení", "Kótování", "Určete první bod"] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    assert_eq!(app.language(), "cs");
    assert_eq!(app.session.state().unwrap().title, "Open");
    assert_eq!(crate::control::cmdline_state(&app), prompt);
    assert_eq!(crate::control::all_commands(&app), commands);
    assert_eq!(t("Properties"), "Properties");
    app.run("ui.resetpalettes", json!({})).unwrap();
    assert_eq!(app.language(), "cs");
    app.run("ui.language", json!({"lang":"en"})).unwrap();
    assert!(render(&mut app, &ctx).contains("Properties"));
}

#[test]
fn czech_dialogs_render_with_stable_ids() {
    for (dialog, heading) in [
        ("dsettings", "Nastavení kreslení"),
        ("about", "O aplikaci muhandes"),
        ("commands", "Přehled příkazů"),
        ("blocks", "Bloky"),
        ("layers", "Správce vlastností hladin"),
        ("qselect", "Rychlý výběr"),
        ("parameters", "Správce parametrů"),
        ("language", "Čeština"),
        ("options:preferences", "Uživatelské předvolby"),
    ] {
        let mut app = CadApp::new(Session::new(), Services::default());
        app.ui.interface_language = Preference::Czech;
        app.ui.dialog = Some(dialog.into());
        let ctx = egui::Context::default();
        app.logic(&ctx);
        let text = render(&mut app, &ctx);
        assert!(text.contains(heading), "{dialog}: {text}");
        assert_eq!(app.ui.dialog.as_deref(), Some(dialog));
    }
}

#[test]
fn default_fonts_cover_czech_letters_and_typographic_quotes() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let letters: BTreeSet<_> = "ÁČĎÉĚÍŇÓŘŠŤÚŮÝŽáčďéěíňóřšťúůýž„“".chars().collect();
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing {c} in {family:?}");
            }
        }
    });
}

#[test]
fn russian_and_french_plurals() {
    with_language("ru", || {
        for (n, word) in [(1, "участник"), (2, "участника"), (5, "участников"), (21, "участник"), (22, "участника"), (11, "участников")]
        {
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} {word}"));
        }
    });
    with_language("fr", || {
        for (n, word) in [(0, "contributeur"), (1, "contributeur"), (2, "contributeurs"), (10, "contributeurs")] {
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} {word}"));
        }
    });
}

#[test]
fn italian_locales_plurals_and_accents() {
    for tag in ["it", "it-IT", "it_IT.UTF-8", "IT-ch", " it-CH ", "it_IT@euro"] {
        assert_eq!(locale_language(tag), Some("it"), "{tag}");
    }
    assert_eq!(locale_language("italic"), None);
    assert_eq!(Preference::parse("it"), Some(Preference::Italian));
    assert_eq!(Preference::Auto.resolve(&["it-IT".into(), "en-US".into()]), "it");
    assert_eq!(Preference::Italian.resolve(&["uk-UA".into()]), "it");
    with_language("it", || {
        for (n, word) in [(0, "collaboratori"), (1, "collaboratore"), (2, "collaboratori"), (21, "collaboratori")] {
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} {word}"));
        }
        assert_eq!(t("Draw"), "Disegna");
        assert_eq!(tr_menu("it", "Modify"), "Edita");
        assert_eq!(tr("it", "Modify"), "Modifica");
        assert_eq!(tr_menu("it", "View"), "Visualizza");
        assert_eq!(tr("it", "View"), "Vista");
    });
    // Italian accented letters (à è é ì ò ù and capitals) come from the bundled egui fonts.
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let letters: BTreeSet<_> = include_str!("it.tsv").chars().filter(|c| ('\u{00c0}'..='\u{00ff}').contains(c)).collect();
    assert!(letters.contains(&'à') && letters.contains(&'è'));
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing {c} in {family:?}");
            }
        }
    });
}

/// Chinese shows only with a font for it: an installed CJK font, or one this build embedded from
/// craft-fonts (the web build has no system fonts). Without either the interface stays English
/// instead of drawing boxes, and choosing it says why.
#[test]
fn chinese_needs_a_cjk_font() {
    assert_eq!(locale_language("zh-CN"), Some("zh"));
    let mut app = CadApp::new(Session::new(), Services::default());
    let r = app.run("ui.language", json!({"lang": "zh"})).unwrap();
    assert_eq!(r["interfaceLanguage"], "zh");
    let shown = crate::theme::can_show("zh");
    assert_eq!(app.language(), if shown { "zh" } else { "en" });
    assert_eq!(r["shown"], app.language());
    assert!(crate::theme::can_show("uk") && crate::theme::can_show("en"));
    if !shown {
        assert!(app.status.as_ref().is_some_and(|(s, _)| s.contains("No Chinese font")));
    }
}

/// Arabic shows only with an installed Arabic font (none is bundled), like Chinese with its CJK font.
#[test]
fn arabic_needs_an_installed_arabic_font() {
    assert_eq!(locale_language("ar-SA"), Some("ar"));
    let mut app = CadApp::new(Session::new(), Services::default());
    let r = app.run("ui.language", json!({"lang": "ar"})).unwrap();
    assert_eq!(r["interfaceLanguage"], "ar");
    let font = crate::theme::arabic_font();
    assert_eq!(app.language(), if font.is_some() { "ar" } else { "en" });
    assert_eq!(r["shown"], app.language());
    if font.is_none() {
        assert!(app.status.as_ref().is_some_and(|(s, _)| s.contains("No Arabic font")));
    }
}

/// The Arabic catalog reads in visual order: a translation's words are reversed for display.
#[test]
fn arabic_catalog_is_stored_logically_and_shown_in_visual_order() {
    let ar = LANGUAGES.iter().find(|l| l.code == "ar").expect("ar language");
    let shown = tr("ar", "File");
    assert_eq!(shown, "ملف");
    assert_eq!(parse(ar.text()).1, Vec::<String>::new(), "catalog rows are well formed");
    assert_eq!(bidi::visual("ملف جديد"), "جديد ملف");
}

#[test]
fn korean_without_a_hangul_font_stays_english_and_says_why() {
    let mut app = CadApp::new(Session::new(), Services::default());
    let r = app.run("ui.language", json!({"lang": "ko"})).unwrap();
    let shown = crate::theme::can_show("ko");
    assert_eq!(r["shown"], if shown { "ko" } else { "en" });
    assert_eq!(app.ui.interface_language, Preference::Korean);
    if !shown {
        assert!(app.status.as_ref().is_some_and(|(s, _)| s.contains("No font covers the Korean interface")));
        assert_eq!(Preference::Korean.name(), "Korean");
    }
}

#[test]
fn korean_ui_fonts_cover_the_language_picker_in_both_families() {
    if !crate::theme::can_show("ko") {
        return;
    }
    // The picker shows 한국어 in an English interface only when the CJK fonts loaded in every
    // language cover Hangul; the Korean interface itself always does, with a fallback font.
    let ctx = egui::Context::default();
    let english = crate::theme::cjk_covers_hangul();
    assert_eq!(with_language("en", || Preference::Korean.name()), if english { "한국어" } else { "Korean" });
    assert_eq!(with_language("ko", || Preference::Korean.name()), "한국어");
    crate::theme::install_fonts(&ctx, if english { "en" } else { "ko" });
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let missing = ctx.fonts_mut(|view| {
        let mut missing = Vec::new();
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let chars = view.characters(&family);
            for c in "한국어".chars() {
                if !chars.contains_key(&c) {
                    missing.push(format!("{family:?}: {c}"));
                }
            }
        }
        missing
    });
    assert!(missing.is_empty(), "missing Korean UI glyphs: {}", missing.join(", "));
}

#[test]
fn swedish_locales_preferences_and_plural_forms() {
    for tag in ["sv", "sv-SE", "sv_SE.UTF-8", "SV-se", " sv-SE ", "sv-FI"] {
        assert_eq!(locale_language(tag), Some("sv"), "{tag}");
    }
    // `se` is Northern Sami and `swe` the three-letter code; neither picks Swedish.
    for tag in ["se", "swe", "svenska"] {
        assert_eq!(locale_language(tag), None, "{tag}");
    }
    assert_eq!(Preference::parse("sv"), Some(Preference::Swedish));
    assert_eq!(Preference::Swedish.name(), "Svenska");
    assert_eq!(Preference::Swedish.resolve(&["en-US".into()]), "sv");
    assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), "sv-SE".into(), "en-US".into()]), "sv");
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.interface_language = Preference::Swedish;
    let mut restored = CadApp::new(Session::new(), Services::default());
    restored.load_prefs(&app.prefs_json());
    assert_eq!(restored.ui.interface_language, Preference::Swedish);
    with_language("sv", || {
        for n in [0, 1, 2, 5, 21, u64::MAX] {
            // `bidragsgivare` is the same in the singular and the plural; `commit` takes an -s.
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} bidragsgivare"));
            let commit = if n == 1 { "commit" } else { "commits" };
            assert_eq!(count(n, "{n} commit", "{n} commits"), format!("{n} {commit}"));
        }
        assert_eq!(format("Current: {color}", &[("color", "{n} Open".into())]), "Aktuell: {n} Open");
    });
}

#[test]
fn swedish_menu_headings_keep_object_labels() {
    with_language("sv", || {
        assert_eq!(menu("Dimension"), "Måttsättning");
        assert_eq!(t("Dimension"), "Mått");
        assert_eq!(menu("View"), "Visa");
        assert_eq!(menu("Draw"), "Rita");
        assert_eq!(menu("Modify"), "Ändra");
        assert_eq!(menu("Edit"), "Redigera");
        assert_eq!(menu("File"), "Arkiv");
    });
}

#[test]
fn live_swedish_switch_preserves_drawing_command_and_saved_choice() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.in_window_menu = true;
    app.session.state_mut().unwrap().title = "Open".into();
    let ctx = egui::Context::default();
    app.logic(&ctx);
    assert!(render(&mut app, &ctx).contains("Properties"));
    app.start("line");
    let prompt = crate::control::cmdline_state(&app);
    let commands = crate::control::all_commands(&app);
    app.run("ui.language", json!({"lang":"sv"})).unwrap();
    let text = render(&mut app, &ctx);
    for label in ["Egenskaper", "Arkiv", "Visa", "Rita", "Måttsättning", "Ange första punkten"] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    assert_eq!(app.language(), "sv");
    assert_eq!(app.session.state().unwrap().title, "Open");
    assert_eq!(crate::control::cmdline_state(&app), prompt);
    assert_eq!(crate::control::all_commands(&app), commands);
    app.run("ui.language", json!({"lang":"en"})).unwrap();
    assert!(render(&mut app, &ctx).contains("Properties"));
}

#[test]
fn swedish_dialogs_render_with_stable_ids() {
    for (dialog, heading) in [
        ("dsettings", "Ritinställningar"),
        ("about", "Om muhandes"),
        ("blocks", "Block"),
        ("layers", "Hantera lageregenskaper"),
        ("qselect", "Snabbmarkering"),
        ("parameters", "Hantera parametrar"),
        ("language", "Svenska"),
        ("options:preferences", "Användarinställningar"),
    ] {
        let mut app = CadApp::new(Session::new(), Services::default());
        app.ui.interface_language = Preference::Swedish;
        app.ui.dialog = Some(dialog.into());
        let ctx = egui::Context::default();
        app.logic(&ctx);
        let text = render(&mut app, &ctx);
        assert!(text.contains(heading), "{dialog}: {text}");
        assert_eq!(app.ui.dialog.as_deref(), Some(dialog));
    }
}

#[test]
fn default_fonts_cover_swedish_letters_and_quotes() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let letters: BTreeSet<_> = "ÅÄÖÉåäöé”–".chars().collect();
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing {c} in {family:?}");
            }
        }
    });
}

#[test]
fn hungarian_locales_preferences_and_plural_forms() {
    for tag in ["hu", "hu-HU", "hu_HU.UTF-8", "HU-hu", " hu-HU "] {
        assert_eq!(locale_language(tag), Some("hu"), "{tag}");
    }
    // `hun` is the three-letter code; neither it nor the endonym picks Hungarian.
    for tag in ["hun", "magyar"] {
        assert_eq!(locale_language(tag), None, "{tag}");
    }
    assert_eq!(Preference::parse("hu"), Some(Preference::Hungarian));
    assert_eq!(Preference::Hungarian.name(), "Magyar");
    assert_eq!(Preference::Hungarian.resolve(&["en-US".into()]), "hu");
    assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), "hu-HU".into(), "en-US".into()]), "hu");
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.interface_language = Preference::Hungarian;
    let mut restored = CadApp::new(Session::new(), Services::default());
    restored.load_prefs(&app.prefs_json());
    assert_eq!(restored.ui.interface_language, Preference::Hungarian);
    with_language("hu", || {
        for n in [0, 1, 2, 5, 21, u64::MAX] {
            // A numeral takes the singular noun, so every count has the same form.
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} közreműködő"));
            assert_eq!(count(n, "{n} commit", "{n} commits"), format!("{n} commit"));
        }
        assert_eq!(format("Current: {color}", &[("color", "{n} Open".into())]), "Aktuális: {n} Open");
    });
}

#[test]
fn hungarian_menu_headings_keep_object_labels() {
    with_language("hu", || {
        assert_eq!(menu("Dimension"), "Méretezés");
        assert_eq!(t("Dimension"), "Méret");
        assert_eq!(menu("View"), "Nézet");
        assert_eq!(menu("Draw"), "Rajzolás");
        assert_eq!(menu("Modify"), "Módosítás");
        assert_eq!(menu("Edit"), "Szerkesztés");
        assert_eq!(menu("File"), "Fájl");
    });
}

#[test]
fn live_hungarian_switch_preserves_drawing_command_and_saved_choice() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.in_window_menu = true;
    app.session.state_mut().unwrap().title = "Open".into();
    let ctx = egui::Context::default();
    app.logic(&ctx);
    assert!(render(&mut app, &ctx).contains("Properties"));
    app.start("line");
    let prompt = crate::control::cmdline_state(&app);
    let commands = crate::control::all_commands(&app);
    app.run("ui.language", json!({"lang":"hu"})).unwrap();
    let text = render(&mut app, &ctx);
    for label in ["Tulajdonságok", "Fájl", "Nézet", "Rajzolás", "Méretezés", "Add meg az első pontot"] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    assert_eq!(app.language(), "hu");
    assert_eq!(app.session.state().unwrap().title, "Open");
    assert_eq!(crate::control::cmdline_state(&app), prompt);
    assert_eq!(crate::control::all_commands(&app), commands);
    app.run("ui.language", json!({"lang":"en"})).unwrap();
    assert!(render(&mut app, &ctx).contains("Properties"));
}

#[test]
fn hungarian_dialogs_render_with_stable_ids() {
    for (dialog, heading) in [
        ("dsettings", "Rajzolási beállítások"),
        ("about", "A muhandes névjegye"),
        ("blocks", "Blokk"),
        ("layers", "Rétegtulajdonság-kezelő"),
        ("qselect", "Gyorskijelölés"),
        ("parameters", "Paraméterkezelő"),
        ("language", "Magyar"),
        ("options:preferences", "Felhasználói beállítások"),
    ] {
        let mut app = CadApp::new(Session::new(), Services::default());
        app.ui.interface_language = Preference::Hungarian;
        app.ui.dialog = Some(dialog.into());
        let ctx = egui::Context::default();
        app.logic(&ctx);
        let text = render(&mut app, &ctx);
        assert!(text.contains(heading), "{dialog}: {text}");
        assert_eq!(app.ui.dialog.as_deref(), Some(dialog));
    }
}

#[test]
fn default_fonts_cover_hungarian_letters_and_quotes() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    // Ő ő Ű ű (double acute) are in Latin Extended-A, outside the ranges the other tests check.
    let letters: BTreeSet<_> = "ÁÉÍÓÖŐÚÜŰáéíóöőúüű„”–…".chars().collect();
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing {c} in {family:?}");
            }
        }
    });
}

#[test]
fn turkish_locales_preferences_and_single_count_form() {
    for tag in ["tr", "tr-TR", "tr_TR.UTF-8", "TR-tr", " tr-TR ", "tr-CY"] {
        assert_eq!(locale_language(tag), Some("tr"), "{tag}");
    }
    for tag in ["tur", "türkçe"] {
        assert_eq!(locale_language(tag), None, "{tag}");
    }
    assert_eq!(Preference::parse("tr"), Some(Preference::Turkish));
    assert_eq!(Preference::Turkish.name(), "Türkçe");
    assert_eq!(Preference::Turkish.resolve(&["en-US".into()]), "tr");
    assert_eq!(Preference::Auto.resolve(&["pl-PL".into(), "tr-TR".into(), "en-US".into()]), "tr");
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.interface_language = Preference::Turkish;
    let mut restored = CadApp::new(Session::new(), Services::default());
    restored.load_prefs(&app.prefs_json());
    assert_eq!(restored.ui.interface_language, Preference::Turkish);
    with_language("tr", || {
        // A Turkish noun stays singular after a number, 1 included.
        for n in [0, 1, 2, 5, 21, u64::MAX] {
            assert_eq!(count(n, "{n} contributor", "{n} contributors"), format!("{n} katkıda bulunan"));
            assert_eq!(count(n, "{n} commit", "{n} commits"), format!("{n} commit"));
        }
        assert_eq!(format("Current: {color}", &[("color", "{n} Open".into())]), "Geçerli: {n} Open");
    });
}

#[test]
fn turkish_menu_headings_keep_object_labels_and_prompt_keywords() {
    with_language("tr", || {
        assert_eq!(menu("Dimension"), "Ölçülendirme");
        assert_eq!(t("Dimension"), "Ölçü");
        assert_eq!(menu("View"), "Görünüm");
        assert_eq!(menu("Draw"), "Çiz");
        assert_eq!(menu("Modify"), "Değiştir");
        assert_eq!(menu("Edit"), "Düzenle");
        assert_eq!(menu("File"), "Dosya");
        // Clickable keywords keep the spelling the engine accepts.
        assert_eq!(t("Enter Top, Zero or Bottom."), "Top, Zero veya Bottom girin.");
        assert_eq!(t("Specify first point on circle: _tan to"), "Daire üzerindeki ilk noktayı belirtin: _tan to");
    });
}

#[test]
fn live_turkish_switch_preserves_drawing_command_and_saved_choice() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.in_window_menu = true;
    app.session.state_mut().unwrap().title = "Open".into();
    let ctx = egui::Context::default();
    app.logic(&ctx);
    assert!(render(&mut app, &ctx).contains("Properties"));
    app.start("line");
    let prompt = crate::control::cmdline_state(&app);
    let commands = crate::control::all_commands(&app);
    app.run("ui.language", json!({"lang":"tr"})).unwrap();
    let text = render(&mut app, &ctx);
    for label in ["Özellikler", "Dosya", "Görünüm", "Çiz", "Ölçülendirme", "İlk noktayı belirtin"] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    assert_eq!(app.language(), "tr");
    assert_eq!(app.session.state().unwrap().title, "Open");
    assert_eq!(crate::control::cmdline_state(&app), prompt);
    assert_eq!(crate::control::all_commands(&app), commands);
    app.run("ui.language", json!({"lang":"en"})).unwrap();
    assert!(render(&mut app, &ctx).contains("Properties"));
}

#[test]
fn turkish_dialogs_render_with_stable_ids() {
    for (dialog, heading) in [
        ("dsettings", "Çizim Ayarları"),
        ("about", "muhandes Hakkında"),
        ("blocks", "Bloklar"),
        ("layers", "Katman Özellikleri Yöneticisi"),
        ("qselect", "Hızlı Seçim"),
        ("parameters", "Parametre Yöneticisi"),
        ("language", "Türkçe"),
        ("options:preferences", "Kullanıcı Tercihleri"),
    ] {
        let mut app = CadApp::new(Session::new(), Services::default());
        app.ui.interface_language = Preference::Turkish;
        app.ui.dialog = Some(dialog.into());
        let ctx = egui::Context::default();
        app.logic(&ctx);
        let text = render(&mut app, &ctx);
        assert!(text.contains(heading), "{dialog}: {text}");
        assert_eq!(app.ui.dialog.as_deref(), Some(dialog));
    }
}

#[test]
fn default_fonts_cover_every_turkish_letter_in_the_catalog() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let mut letters: BTreeSet<_> = "ÇĞİÖŞÜçğıöşü".chars().collect();
    let translations = include_str!("tr.tsv").lines().filter(|l| !l.starts_with('#')).filter_map(|l| l.rsplit('\t').next());
    letters.extend(translations.flat_map(str::chars).filter(|c| !c.is_ascii() && c.is_alphabetic()));
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing {c} in {family:?}");
            }
        }
    });
}

#[test]
fn greek_catalog_locale_preferences_and_contexts() {
    assert_eq!(Preference::parse("el"), Some(Preference::Greek));
    assert_eq!(Preference::Greek.code(), "el");
    assert_eq!(Preference::Greek.name(), "Ελληνικά");
    for locale in ["el", "el-GR", "el_GR.UTF-8", "EL-cy"] {
        assert_eq!(locale_language(locale), Some("el"));
    }
    assert_eq!(locale_language("elvish"), None);
    assert_eq!(Preference::Greek.resolve(&["en-US".into()]), "el");
    let saved = serde_json::to_string(&Preference::Greek).unwrap();
    assert_eq!(saved, "\"el\"");
    assert_eq!(serde_json::from_str::<Preference>(&saved).unwrap(), Preference::Greek);
    assert_eq!(tr("el", "Layer"), "Επίπεδο (Layer)");
    assert_eq!(tr_menu("el", "Dimension"), "Διαστασιολόγηση");
    assert_eq!(tr("el", "Dimension"), "Διαστασιολόγηση (Dimension)");
    with_language("el", || {
        assert_eq!(t("Open"), "Άνοιγμα");
        assert_eq!(count(1, "{n} contributor", "{n} contributors"), "1 συνεισφέρων");
        assert_eq!(count(0, "{n} contributor", "{n} contributors"), "0 συνεισφέροντες");
        assert_eq!(count(2, "{n} contributor", "{n} contributors"), "2 συνεισφέροντες");
        assert_eq!(count(21, "{n} contributor", "{n} contributors"), "21 συνεισφέροντες");
    });
    assert_eq!(t("Open"), "Open", "language must not leak outside a render scope");
}

#[test]
fn default_fonts_cover_all_greek_letters_in_the_catalog() {
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    let letters: BTreeSet<_> =
        include_str!("el.tsv").chars().filter(|c| ('\u{0370}'..='\u{03ff}').contains(c) || ('\u{1f00}'..='\u{1fff}').contains(c)).collect();
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let mut font = view.fonts.font(&family);
            for c in &letters {
                assert!(font.characters().contains_key(c), "missing Greek glyph {c} in {family:?}");
            }
        }
    });
}

/// The production font chain covers alphanumeric ASCII in proportional and monospace, plus ⌘ ⇧ ↩ … − in proportional and ⇧ in monospace.
#[test]
fn production_font_chain_covers_latin_and_shortcut_symbols() {
    // Exercise the same chain the app installs: the workspace egui dependency enables
    // `default_fonts` and `monochrome_emoji_fonts` in production, so web and native builds
    // always start with bounded Latin and monochrome icon faces instead of an empty family.
    let ctx = egui::Context::default();
    crate::theme::install_fonts(&ctx, "en");
    let mut output = ctx.run_ui(Default::default(), |_| {});
    output.textures_delta.clear();
    ctx.fonts_mut(|view| {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars() {
                assert!(view.has_glyph(&egui::FontId::new(12.0, family.clone()), c), "Latin {c:?} missing in {family:?}");
            }
        }
        // Proportional shortcut labels (`Button::shortcut_text` and the translated menus) use
        // these symbols: ⌘ from the monochrome icon face, ⇧ from Hack as a bounded fallback.
        for c in ['\u{2318}', '\u{21e7}', '\u{21a9}', '\u{2026}', '\u{2212}'] {
            assert!(view.has_glyph(&egui::FontId::proportional(12.0), c), "U+{:04X} missing in Proportional", c as u32);
        }
        assert!(view.has_glyph(&egui::FontId::monospace(12.0), '\u{21e7}'), "U+21E7 shift symbol missing in Monospace");
    });
}
