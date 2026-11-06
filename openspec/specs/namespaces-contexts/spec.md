# namespaces-contexts Specification

## Purpose

Defines how sofka chooses and switches the namespace and the kubeconfig
context: the `n` namespace switcher, favourite and all-namespaces shortcuts,
the `W` jump, per-context namespace memory, namespace wildcard patterns, the
`:ns` and `:ctx` commands, the context picker and what a context switch keeps
or resets.

## Requirements

### Requirement: Namespace switcher opening

`n` in the resource table SHALL open the namespace switcher with the cursor on the current namespace and fetch the namespace list in the background.

#### Scenario: Open the switcher

- **WHEN** the current namespace is `prod`
- **AND** the user presses `n`
- **THEN** the namespace switcher opens with `prod` highlighted

### Requirement: Namespace switcher ordering

With an empty filter the switcher SHALL list `<all>` first, then configured `favorite_namespaces` in config order (even when not listed by the cluster), then up to 8 namespaces recently picked in this context during the session (newest first, only if they still exist), then the rest alphabetically. The current namespace and the context's default namespace MUST always be included.

#### Scenario: Favourites lead

- **WHEN** `favorite_namespaces = ["prod", "dev"]` is configured
- **AND** the user opens the switcher
- **THEN** the list starts with `<all>`, `prod`, `dev`

#### Scenario: Recents follow favourites

- **WHEN** the user picked `team-b` earlier in this session and it is not a favourite
- **AND** opens the switcher
- **THEN** `team-b` is listed right after the favourites

### Requirement: Namespace switcher labels

The switcher SHALL show each favourite's digit shortcut, tag favourites with ★ and recents with · while the filter is empty, and append "(current)", "(context default)" or "(current, context default)" to the matching entries.

#### Scenario: Current namespace label

- **WHEN** the current namespace is `prod` and the context default is `default`
- **AND** the user opens the switcher
- **THEN** the `prod` entry ends with "(current)"
- **AND** the `default` entry ends with "(context default)"

#### Scenario: Tags hidden while filtering

- **WHEN** `prod` is a favourite
- **AND** the user types `pr` in the switcher
- **THEN** the `prod` entry has no ★ tag

### Requirement: Namespace switcher filtering

Typing in the switcher SHALL fuzzy-filter the namespaces, dropping favourite and recent pinning while keeping `<all>` at the top, and MUST put the cursor on the best real match. Text editing keys MUST update the filter the same way.

#### Scenario: Type to filter

- **WHEN** the switcher lists `<all>`, `kube-system`, `prod`
- **AND** the user types `pro`
- **THEN** `prod` is highlighted

### Requirement: Namespace switcher esc

`esc` in the switcher SHALL clear a non-empty filter and move the cursor to the top; with an empty filter it MUST close the switcher without changing the namespace.

#### Scenario: Two presses to close

- **WHEN** the switcher filter is `pro`
- **AND** the user presses `esc` twice
- **THEN** the first press clears the filter
- **AND** the second press closes the switcher

### Requirement: Namespace switcher accept

`enter` in the switcher SHALL select the highlighted namespace. When the filter has text and matches no real namespace, `enter` MUST use the typed text verbatim as the namespace.

#### Scenario: Unlisted namespace

- **WHEN** the user cannot list namespaces
- **AND** types `secret-team` in the switcher and presses `enter`
- **THEN** the namespace becomes `secret-team`

### Requirement: Digit shortcuts in the switcher

While the switcher filter is empty, digits `1`-`9` SHALL select the corresponding configured favourite and `0` MUST select all namespaces. A digit without a configured favourite MUST be typed into the filter instead.

#### Scenario: Digit picks a favourite

- **WHEN** `favorite_namespaces = ["prod"]` is configured and the switcher filter is empty
- **AND** the user presses `1`
- **THEN** the namespace becomes `prod`

#### Scenario: Unassigned digit filters

- **WHEN** only one favourite is configured and the switcher filter is empty
- **AND** the user presses `5`
- **THEN** the filter becomes `5`

### Requirement: All-namespaces shortcut

