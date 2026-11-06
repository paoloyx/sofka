## ADDED Requirements

### Requirement: Half-page scroll actions

sofka SHALL provide the `half_page_down` and `half_page_up` actions in every scope that has `page_down` and `page_up`, moving half of the view's page, at least one row or line, in tables, document views, logs, help, drain reviews and pickers. Both actions MUST be unbound by default. Set in `[keys.navigation]`, they SHALL also reach text-input screens that page, such as the namespace, context, sort and copy pickers, where an inherited chord that types text or that the screen already uses MUST be skipped; a setting in the screen's own scope MUST be used as written.

#### Scenario: Unbound by default

- **WHEN** no half-page keys are configured
- **AND** the user presses `ctrl-d` on a pod row
- **THEN** the delete confirmation opens
- **AND** help shows `unbound` for `half_page_down`

#### Scenario: Half page in the table

- **WHEN** the config sets `[keys.navigation] half_page_down = "ctrl-d"` and `[keys.table] delete = "alt-d"`
- **AND** the table shows 8 rows per page with the cursor on row 1
- **AND** the user presses `ctrl-d`
- **THEN** the cursor is on row 5

#### Scenario: Picker filter keeps its line keys

- **WHEN** the config sets `[keys.navigation] half_page_up = "ctrl-u"` and `half_page_down = "ctrl-d"` and `[keys.table] delete = "alt-d"`
- **AND** the user presses `ctrl-d` in the namespace picker
- **THEN** the selection moves down half a page
- **AND** `ctrl-u` still clears the filter text

#### Scenario: Plain key stays text in a picker

- **WHEN** the config sets `[keys.navigation] half_page_down = ["%", "alt-j"]`
- **AND** the namespace picker is open with an empty filter
- **THEN** pressing `alt-j` moves the selection down half a page
- **AND** pressing `%` types `%` into the filter

## MODIFIED Requirements

### Requirement: Key scopes

sofka SHALL accept the shared scopes `global`, `navigation` and `input` and one scope per screen, including `table`, `detail`, `diff`, `events`, `logs`, `help`, `containers`, `confirm`, `prompt`, `pulse`, `xray`, `rbac`, `explain`, `timeline`, `gitops`, `argocd`, `adjacent`, `skins`, `snapshots`, `fleet`, `find`, `pvc_explore`, `dir`, `contexts`, `command`, `filter`, `namespaces`, `drain` and the other picker and form scopes. An unknown scope MUST be rejected with `keys.<scope>: unknown scope`.

#### Scenario: Unknown scope

- **WHEN** the config contains `[keys.tabel]`
- **THEN** sofka reports `keys.tabel: unknown scope`

### Requirement: Navigation action scope

The navigation actions `up`, `down`, `first`, `last`, `page_up`, `page_down`, `half_page_up`, `half_page_down`, `left`, `right`, `back`, `close`, `command` and `help` SHALL be accepted in `[keys.navigation]` and MUST apply to non-text screens only, except that `back` in the confirm dialog is configured by `[keys.confirm]` and the half-page actions also reach text-input screens that page.

#### Scenario: Navigation does not touch text input

- **WHEN** the config sets `[keys.navigation] down = "j"`
- **AND** the user types `j` in the command palette
- **THEN** `j` is inserted into the command text

### Requirement: Key labels in header, footer and help

sofka SHALL show bindings with canonical labels such as `ctrl-alt-delete` and `space`. The header and footer hints MUST show the first chord of an action, the help panel MUST show all chords joined with `/`, and an unbound action MUST be shown as `unbound`. Hint keys in the header, the status bar and popup footers, including the fixed keys spelled out in the drain and command failure footers, MUST be drawn bold in the skin's key colour while their labels keep the surrounding style.

#### Scenario: Custom key in the footer

- **WHEN** the config sets `[keys.table] yaml = ["ctrl-y", "y"]`
- **THEN** the footer shows `ctrl-y` for the YAML action
- **AND** help shows `ctrl-y / y`

#### Scenario: Styled key in the status bar

- **WHEN** the pods table is shown
- **THEN** the status bar hint `::command` draws its `:` key bold in the skin's key colour
- **AND** the rest of the hint keeps the dim style without bold

#### Scenario: Fixed keys in the drain footer

- **WHEN** the drain typed-confirmation footer shows "Enter: confirm   Esc: cancel"
- **THEN** `Enter` and `Esc` are bold in the skin's key colour
- **AND** the rest of the footer is not bold

### Requirement: Help panel contents

`?` SHALL open a help panel that lists, for each scope, a `Keys: <scope>` heading with every action's keys and description from the active keymap, followed by the command palette commands, configured bookmark and workspace keys, and the help navigation keys. Configured plugins MUST be listed in a `Plugins` section right after `Keys: table`, each with its key and its `:<command>` palette command where set, its name and its scopes (`all resources` when unscoped). The section order MUST be the same from every view; opened from the table, help MUST list the plugins scoped to the current kind or to all resources before the other plugins.

#### Scenario: Rebound key in help

- **WHEN** the config sets `[keys.table] sort_age = "ctrl-a"` and the user opens help
- **THEN** the `Keys: table` section shows `ctrl-a` for the age sort

#### Scenario: Plugins after the table keys

- **WHEN** plugins scoped to `deployments`, to `pods` and to all resources are configured in that order
- **AND** the user opens help from the Pods table
- **THEN** the `Plugins` section follows the `Keys: table` section
- **AND** it lists the pods plugin and the all-resources plugin before the deployments plugin

