# keybindings-skins Specification

## Purpose

Describe how sofka binds keyboard actions per screen through `[keys]` (chord syntax, defaults, unbinding, conflicts, legacy palette key migration), how bindings are shown in the header, footer and `?` help panel, and how colour skins are selected, auto-detected, overridden per swatch, switched live with `:skin` and overridden per cluster or context.

## Requirements

### Requirement: Key binding tables

sofka SHALL read key bindings from `[keys.<scope>]` tables that map action names to a key chord string or an array of chord strings. A configured value MUST replace that action's default chords, an empty array `[]` MUST unbind the action, and equivalent chords repeated in one list SHALL count once. Bindings SHALL follow cluster and context overrides and `:reload`.

#### Scenario: Rebind an action

- **WHEN** the config sets `[keys.table] sort_age = "ctrl-a"`
- **THEN** `ctrl-a` sorts the table by age
- **AND** `A` no longer sorts by age

#### Scenario: Unbind an action

- **WHEN** the config sets `[keys.table] favorite_namespace_2 = []`
- **THEN** pressing `2` in the table does not switch namespace

#### Scenario: Several chords

- **WHEN** the config sets `[keys.logs] log_marker = ["m", "ctrl-m"]`
- **THEN** both `m` and `ctrl-m` add a log marker

### Requirement: Key scopes

sofka SHALL accept the shared scopes `global`, `navigation` and `input` and one scope per screen, including `table`, `detail`, `diff`, `events`, `logs`, `help`, `containers`, `confirm`, `prompt`, `pulse`, `xray`, `rbac`, `explain`, `timeline`, `gitops`, `argocd`, `adjacent`, `skins`, `snapshots`, `fleet`, `find`, `pvc_explore`, `contexts`, `command`, `filter`, `namespaces`, `drain` and the other picker and form scopes. An unknown scope MUST be rejected with `keys.<scope>: unknown scope`.

#### Scenario: Unknown scope

- **WHEN** the config contains `[keys.tabel]`
- **THEN** sofka reports `keys.tabel: unknown scope`

### Requirement: Shared scope precedence

`[keys.global]` SHALL apply to every screen, `[keys.navigation]` to every non-text screen, and `[keys.input]` to every text-entry screen. A binding in a screen's own scope MUST take priority over the shared scopes regardless of the order in the file.

#### Scenario: Screen scope wins

- **WHEN** the config sets `[keys.navigation] page_down = "ctrl-d"` and `[keys.table] page_down = "f8"`
- **THEN** `f8` pages down in the table
- **AND** `ctrl-d` pages down in the logs view

### Requirement: Global default keys

In every screen `ctrl-c` SHALL quit, `ctrl-e` SHALL toggle compact mode and `ctrl-alt-t` SHALL open plugin activity, unless the user rebinds these actions in `[keys.global]` or a screen scope.

#### Scenario: Compact from any screen

- **WHEN** the logs view is open and the user presses `ctrl-e`
- **THEN** compact mode toggles

### Requirement: Text input default keys

Text-entry screens (command palette, filters, prompts, plugin forms, sort and copy pickers, namespace and context filters, drain options) SHALL default to `esc` back, `enter` accept, `backspace` or `ctrl-h` delete a character, `ctrl-u` clear the line, and `ctrl-w`, `alt-backspace` or `ctrl-backspace` delete a word.

#### Scenario: Clear the command line

- **WHEN** the user types `:deployments` and presses `ctrl-u`
- **THEN** the command input is empty

### Requirement: View default keys for command and help

Every non-text screen SHALL open the command palette with `:` and the help panel with `?`, except the help panel, where `q` and `?` close help.

#### Scenario: Help from the logs view

- **WHEN** the logs view is open and the user presses `?`
- **THEN** the help panel opens

### Requirement: Shifted navigation aliases

sofka SHALL add shifted aliases for the arrow, page, home and end navigation keys in every scope except the command palette, so `shift-down` acts like `down`. In the table, `shift-up` and `shift-down` MUST extend a range selection instead.

#### Scenario: Shift-arrow in a picker

- **WHEN** the namespace switcher is open and the user presses `shift-down`
- **THEN** the selection moves down one entry

#### Scenario: Range selection in the table

- **WHEN** the table is open and the user presses `shift-down` twice
- **THEN** the current row and the rows below it are marked as one range

### Requirement: Navigation action scope

The navigation actions `up`, `down`, `first`, `last`, `page_up`, `page_down`, `left`, `right`, `back`, `close`, `command` and `help` SHALL be accepted in `[keys.navigation]` and MUST apply to non-text screens only, except that `back` in the confirm dialog is configured by `[keys.confirm]`.