`0` in the resource table SHALL switch to all namespaces, flash "namespace: all namespaces", drop any owner scope, remember the choice for the context and record it in view history.

#### Scenario: Zero in the table

- **WHEN** the Pods list shows namespace `prod`
- **AND** the user presses `0`
- **THEN** pods from every namespace are listed
- **AND** the status line shows "namespace: all namespaces"

### Requirement: Favourite shortcuts in the table

`1`-`9` in the resource table SHALL select the first nine configured favourite namespaces in config order; an unconfigured slot MUST do nothing.

#### Scenario: Unconfigured slot

- **WHEN** two favourites are configured
- **AND** the user presses `7` in the table
- **THEN** the namespace does not change

### Requirement: Favourites in the header

The header's `Namespace:` line SHALL show configured favourites with their key, highlighting the current one, and MUST truncate with `…` when the terminal is too narrow.

#### Scenario: Wide terminal

- **WHEN** favourites `prod` and `dev` are configured and the terminal is wide
- **THEN** the header shows `1 prod` and `2 dev`

### Requirement: Selected row namespace

`W` SHALL switch to the namespace of the selected row while keeping the resource kind, using the normal namespace selection. With no selected row it MUST flash "no resource selected"; for a cluster-scoped row it MUST flash "resource has no namespace"; when already in that namespace without an owner scope it MUST only flash "namespace: <ns>".

#### Scenario: Jump to a pod's namespace

- **WHEN** all-namespace Pods are listed and a pod in `prod` is selected
- **AND** the user presses `W`
- **THEN** the Pods list shows namespace `prod`

#### Scenario: Cluster-scoped row

- **WHEN** a node is selected
- **AND** the user presses `W`
- **THEN** the status line shows "resource has no namespace"

### Requirement: Namespace selection effects

Selecting a namespace SHALL clear the switcher filter, drop any owner scope, add it to the context's session recents, remember it for the context, flash "namespace: <label>", put the cursor on the first row, record view history and restart the resource watch in the new namespace.

#### Scenario: Pick from the switcher

- **WHEN** the user selects `dev` in the switcher
- **THEN** the table reloads in `dev` with the cursor on the first row
- **AND** the status line shows "namespace: dev"

### Requirement: All-namespaces spellings

`all`, `*` and `<all>` SHALL all mean all namespaces wherever a namespace is accepted.

#### Scenario: Palette with star

- **WHEN** the user runs `:pods *`
- **THEN** pods from every namespace are listed

### Requirement: Per-context namespace memory

sofka SHALL remember the last namespace selected in each context in `<state-dir>/namespaces.toml`, storing all namespaces as `all` and accepting `*`, `<all>` or an empty value as all namespaces when reading. An empty context name MUST NOT be recorded, and a missing or unreadable file MUST load as empty. A failed write MUST flash "failed to save namespace state: <error>".

#### Scenario: Restart restores the namespace

- **WHEN** the user selected `dev` in context `kind`
- **AND** restarts sofka on context `kind` without `-n` or `-A`
- **THEN** the namespace is `dev`

### Requirement: Namespace precedence on connect

When a context connects, sofka SHALL choose the namespace in this order: an explicit `-n`/`-A` (first successful connection only), the context's kubeconfig namespace when `prefer_context_namespace = true`, the remembered namespace, `default_namespace` from config, then the cluster's default namespace.

#### Scenario: Explicit flag wins once

- **WHEN** sofka starts with `-n ops` and the remembered namespace is `dev`
- **THEN** the first view uses `ops`
- **AND** a later switch back to that context uses the remembered namespace

#### Scenario: Prefer context namespace

- **WHEN** `prefer_context_namespace = true` and the context's kubeconfig namespace is `team-a`
- **AND** a remembered namespace `dev` exists
- **THEN** the namespace is `team-a`

### Requirement: Namespace command

`:ns`, `:namespace` or `:namespaces` without an argument SHALL open the Namespaces list. With an argument from the Namespaces list it MUST select that namespace and return to the previous breadcrumb frame, or open Pods when there is none; elsewhere it MUST select that namespace and keep the current kind.

