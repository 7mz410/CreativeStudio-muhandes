#!/usr/bin/env python3
"""Apply the muhandes (Creative Studio) branding on top of upstream CADCraft.

Only user-visible text is rebranded; crate names, the file format, settings folders and the
plot-style device names keep their upstream names so `git merge upstream/main` stays easy. The
script is idempotent: after merging upstream, resolve any conflicts in favour of upstream and run
it again.

    python3 studio/rebrand.py
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import arabic_names  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
NAME = 'muhandes'
ARABIC = 'مهندس'
REPO = 'https://github.com/7mz410/CreativeStudio-muhandes'
SITE = 'https://studio.creative.ps'
APP_SITE = 'https://muhandes.studio.creative.ps'
# Persisted names and legal lines; renaming them would lose settings or misstate copyright.
KEEP = (
    'by the ArtCraft team', 'Application Support/CADCraft', '/roaming/CADCraft', 'CADCraft authors',
    'CADCraft contributors', 'CADCraft PDF', 'CADCraft PNG', 'CADCraft SVG', 'CADCraft Stroke',
    'مبني على CADCraft من فريق ArtCraft',
    'CADCraft.Drawing', 'CADCRAFT_', 'на основі CADCraft командою ArtCraft',
)

# (file, upstream text, our text[, marker meaning "already applied"]).
RULES = [
    # About: our links, the Creative Studio suite, no Discord.
    ('crates/ui-egui/src/dialogs.rs', '                    ui.heading(crate::tl!("CADCraft"));',
     f'                    ui.heading(crate::tl!("{NAME}"));'),
    ('crates/ui-egui/src/dialogs.rs', 'ui.hyperlink_to("getartcraft.com/apps/cadcraft", "https://getartcraft.com/apps/cadcraft");',
     f'ui.hyperlink_to("{NAME}.studio.creative.ps", "{APP_SITE}");'),
    ('crates/ui-egui/src/dialogs.rs', 'crate::tl!("One of the Crafting Apps by the ArtCraft team and community.")',
     'crate::tl!("Part of the Creative Studio suite. Built on CADCraft by the ArtCraft team.")'),
    # Visible titles that are exact "CADCraft" literals (the general rename leaves those alone,
    # because the same word names the Windows and macOS settings folders).
    ('crates/ui-egui/src/chrome.rs', 'crate::tl!("CADCraft"), egui::FontId', f'crate::tl!("{NAME}"), egui::FontId'),
    ('crates/ui-egui/src/closing.rs', 'RichText::new("CADCraft").strong()', f'RichText::new("{NAME}").strong()'),
    ('crates/ui-egui/src/i18n/tests.rs', '["X", "Y", "B", "CADCraft"]', f'["X", "Y", "B", "CADCraft", "{NAME}"]'),
    ('apps/cadcraft/src/native_menu.rs', 'Submenu::new("CADCraft", true)', f'Submenu::new("{NAME}", true)'),
    ('apps/cadcraft/src/main.rs', '.with_title("CADCraft")', f'.with_title("{NAME}")'),
    # Web page and packages.
    ('apps/cadcraft-web/index.html', '<title>CADCraft</title>', f'<title>{NAME}</title>'),
    ('apps/cadcraft/build.rs', '.set("ProductName", "CADCraft")', f'.set("ProductName", "{NAME}")'),
    ('packaging/linux/ai.storyteller.cadcraft.desktop', 'Name=CADCraft', f'Name={NAME}'),
    ('packaging/linux/ai.storyteller.cadcraft.metainfo.xml.in', '<name>CADCraft</name>', f'<name>{NAME}</name>'),
    ('packaging/windows/cadcraft.wxs', '<Feature Id="Main" Title="CADCraft" Level="1">', f'<Feature Id="Main" Title="{NAME}" Level="1">'),
    # Installer identity: our own UpgradeCode (so muhandes never replaces a CADCraft install), our
    # registry keys and ProgId. Component GUIDs are ours too (Windows Installer tracks them machine-wide).
    ('packaging/windows/cadcraft.wxs', 'UpgradeCode="119D7D76-0167-4A9E-8F37-09DA0235F68E"',
     'UpgradeCode="EF215B5F-639A-48A1-ACDC-24F0482B6704"', 'EF215B5F-639A-48A1-ACDC-24F0482B6704'),
    ('packaging/windows/cadcraft.wxs', 'Software\\Learning Machines LLC\\CADCraft', 'Software\\Creative Studio\\muhandes', 'Creative Studio\\muhandes'),
    ('packaging/windows/cadcraft.wxs', 'Name="CADCraft.Drawing"', 'Name="muhandes.Drawing"', 'muhandes.Drawing'),
    ('packaging/windows/cadcraft.wxs', 'Software\\Classes\\CADCraft.Drawing\\shell', 'Software\\Classes\\muhandes.Drawing\\shell', 'muhandes.Drawing\\shell'),
    ('packaging/windows/cadcraft.wxs', 'Guid="2AC18056-48DE-4079-AB79-4FDEF4FBB075"', 'Guid="E860A02D-5476-4521-9A17-882D798476AD"', 'E860A02D-5476-4521-9A17-882D798476AD'),
    ('packaging/windows/cadcraft.wxs', 'Guid="76029CD7-EBDB-420F-AC34-FF0E82A99E32"', 'Guid="6EE98F27-6F26-41EB-8B1A-35E5CF16A122"', '6EE98F27-6F26-41EB-8B1A-35E5CF16A122'),
    # Right to left (studio/RTL.md): the Arabic interface flips through egui's switch (vendor/egui).
    ('Cargo.toml', '[workspace.dependencies]\n',
     '[patch.crates-io]\n# Creative Studio: egui with a right-to-left switch for the Arabic interface (studio/RTL.md).\negui = { path = "vendor/egui" }\n\n[workspace.dependencies]\n',
     '[patch.crates-io]'),
    ('crates/ui-egui/src/lib.rs', "        i18n::with_language(self.language(), || self.draw(ui));\n",
     "        let language = self.language();\n"
     "        // Creative Studio: Arabic lays the interface out right to left (vendor/egui). The switch is\n"
     "        // process-wide, so unit tests, which run in parallel, leave it off (tests/rtl.rs covers it).\n"
     "        #[cfg(not(test))]\n"
     "        egui::set_rtl(i18n::bidi::RTL_CODES.contains(&language));\n"
     "        i18n::with_language(language, || self.draw(ui));\n", 'egui::set_rtl('),
    ('crates/ui-egui/src/palettes.rs', '    egui::Panel::right("cc_palettes").exact_size(300.0)',
     '    // Creative Studio: right to left, the palettes sit on the left.\n'
     '    let panel = if egui::is_rtl() { egui::Panel::left("cc_palettes") } else { egui::Panel::right("cc_palettes") };\n'
     '    panel.exact_size(300.0)', 'is_rtl() { egui::Panel::left("cc_palettes")'),
]
# Text in string literals and catalogs, before the general rename.
LITERALS = []
STRING = re.compile(r'"(?:[^"\\\n]|\\.)*"')
# Literals that are only a settings-folder name (or a plot device) are kept as they are.
EXACT_KEEP = ('"CADCraft"',)
# The MCP server (crates/mcp) keeps upstream's wording, so its tests keep it too.
SKIP_FILES = ('apps/cadcraft-cli/tests/mcp_connect.rs', 'apps/cadcraft-cli/tests/it/mcp_connect.rs')


def rename(lit: str) -> str:
    for a, b in LITERALS:
        lit = lit.replace(a, b)
    if lit in EXACT_KEEP or any(k in lit for k in KEEP):
        return lit
    return lit.replace('CADCraft', NAME)


def renamed_code(text: str) -> str:
    return STRING.sub(lambda m: rename(m.group(0)), text)


def apply_rules() -> list:
    changed = []
    for rel, a, b, *marker in RULES:
        p = ROOT / rel
        s = p.read_text(encoding='utf-8')
        if marker and marker[0] in s:
            continue
        if a not in s and renamed_code(a) in s:
            a = renamed_code(a)  # the general rename got there first
        if a not in s or (b and a in b and b in s):
            if not b or b in s or renamed_code(b) in s:
                continue  # already applied
            print(f'warning: {rel}: upstream text not found (did upstream change it?): {a[:70]!r}')
            continue
        p.write_text(s.replace(a, b), encoding='utf-8')
        changed.append(rel)
    return changed


def rename_sources() -> list:
    changed = []
    for p in sorted((ROOT / 'crates/ui-egui').rglob('*.rs')) + [q for q in sorted((ROOT / 'crates/engine').rglob('*.rs')) if not q.name.startswith('tests')] + sorted((ROOT / 'apps').rglob('*.rs')):
        if p.relative_to(ROOT).as_posix() in SKIP_FILES:
            continue
        s = p.read_text(encoding='utf-8')
        o = renamed_code(s)
        if o != s:
            p.write_text(o, encoding='utf-8')
            changed.append(p.relative_to(ROOT).as_posix())
    # Catalog rows are `context<TAB>English<TAB>translation`: rename both text columns alike.
    for p in sorted((ROOT / 'crates/ui-egui/src/i18n').glob('*.tsv')):
        s = p.read_text(encoding='utf-8')
        rows = []
        for line in s.split('\n'):
            cells = line.split('\t')
            if len(cells) >= 3 and not line.startswith('#'):
                line = '\t'.join([cells[0], *(c if any(k in c for k in KEEP) else c.replace('CADCraft', NAME) for c in cells[1:])])
            rows.append(line)
        o = '\n'.join(rows)
        if o != s:
            p.write_text(o, encoding='utf-8')
            changed.append(p.relative_to(ROOT).as_posix())
    # macOS bundle: display and name only (the executable name must stay the binary's).
    plist = ROOT / 'packaging/macos/Info.plist.in'
    s = plist.read_text(encoding='utf-8')
    o = re.sub(r'(<key>CFBundle(?:Display)?Name</key>\s*<string>)CADCraft(</string>)', rf'\g<1>{NAME}\2', s)
    o = o.replace('<string>ai.storyteller.cadcraft</string>', '<string>ps.creative.studio.muhandes</string>')
    # Our own document types: the CADCraft UTIs (ai.storyteller.cadcraft.dxf/dwg) are upstream's, so
    # muhandes declares its own, and both document types read "muhandes Drawing".
    for ext in ('dxf', 'dwg'):
        o = o.replace(f'ai.storyteller.cadcraft.{ext}', f'ps.creative.studio.muhandes.{ext}')
    o = re.sub(r'(<key>(?:CFBundleTypeName|UTTypeDescription)</key>\s*<string>)[^<]*(</string>)', rf'\g<1>{NAME} Drawing\2', o)
    if o != s:
        plist.write_text(o, encoding='utf-8')
        changed.append('packaging/macos/Info.plist.in')
    # Windows installer: visible names only (UpgradeCode, registry keys and GUIDs come from RULES).
    wxs = ROOT / 'packaging/windows/cadcraft.wxs'
    s = wxs.read_text(encoding='utf-8')
    o = s.replace('Name="CADCraft"', f'Name="{NAME}"').replace('Name="CADCraft" Description', f'Name="{NAME}" Description')
    o = o.replace('Description="CADCraft computer-aided design"', f'Description="{NAME} computer-aided design"')
    o = o.replace('Name="CADCraft"', f'Name="{NAME}"')
    if o != s:
        wxs.write_text(o, encoding='utf-8')
        changed.append('packaging/windows/cadcraft.wxs')
    return changed


def main():
    changed = apply_rules() + rename_sources()
    changed += arabic_names.apply(ROOT, NAME, ARABIC)
    for c in sorted({str(c.relative_to(ROOT)) if isinstance(c, Path) else c for c in changed}):
        print('rebranded', c)


if __name__ == '__main__':
    main()


# Creative Studio: the macOS app and DMG volume carry our name (the executable keeps upstream's,
# which Info.plist names). Same rule as the other apps' rebrand.py.
PACKAGING_NAMES = {
    'packaging/macos/package.sh': [
        ('CADCraft.app', 'muhandes.app'),
        ('-hfs-volume-name "CADCraft"', '-hfs-volume-name "muhandes"'),
    ],
}
for _rel, _pairs in PACKAGING_NAMES.items():
    _path = ROOT / _rel
    _src = _path.read_text(encoding='utf-8')
    _out = _src
    for _a, _b in _pairs:
        _out = _out.replace(_a, _b)
    if _out != _src:
        _path.write_text(_out, encoding='utf-8')
        print('rebranded', _rel)