#### Scenario: Navigation does not touch text input

- **WHEN** the config sets `[keys.navigation] down = "j"`
- **AND** the user types `j` in the command palette
- **THEN** `j` is inserted into the command text

### Requirement: Command completion keys take priority

Keys explicitly configured in `[keys.command]` for `down`, `up`, `accept` and `complete` MUST take priority over input editing keys in the command palette.

#### Scenario: ctrl-w completes

- **WHEN** the config sets `[keys.command] complete = "ctrl-w"`
- **AND** the user types `:depl` and presses `ctrl-w`
- **THEN** the highlighted suggestion is filled in instead of deleting a word

### Requirement: Inherited backspace alias yields

The inherited `ctrl-h` backspace alias MUST yield when another action in the same scope binds `ctrl-h`, unless `backspace` is configured explicitly for that scope, in which case the conflict SHALL be reported.

#### Scenario: Rebinding ctrl-h in the filter

- **WHEN** the config sets `[keys.filter] accept = "ctrl-h"`
- **THEN** `ctrl-h` accepts the filter
- **AND** no conflict is reported

### Requirement: Confirm dialog cancel keys

By default a confirm dialog SHALL cancel on any key it does not handle and on a mouse wheel event. Setting `[keys.confirm] back` MUST limit keyboard cancel to the configured chords, and `back = []` MUST disable both keyboard and wheel cancel.

#### Scenario: Default cancel

- **WHEN** a delete confirmation is open and the user presses `x`
- **THEN** the dialog closes without deleting

#### Scenario: Explicit back

- **WHEN** the config sets `[keys.confirm] back = "esc"` and a confirmation is open
- **AND** the user presses `x`
- **THEN** the dialog stays open

### Requirement: Key value validation

sofka MUST reject a non-table `keys` value with `keys: expected a table`, a non-table scope with `keys.<scope>: expected a table of actions`, an unknown action with `keys.<scope>.<name>: unsupported action`, and a value that is not a string or array with `<path>: expected a key string or an array of key strings`. A non-string array element MUST report `<path>: expected a key string, got <type>`.

#### Scenario: Unsupported action

- **WHEN** the config sets `[keys.table] fly = "f"`
- **THEN** sofka reports `keys.table.fly: unsupported action`

#### Scenario: Number value

- **WHEN** the config sets `[keys.table] sort_age = 5`
- **THEN** sofka reports `keys.table.sort_age: expected a key string or an array of key strings`

### Requirement: Key conflict detection

When one chord is bound to two actions in the same effective scope, sofka MUST report `keys.<scope>: <chord> conflicts between <a> and <b>`, including conflicts between configured and default chords.

#### Scenario: Conflict with a default

- **WHEN** the config sets `[keys.table] sort_age = "d"`
- **THEN** sofka reports `keys.table: d conflicts between` naming `describe` and `sort_age`

### Requirement: Invalid keymap is not applied

When the effective key configuration has any error, sofka MUST NOT apply any part of it: at startup it SHALL keep the default keymap with the warning `<paths>: <error>; default keymap kept`, and on `:reload` or a context switch it SHALL keep the previous keymap with `<paths>: <error>; previous keymap kept`. Other settings MUST still apply.

#### Scenario: Atomic reload

- **WHEN** a working keymap is active and the user reloads a config with a key conflict
- **THEN** every binding stays as before
- **AND** a warning names the config file and the conflict
- **AND** other changed settings are applied

### Requirement: Chord modifiers

A chord SHALL be one key optionally preceded by modifiers joined with `-`. Modifier names MUST be case-insensitive: `ctrl`, `control` or `c`; `alt`, `option`, `opt`, `meta`, `a` or `m`; and `shift` or `s`. A trailing `--` SHALL mean the minus key, and a single character SHALL mean that character.

#### Scenario: Alternative modifier names

- **WHEN** the config binds `C-M-x`
- **THEN** the binding matches `ctrl-alt-x`

#### Scenario: Minus key

- **WHEN** the config binds `ctrl--`
- **THEN** pressing `ctrl` with `-` triggers the action

### Requirement: Named keys

sofka SHALL accept the named keys `enter` (`return`, `ret`), `tab`, `backtab`, `esc` (`escape`), `space`, `backspace` (`bs`), `delete` (`del`), `insert` (`ins`), `home`, `end`, `pageup` (`pgup`), `pagedown` (`pgdn`, `pgdown`), `up`, `down`, `left`, `right` and `f1` to `f24`.

#### Scenario: Function key