#### Scenario: From the Namespaces list

- **WHEN** the Namespaces list was opened with `:ns` and no breadcrumb frame exists
- **AND** the user runs `:ns dev`
- **THEN** the Pods list opens in `dev`

#### Scenario: From another kind

- **WHEN** the Deployments list is shown
- **AND** the user runs `:ns dev`
- **THEN** the Deployments list shows namespace `dev`

### Requirement: Kind with namespace command

`:<kind> <namespace>` SHALL switch kind and namespace together as a root view, add the namespace to session recents, remember it for the context and flash "Viewing <Title> in <namespace label>".

#### Scenario: Deployments in prod

- **WHEN** the user runs `:deploy prod`
- **THEN** the Deployments list in `prod` opens
- **AND** the status line shows "Viewing Deployments in prod"

### Requirement: Namespace wildcard patterns

A namespace containing `*` or `?` (other than the all-namespaces spellings) SHALL be a case-sensitive whole-name wildcard pattern matched against the namespace list, flashing "finding namespaces: <pattern>" while resolving. An invalid pattern MUST flash "invalid namespace pattern: <error>".

#### Scenario: Team prefix

- **WHEN** namespaces `team-a`, `team-b` and `ops` exist
- **AND** the user runs `:pods team-*`
- **THEN** pods from `team-a` and `team-b` only are listed

### Requirement: Pattern label

The namespace label for a resolved pattern SHALL read "<pattern> (N namespaces)", with ", incomplete" added when some namespaces could not be watched, and "<pattern> (unresolved)" before it resolves.

#### Scenario: Header label

- **WHEN** `team-*` matched two namespaces
- **THEN** the namespace label reads "team-* (2 namespaces)"

### Requirement: Pattern watches and refresh

A resolved pattern SHALL watch each matched namespace separately, and `ctrl-r` or `r` refresh MUST re-resolve the pattern so new or removed matching namespaces are picked up.

#### Scenario: New namespace after refresh

- **WHEN** `team-*` is selected and namespace `team-c` is created
- **AND** the user presses `ctrl-r`
- **THEN** pods from `team-c` are included

### Requirement: Pattern resolution failures

When a pattern matches nothing or namespace listing fails, sofka SHALL keep the previous selection and flash "no namespaces match <pattern>; selection unchanged" or "cannot select <pattern>: <error>; selection unchanged". Listing MUST time out after 15 seconds with "namespace discovery timed out", and MUST fail with "namespace discovery is unavailable" when the namespaces kind is missing.

#### Scenario: No match

- **WHEN** no namespace matches `zzz-*`
- **AND** the user runs `:pods zzz-*`
- **THEN** the status line shows "no namespaces match zzz-*; selection unchanged"
- **AND** the namespace is unchanged

### Requirement: Unresolved remembered pattern

When a remembered or startup pattern cannot be resolved, sofka SHALL keep the pattern, show an empty table without falling back to another namespace, and flash a message ending "pattern unresolved; no resources loaded".

#### Scenario: Restored pattern fails

- **WHEN** the remembered namespace is `team-*` and namespace listing is forbidden
- **AND** sofka connects to that context
- **THEN** the table is empty
- **AND** the status line ends with "pattern unresolved; no resources loaded"

### Requirement: Context picker

`:ctx` (aliases `:context`, `:contexts`) SHALL open a picker of kubeconfig contexts sorted by name with the cursor on the current context; an unreadable kubeconfig MUST show its error. `:ctx <name>` MUST complete kubeconfig context names and `enter` MUST switch to the highlighted context.

#### Scenario: Open the picker

- **WHEN** the kubeconfig has contexts `prod` and `dev`
- **AND** the user runs `:ctx`
- **THEN** the picker lists `dev` then `prod`

### Requirement: Context picker filtering

In the context picker, typing a character or pressing `/` SHALL start filtering; in filter mode `esc` MUST clear the filter and put the cursor on the current context, and in browse mode `esc` MUST close the picker.

#### Scenario: Esc in filter mode

