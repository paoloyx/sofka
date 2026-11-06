# bookmarks-workspaces Specification

## Purpose

Saved destinations. A `[[bookmarks]]` entry jumps to one resource view (context, namespace, filter, sort and an optional overview). A `[[workspaces]]` entry groups several such views, which the user cycles with Tab/Shift-Tab.

## Requirements

### Requirement: Bookmark configuration fields

A `[[bookmarks]]` entry SHALL accept `name` (required) and `resource` (required). It SHALL also accept these optional fields: `key` (a key chord), `context`, `namespace`, `filter`, `sort` (`COLUMN[:asc|:desc]`) and `view` (`xray` or `pulse`). A `namespace` of `all` or `*` SHALL mean all namespaces.

#### Scenario: Bookmark with all fields

- **WHEN** the config defines a bookmark with a name, resource `pods`, namespace `prod`, filter `status!=Running` and sort `NAME:desc`
- **THEN** the bookmark appears in the palette and in help
- **AND** applying it restores each saved field

#### Scenario: All-namespaces bookmark

- **WHEN** a bookmark sets `namespace = "all"` or `namespace = "*"`
- **THEN** applying it lists the resource across all namespaces

### Requirement: Bookmark configuration warnings

Invalid bookmark entries SHALL produce non-fatal warnings, shown with the other config warnings:

- `bookmark: missing \`name\``
- `bookmark "N": missing \`resource\``
- `bookmark "N": invalid key — <error>`
- `bookmark "N": unknown view "V" (expected xray/pulse) — ignored`

#### Scenario: Unknown view

- **WHEN** a bookmark sets `view = "graph"`
- **THEN** a warning names the unknown view and says it is ignored
- **AND** sofka still starts

### Requirement: Bookmark key chord

When a key press matches a bookmark's `key` chord and no built-in binding handles it, sofka SHALL apply the first bookmark whose chord matches. Bookmark chords SHALL be tried before workspace chords and before plugin chords.

#### Scenario: Chord opens the bookmark

- **WHEN** a bookmark binds `ctrl-y` and the user presses Ctrl+Y in the table
- **THEN** the bookmark is applied

#### Scenario: Bookmark beats plugin on the same chord

- **WHEN** a bookmark and a plugin both bind the same chord
- **THEN** pressing it applies the bookmark and runs no plugin

### Requirement: Bookmarks in the command palette

The `:` palette SHALL list every bookmark by name with a `★` marker, even when the query is empty. Bookmarks SHALL rank above resource suggestions. Submitting a bookmark name SHALL apply that bookmark.

#### Scenario: Palette shows bookmarks

- **WHEN** the user opens `:` with at least one bookmark configured
- **THEN** the suggestion list shows each bookmark with `★`, above the resource kinds

#### Scenario: Palette applies a bookmark

- **WHEN** the user types a bookmark name in the palette and presses Enter
- **THEN** the bookmark is applied

### Requirement: Bookmark application order

Applying a bookmark in the current context SHALL first validate it and change nothing if validation fails. It SHALL then:

1. switch to the resource with the bookmark's namespace and filter;
2. apply the bookmark's sort;
3. open its `view` (`xray` or `pulse`) if one is set;
4. flash `bookmark: <name>`.

#### Scenario: Resource, namespace, filter and sort

- **WHEN** the user applies a bookmark for `pods` in `prod` with filter `status!=Running` and sort `NAME:desc`
- **THEN** the table shows pods in `prod` filtered by `status!=Running`
- **AND** the table is sorted by NAME descending
- **AND** the status line reads `bookmark: <name>`

### Requirement: Bookmark validation failures keep the current view

If a bookmark has an empty resource, a resource that cannot be resolved, or an invalid filter, sofka SHALL flash a warning and SHALL leave the resource, namespace, filter and sort unchanged. The warnings are `bookmark has no resource`, `No resource matches 'R'` and `filter: <error>`.

#### Scenario: Unknown resource

- **WHEN** a bookmark names resource `nonexistentcrd`
- **THEN** the flash reads `No resource matches 'nonexistentcrd'`
- **AND** the current table, namespace, filter and sort stay as they were

#### Scenario: Invalid filter

- **WHEN** a bookmark's filter is `(`
- **THEN** the flash starts with `filter:`
- **AND** the view does not change

### Requirement: Bookmark sort waits for its column

A bookmark sort SHALL match the column header case-insensitively. If the column is not currently displayed, the sort SHALL stay pending and take effect once the column appears (for example in wide mode). A pending sort SHALL NOT overwrite remembered sort preferences.

#### Scenario: Sort by a wide-only column

- **WHEN** a pods bookmark sorts by `IP:desc` while the IP column is hidden
- **THEN** no column is sorted at first
- **AND** pressing `w` sorts by IP descending
- **AND** the remembered sort for pods is unchanged

### Requirement: Bookmark context switch

When a bookmark names a `context` different from the current one, sofka SHALL switch to that context first. It SHALL apply the rest of the bookmark only after the new context connects. If the switch fails or is cancelled, the pending bookmark SHALL be dropped.