- **WHEN** the config binds `f8` to page down
- **THEN** pressing F8 pages down

### Requirement: Shift handling in chords

`shift-<letter>` SHALL be treated as the uppercase letter, and `shift-tab` SHALL equal `backtab`. `shift--` MUST be rejected with `Shift with minus is not supported; bind the resulting character instead (for example, "_")`.

#### Scenario: Shifted letter

- **WHEN** the config binds `shift-g`
- **THEN** the binding is shown as `G` and triggers on `G`

#### Scenario: Shifted minus

- **WHEN** the config binds `shift--`
- **THEN** sofka reports that Shift with minus is not supported

### Requirement: Chord parse errors

sofka MUST reject an empty chord with `empty key binding`, a doubled or stray separator with `empty modifier (stray '-'?)`, an unrecognised modifier with an `unknown modifier` error, and an unrecognised key name with an `unknown key` error.

#### Scenario: Unknown key name

- **WHEN** the config binds `ctrl-foo`
- **THEN** sofka reports an `unknown key` error for that setting

#### Scenario: Empty chord

- **WHEN** the config binds `""`
- **THEN** sofka reports `empty key binding`

### Requirement: Chord matching

A pressed key SHALL match a chord only when the ctrl and alt modifiers match exactly. Plain characters MUST match case-sensitively, ctrl-letter chords MUST match either case, `backtab` MUST match with or without shift, and named and function keys MUST require an exact shift match.

#### Scenario: Case-sensitive letter

- **WHEN** an action is bound to `g`
- **AND** the user presses `G`
- **THEN** the action does not run

#### Scenario: Ctrl-letter ignores case

- **WHEN** an action is bound to `ctrl-d`
- **AND** the terminal reports `ctrl-D`
- **THEN** the action runs

### Requirement: Key labels in header, footer and help

sofka SHALL show bindings with canonical labels such as `ctrl-alt-delete` and `space`. The header and footer hints MUST show the first chord of an action, the help panel MUST show all chords joined with `/`, and an unbound action MUST be shown as `unbound`.

#### Scenario: Custom key in the footer

- **WHEN** the config sets `[keys.table] yaml = ["ctrl-y", "y"]`
- **THEN** the footer shows `ctrl-y` for the YAML action
- **AND** help shows `ctrl-y / y`

### Requirement: Help panel contents

`?` SHALL open a help panel that lists, for each scope, a `Keys: <scope>` heading with every action's keys and description from the active keymap, followed by the command palette commands, configured plugin, bookmark and workspace keys, and the help navigation keys.

#### Scenario: Rebound key in help

- **WHEN** the config sets `[keys.table] sort_age = "ctrl-a"` and the user opens help
- **THEN** the `Keys: table` section shows `ctrl-a` for the age sort

### Requirement: Help panel search

In the help panel `/` SHALL start a search that keeps only matching lines together with their section headings and titles the panel `Help · /<text> [<matches>]`. `esc` MUST clear an active search first and close help otherwise, and `q` or `?` MUST close help. Opening help, starting a search and clearing a search SHALL reset the scroll to the top.

#### Scenario: Search keeps headings

- **WHEN** the user opens help, presses `/` and types `marker`
- **THEN** the log marker binding is shown under its `Keys: logs` heading

#### Scenario: Esc clears then closes

- **WHEN** a help search is active and the user presses `esc` twice
- **THEN** the first `esc` clears the search and the second closes help

### Requirement: Help panel scrolling

The help panel SHALL scroll one line with `j`/`k` or the arrows, one visible page forward with `ctrl-f`, `PgDn` or `space`, one page back with `ctrl-b` or `PgUp`, and to the top or bottom with `g`/`Home` and `G`/`End`.

#### Scenario: Page forward

- **WHEN** the help panel is longer than the screen and the user presses `space`
- **THEN** the content moves forward by the number of visible rows

### Requirement: Kind-specific table actions

Kind-specific table actions SHALL take priority only on their kinds: `faults`, `attach` and `previous_logs` on pods; `port_forward` on pods and services; `inspect` on secrets and PVCs; `cordon`, `uncordon` and `drain` on nodes; `set_image` on pods and pod-owning workload kinds. On other kinds the same chord MUST go to a bookmark, then a workspace, then a plugin bound to it.

#### Scenario: Plugin key on another kind

- **WHEN** a plugin is bound to `C`, the default `cordon` key
- **AND** the user presses `C` on a deployment
- **THEN** the plugin runs

#### Scenario: Built-in wins on its kind

- **WHEN** a plugin is bound to `C`, the default `cordon` key
- **AND** the user presses `C` on a node
- **THEN** the node is cordoned instead of running the plugin