- **WHEN** the user typed `pr` in the context picker
- **AND** presses `esc`
- **THEN** the filter is cleared and the current context is highlighted
- **AND** the picker stays open

### Requirement: Context rename

`r` or `R` in the context picker SHALL prompt for a new name prefilled with the old one and rename it with `kubectl config rename-context`, flashing "renaming <old> → <new>…" then "renamed context <old> → <new>" or "rename failed: <error>". A name already in use MUST flash "context '<new>' already exists"; renaming MUST be denied in readonly mode.

#### Scenario: Duplicate name

- **WHEN** contexts `dev` and `prod` exist
- **AND** the user renames `dev` to `prod`
- **THEN** the status line shows "context 'prod' already exists"

#### Scenario: Readonly

- **WHEN** sofka runs with `--readonly`
- **AND** the user presses `r` in the context picker
- **THEN** no rename prompt opens

### Requirement: Context switch start

Selecting a context SHALL flash "switching to <name>…", close any PVC browser, clear the table rows and connect in the background. Selecting the current, connected context MUST do nothing, and selecting the context already being switched to MUST NOT start another switch.

#### Scenario: Reselect current context

- **WHEN** the connected context is `dev`
- **AND** the user selects `dev` in the picker
- **THEN** no reconnection happens

### Requirement: Context switch success

After a successful switch sofka SHALL re-resolve the configuration for the new context (aliases, favourites, keys, skin, plugins, bookmarks, workspaces, guardrails, `terminal_title` and others), clear view history, breadcrumbs, filter and cached namespace and permission lists, and flash "context: <name>". The kubeconfig `current-context` MUST NOT change.

#### Scenario: Per-context favourites

- **WHEN** context `prod` has its own `favorite_namespaces`
- **AND** the user switches to `prod`
- **THEN** the table favourite shortcuts use `prod`'s favourites

### Requirement: Readonly recomputed on context switch

A context switch SHALL set readonly mode from the `--readonly`/`--write` command-line flag when given, otherwise from the new context's `readonly` config.

#### Scenario: Readonly context

- **WHEN** no readonly flag was given and context `prod` sets `readonly = true`
- **AND** the user switches to `prod`
- **THEN** mutating actions are denied

### Requirement: Resource kind kept on context switch

After a context switch sofka SHALL keep the previous resource kind when the new cluster serves the same group and plural; otherwise it MUST open `default_resource` (or pods) and warn "<Previous> is unavailable in <name>; viewing <Title>". If neither resolves it MUST flash "No resource matches '<default>' or 'pods' in <name>".

#### Scenario: Kind kept

- **WHEN** the Deployments list is shown
- **AND** the user switches to another context
- **THEN** the Deployments list of the new context is shown

#### Scenario: CRD missing in the new context

- **WHEN** the Widgets list is shown and the new context lacks that CRD
- **AND** the user switches context
- **THEN** the default resource opens
- **AND** a warning says Widgets is unavailable

### Requirement: Pending destinations win on context switch

When a context switch was started by a resource query with `@context`, a bookmark or a workspace, that destination SHALL be applied after connecting instead of keeping the previous kind. Starting another switch MUST drop any destination armed by an earlier one.

#### Scenario: Bookmark on another context

- **WHEN** a bookmark targets Services in context `prod`
- **AND** the user opens it from context `dev`
- **THEN** sofka connects to `prod` and shows Services

### Requirement: Context switch failure

A failed context switch SHALL drop pending destinations, flash "context switch failed: <error>" and stay on the previous context; when no context was ever connected it MUST reopen the context picker.

#### Scenario: Unreachable context

- **WHEN** the user switches to a context whose API server is unreachable
- **THEN** the status line shows "context switch failed: <error>"
- **AND** the previous context stays active

### Requirement: Unreachable context at startup

When the starting context cannot be reached, sofka SHALL open the context picker and flash "cannot connect to '<context>': <error> — pick another context", keeping any `-n`/`-A` for the first successful connection.

#### Scenario: Startup failure

- **WHEN** sofka starts on an unreachable context `old`
- **THEN** the context picker is shown
- **AND** the status line starts with "cannot connect to 'old'"
