# Grep Console for OpenIDEA

A small plugin for coloring Run/Debug stdout and stderr with regular expressions.
Inspired by the original [Grep Console](https://github.com/krasa/grepconsole)
plugin for IntelliJ IDEA.

Requires plugin API **1.12+**; the SDK is pinned to the API 1.12 revision.
It does not read console output. The `events` capability is used only to reload
rules when settings change.

## Usage

Open **Settings > Plugins > Grep Console**. Highlighting starts automatically:

- `ERROR` / `FATAL`: bold bright-red lines, case-insensitive.
- `WARN` / `WARNING`: bright-yellow lines, case-insensitive.
- `INFO`: bold green matched text.

Add, remove or reorder rows in **Highlight rules**, then Apply. Each rule has
an enabled checkbox, a regex, foreground/background palette colors, an optional
style, whole-line mode and an ignore-case checkbox. Empty colors/style preserve
the existing appearance. **Enable console highlighting** switches all rules off
without deleting them. **Grep Console: Reload Rules** reapplies saved settings.

The first enabled whole-line match wins. Match-only rules are drawn over it,
with earlier rules winning overlaps. Colors follow the active console palette.
Rules apply in every window, not to IDE-generated messages. Project overrides
use the invoking project's settings, but the applied list is global: the last
reload wins. Prefer global settings for consistent behaviour across projects.

Patterns use the Rust/JavaScript common regex subset. No inline flags (use
Ignore case), lookaround, backreferences, named groups or Unicode property
classes. The IDE also rejects patterns with unsafe backtracking. At most
100 rules, 512 UTF-8 bytes per pattern; only the first 2,000 characters of each
line are matched. Invalid updates keep the previous active rules and show an
error notification; fix the settings and Apply again.

## Build and install

Install Rust and the OpenIDEA plugin CLI, then:

```sh
 cargo test --locked
 openidea-plugin build
 openidea-plugin link
```

The CLI can be installed from the OpenIDEA checkout:

```sh
 cargo install --path ../openidea/crates/ide-host --bin openidea-plugin --locked
```

`build` creates and validates `dist/plugin.wasm`. `link` registers this folder
with the IDE; use **Settings > Plugins > Reload** after rebuilding.
Alternatively install this folder through **Install Plugin from Disk**.

To create an installable archive:

```sh
 openidea-plugin pack . --out dist/solhovoy.grep-console-0.1.0.oiplugin
```

Choose the resulting `.oiplugin` in **Install Plugin from Disk**. The local
development archive is unsigned.

## License

MIT