### Requirement: Hidden custom binding warning

When a plugin, bookmark or workspace key is shadowed by a table action, sofka MUST warn `<plugin|bookmark|workspace> "<name>": <chord> is hidden by keys.table.<action> when that action is available`.

#### Scenario: Bookmark hidden by describe

- **WHEN** a bookmark named `prod` is bound to `d`
- **THEN** a config warning says `bookmark "prod": d is hidden by keys.table.describe when that action is available`

### Requirement: Favourite namespace keys

The table actions `favorite_namespace_1` to `favorite_namespace_9` SHALL default to `1` to `9` and select entries of `favorite_namespaces` in configuration order. Empty or unconfigured slots MUST do nothing, and the same keys SHALL work in the namespace switcher while its filter is empty.

#### Scenario: Second favourite

- **WHEN** `favorite_namespaces = ["kube-system", "monitoring"]` and the user presses `2` in the table
- **THEN** the namespace changes to `monitoring`

#### Scenario: Empty slot

- **WHEN** only two favourites are configured and the user presses `5`
- **THEN** the namespace does not change

### Requirement: Mouse wheel follows the keymap

With mouse capture on, a wheel event SHALL run the current screen's `up` or `down` action, and in a confirm dialog it SHALL run `back`.

#### Scenario: Wheel in a picker

- **WHEN** the skins picker is open with mouse capture on and the user scrolls down
- **THEN** the selection moves down

### Requirement: Legacy palette key migration

sofka SHALL convert the legacy settings `keys.palette_next`, `keys.palette_prev` and `keys.palette_accept` of each config file to `keys.command.down`, `up` and `accept` before merging, and SHALL save the converted file, keeping TOML comments and rewriting YAML in full. On success it MUST warn `<path>: moved legacy palette keys to [keys.command]; backup: <backup>`.

#### Scenario: Successful migration

- **WHEN** a writable `config.toml` contains `[keys] palette_next = "ctrl-n"` and the user runs `:reload`
- **THEN** the file now contains `[keys.command] down = "ctrl-n"`
- **AND** `config.toml.bak` holds the original content
- **AND** `ctrl-n` moves down in the command palette

### Requirement: Migration backup safety

The migration MUST write the original content to `<file>.<ext>.bak` without overwriting an existing backup and MUST keep the file's permissions. When the backup already exists or the file cannot be written, sofka MUST NOT change the file.

#### Scenario: Backup already exists

- **WHEN** `config.toml.bak` already exists and a legacy palette key is found
- **THEN** `config.toml` is left unchanged
- **AND** the warning includes `cannot create backup`

### Requirement: Read-only config migration

When the config file is read-only or a symlink (such as a Home Manager managed file), sofka MUST NOT write it and SHALL use the migrated keys in memory with the warning `<path>: cannot save config migration: <reason>; using migrated keys in memory. Update [keys.command] in the config source: palette_next -> down, palette_prev -> up, palette_accept -> accept`.

#### Scenario: Managed symlink

- **WHEN** `config.toml` is a symlink and contains `palette_accept = "tab"`
- **THEN** the file is not modified
- **AND** `tab` accepts in the command palette

### Requirement: Migration refusals

sofka MUST NOT migrate a file whose legacy values are invalid chords, warning `cannot migrate palette keys`, or whose `keys.command` already sets the target action, warning `cannot migrate keys.<old>; check keys.command.<new> and remove the legacy field`. When the effective keymap has errors, it MUST NOT save the migration and SHALL warn `config migration was not saved; correct the config errors, then reload`. The `sofka info` report MUST NOT write migrations.

#### Scenario: Legacy and new key together

- **WHEN** a file sets both `keys.palette_next` and `keys.command.down`
- **THEN** sofka warns `cannot migrate keys.palette_next; check keys.command.down and remove the legacy field`
- **AND** the file is not changed

### Requirement: Built-in skins

sofka SHALL provide the built-in skins `catppuccin-mocha`, `catppuccin-latte`, `catppuccin-frappe`, `catppuccin-macchiato`, `gruvbox-dark`, `gruvbox-light`, `nord`, `dracula`, `solarized-dark`, `solarized-light`, `tokyo-night`, `one-dark`, `rose-pine`, `rose-pine-dawn`, `monokai`, `flexoki-dark` and `flexoki-light`. Names MUST match case-insensitively, and short aliases such as `mocha`, `latte`, `gruvbox`, `solarized`, `tokyonight` and `flexoki` SHALL also be accepted.

#### Scenario: Alias name