#### Scenario: Cross-context bookmark

- **WHEN** the user applies a bookmark for context `prod` while connected to `dev`
- **THEN** sofka switches to `prod`
- **AND** once connected, the bookmark's resource, namespace, filter and sort are applied

#### Scenario: Failed switch

- **WHEN** the context switch for a bookmark fails
- **THEN** the bookmark is not applied later

### Requirement: Workspace configuration fields

A `[[workspaces]]` entry SHALL accept `name`, an optional `key` and `context`, and an ordered list of `[[workspaces.views]]`. Each view SHALL accept `name` and `resource`, plus optional `namespace`, `filter`, `sort` and `view` fields that mean the same as the bookmark fields.

#### Scenario: Two-view workspace

- **WHEN** a workspace `ops` defines a view `pods` (namespace `checkout`) and a view `deploys` (resource `deployments`)
- **THEN** the workspace appears in the palette and in help with 2 views

### Requirement: Workspace configuration warnings

Invalid workspaces SHALL produce non-fatal warnings:

- `workspace: missing \`name\``
- `workspace "N": no [[workspaces.views]]`
- `workspace "N": a view is missing \`resource\``
- an unknown `view` value, with the suffix `(expected xray/pulse) — ignored`
- an invalid `key`

#### Scenario: Workspace without views

- **WHEN** a workspace defines no `[[workspaces.views]]`
- **THEN** the config warnings include one saying it has no `[[workspaces.views]]`

### Requirement: Opening a workspace

A workspace SHALL open from its `key` chord or from its name in the `:` palette. Its chord SHALL be tried after bookmark chords and before plugin chords. The palette SHALL list it with a `▦` marker. Opening a workspace SHALL apply its first view and flash `workspace <name> [1/<n>]: <view name>`. A workspace with no views SHALL instead flash `workspace '<name>' has no views` and change nothing.

#### Scenario: Chord opens first view

- **WHEN** workspace `ops` binds `ctrl-w` and the user presses Ctrl+W
- **THEN** the table shows the first view's resource and namespace
- **AND** the flash contains `[1/2]`

#### Scenario: Empty workspace

- **WHEN** the user opens a workspace with no views
- **THEN** the flash reads `workspace '<name>' has no views`

### Requirement: Workspace context switch

When a workspace names a `context` different from the current one, sofka SHALL switch context once. It SHALL open the first view only after the new context connects. Cycling between views SHALL NOT switch context again.

#### Scenario: Cross-context workspace

- **WHEN** the user opens a workspace for context `prod` while connected to `dev`
- **THEN** sofka switches to `prod`
- **AND** the first view opens after the connection succeeds

### Requirement: Workspace view cycling

While a workspace is active:

- Tab SHALL apply the next view and Shift-Tab the previous one, wrapping at both ends.
- Each step SHALL flash `workspace <name> [i/n]: <view name>`.
- A view with no `namespace` SHALL keep the current namespace.

When no workspace is active, Tab/Shift-Tab keep their default behaviour (see the navigation capability).

#### Scenario: Tab wraps

- **WHEN** a two-view workspace is on view 1 and the user presses Tab twice
- **THEN** the first press shows view 2 with `[2/2]`
- **AND** the second press returns to view 1 with `[1/2]`

#### Scenario: Shift-Tab goes back

- **WHEN** the workspace is on view 1 and the user presses Shift-Tab
- **THEN** the last view is shown

### Requirement: Rejected workspace view keeps the previous view

When a workspace view fails bookmark validation (an empty or unknown resource, or an invalid filter), sofka SHALL show the matching warning. It SHALL keep the previous resource, namespace, filter, sort and table mode.

#### Scenario: Invalid second view

- **WHEN** view 2 of the active workspace has an invalid filter and the user presses Tab
- **THEN** the flash is an error starting with `filter:`
- **AND** the table still shows view 1's resource, namespace, filter and sort

### Requirement: Workspace stays active

Once a workspace is opened, it SHALL stay the active cycling target until another workspace is opened. Ordinary navigation SHALL NOT leave it. The table hint SHALL read `Tab/Shift-Tab: workspace` while a workspace is active, and `Tab/Shift-Tab: resources` otherwise.

#### Scenario: Hint changes

- **WHEN** no workspace has been opened
- **THEN** the table hint mentions `resources`
- **AND** after a workspace is opened, the hint mentions `workspace`

### Requirement: Help lists bookmarks and workspaces

The `?` help SHALL list each bookmark as `<chord or :> ★ <name>`. It SHALL list each workspace as `<chord or :> ▦ <name> (<n> views · Tab to cycle)`. The entry SHALL show `:` when no chord is set.

#### Scenario: Help entries

- **WHEN** one bookmark without a key and one workspace bound to `ctrl-w` are configured, and the user presses `?`
- **THEN** help shows a Bookmarks section with `: ★ <name>`
- **AND** a Workspaces section with the Ctrl+W label and the view count