#### Scenario: Plugins in config order outside the table

- **WHEN** the same plugins are configured
- **AND** the user opens help from the logs view
- **THEN** the `Plugins` section lists them in config order

### Requirement: Help panel search

In the help panel `/` SHALL start a search that keeps only matching lines together with their section headings and titles the panel `Help · /<text> [<matches>]`. A search that matches a section heading MUST keep that whole section. `esc` MUST clear an active search first and close help otherwise, and `q` or `?` MUST close help. Starting a search and clearing a search SHALL reset the scroll to the top.

#### Scenario: Search keeps headings

- **WHEN** the user opens help, presses `/` and types `marker`
- **THEN** the log marker binding is shown under its `Keys: logs` heading

#### Scenario: Esc clears then closes

- **WHEN** a help search is active and the user presses `esc` twice
- **THEN** the first `esc` clears the search and the second closes help

#### Scenario: Search matching a heading

- **WHEN** the user opens help, presses `/` and types `table`
- **THEN** every binding of the `Keys: table` section is shown, including the age sort
- **AND** the `Keys: detail` section is not shown

### Requirement: Help panel scrolling

The help panel SHALL open scrolled to the `Keys:` section of the view it was opened from, keeping the same section order from every view, or at the top when that view has no section. It SHALL scroll one line with `j`/`k` or the arrows, one visible page forward with `ctrl-f`, `PgDn` or `space`, one page back with `ctrl-b` or `PgUp`, and to the top or bottom with `g`/`Home` and `G`/`End`. `next_section` (`]`) MUST jump to the next section heading, and `previous_section` (`[`) to the previous heading, or to the start of the current section when the panel is scrolled inside it.

#### Scenario: Page forward

- **WHEN** the help panel is longer than the screen and the user presses `space`
- **THEN** the content moves forward by the number of visible rows

#### Scenario: Help from the logs view

- **WHEN** the logs view is open and the user presses `?`
- **THEN** the help panel opens with the `Keys: logs` heading as its first visible line

#### Scenario: Previous section from inside a section

- **WHEN** the help panel is scrolled one line below the `Keys: detail` heading
- **AND** the user presses `[`
- **THEN** the `Keys: detail` heading is the first visible line again

### Requirement: Kind-specific table actions

Kind-specific table actions SHALL take priority only on their kinds: `attach` and `previous_logs` on pods; `port_forward` on pods and services; `inspect` on secrets and PVCs; `cordon`, `uncordon` and `drain` on nodes; `approve` and `deny` on certificatesigningrequests; `ns_resources` on namespaces; `toggle_owned` on the `:workloads` view; `set_image` on pods, pod-owning workload kinds and Flux HelmReleases. A `helmreleases` kind MUST count only in the Flux group `helm.toolkit.fluxcd.io`. On other kinds the same chord MUST go to a bookmark, then a workspace, then a plugin bound to it. The `faults` action, which applies to pods, the workload kinds, the `:workloads` view and the Flux, Flux Operator and Argo CD kinds, MUST yield to a bookmark, workspace or plugin bound to its chord on every kind.

#### Scenario: Plugin key on another kind

- **WHEN** a plugin is bound to `C`, the default `cordon` key
- **AND** the user presses `C` on a deployment
- **THEN** the plugin runs

#### Scenario: Built-in wins on its kind

- **WHEN** a plugin is bound to `C`, the default `cordon` key
- **AND** the user presses `C` on a node
- **THEN** the node is cordoned instead of running the plugin

#### Scenario: HelmRelease outside Flux

- **WHEN** a plugin scoped to `helmreleases` is bound to `i`, the default `set_image` key
- **AND** the user presses `i` on a HelmRelease whose group is not `helm.toolkit.fluxcd.io`
- **THEN** the plugin runs

#### Scenario: Faults yields to a plugin on pods

- **WHEN** a plugin is bound to `ctrl-z`, the default `faults` key
- **AND** the user presses `ctrl-z` on a pod
- **THEN** the plugin runs and the faults filter does not change

### Requirement: Skins picker

`:skin` or `:skins` without a name SHALL open a Skins picker listing the built-in skins, with the active skin selected, also when it was set by an alias such as `mocha`. Moving the selection MUST apply the highlighted skin with the current `[skin.colors]` overrides as a preview, without making it the session skin. `enter` MUST keep the selected skin as the session skin and flash `skin: <name>`, and `esc` or `q` MUST close the picker and restore the skin that was active when it opened. Leaving the picker for the palette or help and returning MUST keep the preview, and reopening `:skin` during a preview MUST keep the original skin to restore. A palette command that leaves the picker, or a `:reload` whose configured skin is unknown, MUST restore the original skin, while `:skin <name>` or a context skin replaces it.

#### Scenario: Pick a skin

- **WHEN** the user runs `:skin`, selects `nord` and presses `enter`
- **THEN** the colours change to the nord palette
- **AND** sofka flashes `skin: nord`

#### Scenario: Preview and cancel

- **WHEN** `catppuccin-mocha` is active and the user runs `:skin`
- **AND** moves the selection to `catppuccin-latte`
- **THEN** the latte colours are shown while the picker stays open
- **AND** `esc` closes the picker and restores the mocha colours

#### Scenario: Command leaves the picker

- **WHEN** the user previews `catppuccin-latte` in the Skins picker opened over `catppuccin-mocha`
- **AND** runs `:pods` from the palette
- **THEN** the mocha colours are restored
- **AND** the session skin is still `catppuccin-mocha`