- **WHEN** the config sets `[skin] name = "Gruvbox"`
- **THEN** sofka uses the `gruvbox-dark` palette

### Requirement: Automatic dark or light skin

When the base config sets no `skin.name`, sofka SHALL query the terminal background before entering the full-screen interface and use `catppuccin-latte` on a light background and `catppuccin-mocha` otherwise, including when the terminal does not answer.

#### Scenario: Light terminal

- **WHEN** no skin is configured and the terminal reports a light background
- **THEN** sofka uses `catppuccin-latte`

#### Scenario: No answer

- **WHEN** no skin is configured and the terminal does not answer the background query
- **THEN** sofka uses `catppuccin-mocha`

### Requirement: Unknown skin in config

An unknown `skin.name` MUST fall back to `catppuccin-mocha`: at startup sofka SHALL print `warning: unknown skin '<name>' (known: <list>); using catppuccin-mocha` to stderr, and `:reload` SHALL report the config warning `skin.name: unknown skin '<name>' (known: <list>)`. On a context switch to an unknown override skin it SHALL flash `unknown skin '<name>' in config` and keep the current colours.

#### Scenario: Unknown name on reload

- **WHEN** the config sets `[skin] name = "no-such-skin"` and the user runs `:reload`
- **THEN** `:config` lists a warning containing `skin.name` and `no-such-skin`

### Requirement: Per-swatch colour overrides

`[skin.colors]` SHALL override individual swatches of the active skin by swatch name (for example `red`, `mauve`, `text`, `base`) with `#rrggbb` or `rrggbb` values. Unknown swatch names and invalid hex values MUST be ignored with the warnings `skin.colors.<name>: unknown swatch name` and `skin.colors.<name>: invalid hex '<value>' (expected #rrggbb)`.

#### Scenario: Override red

- **WHEN** the config sets `[skin.colors] red = "#fb4934"`
- **THEN** elements drawn in the red swatch use `#fb4934`

#### Scenario: Invalid hex

- **WHEN** the config sets `[skin.colors] green = "zzz"` and the user runs `:reload`
- **THEN** a warning containing `skin.colors.green` and `invalid hex` is reported
- **AND** the green swatch keeps the skin's colour

### Requirement: Skin background fill

`skin.background = true` SHALL fill every view with the active skin's `base` swatch. The default MUST be `false`, leaving the terminal background visible. The setting SHALL follow overrides, context switches and `:reload`.

#### Scenario: Background on

- **WHEN** the config sets `[skin] background = true`
- **THEN** views are filled with the skin's base colour

### Requirement: Skins picker

`:skin` or `:skins` without a name SHALL open a Skins picker listing the built-in skins. `enter` MUST apply the selected skin and `esc` or `q` MUST close the picker without changing the skin.

#### Scenario: Pick a skin

- **WHEN** the user runs `:skin`, selects `nord` and presses `enter`
- **THEN** the colours change to the nord palette
- **AND** sofka flashes `skin: nord`

### Requirement: Apply a skin by name

`:skin <name>` SHALL apply the named built-in skin immediately with the current `[skin.colors]` overrides and flash `skin: <name>`. An unknown name MUST flash `unknown skin: <name>` and keep the current skin.

#### Scenario: Direct skin

- **WHEN** the user runs `:skin dracula`
- **THEN** the colours change to the dracula palette

#### Scenario: Unknown name

- **WHEN** the user runs `:skin neon`
- **THEN** sofka flashes `unknown skin: neon`

### Requirement: Session skin

The session skin SHALL start as the base config `skin.name` or the auto-detected skin. A skin chosen with `:skin` MUST become the session skin for the rest of the session. `:reload` SHALL make the base config `skin.name` the session skin again when one is configured and otherwise keep the current session skin.

#### Scenario: Manual choice survives a context switch

- **WHEN** the user runs `:skin nord` and switches to a context with no skin override
- **THEN** the nord skin stays active

#### Scenario: Reload restores configured skin

- **WHEN** the config sets `[skin] name = "dracula"` and the user picked `nord` with `:skin`
- **AND** the user runs `:reload`
- **THEN** the dracula skin becomes active

### Requirement: Per-cluster and per-context skin

A `skin.name` set by a cluster or context override SHALL be used while that context is active, including at startup, and MUST NOT become the session skin. Switching to a context without a skin override SHALL restore the session skin.

#### Scenario: Prod override

- **WHEN** `clusters/prod-cluster/config.toml` sets `[skin] name = "catppuccin-latte"`
- **AND** the user switches from a dev context to a prod-cluster context and back
- **THEN** latte is active in prod
- **AND** the session skin returns in dev
