# document-views Specification

## Purpose

Document views show one Kubernetes object as scrollable text: its YAML, its description, its decoded Secret data, a diff against a baseline, or a live list of its events. They share scrolling, search, wrap, fullscreen, copy, and optional automatic refresh, and always return to the view that opened them.

## Requirements

### Requirement: YAML view

Pressing `y` on a resource row SHALL open a YAML document of the selected object titled `<name> — YAML`, with `metadata.managedFields` removed and with `apiVersion` and `kind` added when the cached object lacks them.

#### Scenario: Open YAML of a pod

- **WHEN** a pod row named `web` is selected in the table
- **AND** the user presses `y`
- **THEN** a document titled `web — YAML` opens showing the pod as YAML
- **AND** no `managedFields` entry appears in the document

### Requirement: Managed fields toggle

In a YAML view, `m` SHALL toggle `managedFields`: turning it on reads the resource from the API once, flashing `managedFields: on (reading resource)` and then `managedFields: on`; turning it off re-renders at once with the flash `managedFields: off`. `m` SHALL have no effect in other document types.

#### Scenario: Show and hide managed fields

- **WHEN** a YAML view is open
- **AND** the user presses `m`
- **THEN** the resource is read from the API and the document shows its `managedFields`
- **AND** pressing `m` again hides them with the flash `managedFields: off`

### Requirement: Managed fields read errors

When reading the resource for `managedFields` fails, the view SHALL flash `cannot show managedFields: <error>` as an error, keep `managedFields` hidden, and allow a retry with `m`. A read result that arrives after the fields were hidden or the view was closed MUST be ignored.

#### Scenario: Managed fields read fails

- **WHEN** a YAML view is open
- **AND** the user presses `m` and the API read fails
- **THEN** the flash shows `cannot show managedFields: <error>`
- **AND** the document still hides `managedFields`

### Requirement: Managed fields and auto-refresh

Toggling `managedFields` while automatic refresh runs SHALL keep refresh running with the new choice, and YAML refreshes SHALL keep `managedFields` hidden unless the user turned them on.

#### Scenario: Refresh keeps the managed fields choice

- **WHEN** a YAML view has automatic refresh on and `managedFields` hidden
- **AND** a refresh result arrives
- **THEN** the refreshed document still hides `managedFields`

### Requirement: Decoded Secret view

Pressing `x` on a core Secret row, or inside a YAML or describe view of a core Secret, SHALL open a document titled `<name> - decoded` listing each `data` key as `key: value` with the base64 value decoded.

#### Scenario: Decode a secret from the table

- **WHEN** a Secret row with `data.password` set to the base64 of `hunter2` is selected
- **AND** the user presses `x`
- **THEN** a document titled `<name> - decoded` shows `password: hunter2`

#### Scenario: Decode from inside the detail view

- **WHEN** the YAML view of a core Secret is open
- **AND** the user presses `x`
- **THEN** the view switches to the decoded Secret document

### Requirement: Decoded value rendering

The decoded Secret view SHALL render a multi-line value as `key: |` followed by indented lines with trailing newlines trimmed, and SHALL show `<invalid base64>`, `<binary: N bytes>` or `<not a string>` instead of a value that cannot be shown as text.

#### Scenario: Multi-line and binary values

- **WHEN** a Secret has a multi-line PEM value and a non-UTF-8 value
- **AND** the user opens the decoded view
- **THEN** the PEM key renders as `key: |` with indented lines
- **AND** the binary key renders as `key: <binary: N bytes>`

### Requirement: Secret without data

Pressing `x` on a Secret with no `data` entries SHALL flash the warning `secret has no data` and SHALL NOT open a document.

#### Scenario: Empty secret

- **WHEN** a Secret row without data is selected
- **AND** the user presses `x`
- **THEN** the flash warns `secret has no data`
- **AND** the table stays open

### Requirement: Describe with kubectl

By default, `d` SHALL run `kubectl describe <resource> <name>` (with `-n <namespace>` for namespaced objects), showing `loading describe...` and the flash `describing <name>…` while it runs, then the output in a document titled `<name> — describe`.

#### Scenario: Describe a pod

- **WHEN** a pod row named `web` is selected and native describe is off
- **AND** the user presses `d`
- **THEN** a document shows `loading describe...`
- **AND** when kubectl finishes the document titled `web — describe` shows its output

### Requirement: Describe fallback to YAML

In the default kubectl mode, when `kubectl describe` fails or cannot start, the view SHALL show the cached object's YAML titled `<name> — YAML` and warn `kubectl describe failed (<first stderr line>); showing YAML` or `kubectl describe could not start: <error>; showing YAML`. Refreshing that document SHALL then read YAML.

#### Scenario: kubectl describe fails

- **WHEN** native describe is off
- **AND** the user presses `d` and kubectl exits with an error
- **THEN** the document title is `<name> — YAML` and it shows the cached YAML
- **AND** the flash warns `kubectl describe failed (...); showing YAML`

### Requirement: Native describe opt-in

Native describe SHALL be enabled when either the `--experimental-describe` flag or `[experimental] native_describe = true` is set, and `:reload` SHALL apply a config change. When enabled, `d` on a supported resource SHALL read the object and its events from the API and render the description without running kubectl.

#### Scenario: Enable native describe in config

- **WHEN** `[experimental] native_describe = true` is set and the config is reloaded
- **AND** the user presses `d` on a pod
- **THEN** the description is built from API reads and kubectl is not run

### Requirement: Native describe errors

When a native describe read fails, the document SHALL show `Describe failed: <error>` with an error flash and MUST NOT fall back to cached YAML; refreshing SHALL retry natively.

#### Scenario: Forbidden secret read

- **WHEN** native describe is on
- **AND** the user presses `d` on a Secret and the API answers 403
- **THEN** the document shows `Describe failed: ...`
- **AND** no Secret value from the cache appears

### Requirement: Native describe secret values

Native describe of a Secret MUST NOT print data values, except the `token` of a `kubernetes.io/service-account-token` Secret.

#### Scenario: Opaque secret description

- **WHEN** native describe is on
- **AND** the user describes an Opaque Secret with a `password` key
- **THEN** the description does not contain the password value

### Requirement: Native describe kubectl fallback

When native describe is on but the resource type has no native renderer, `d` SHALL title the document `<name> — describe (kubectl fallback)`, show `native description unavailable; loading kubectl fallback...`, and run kubectl; if kubectl fails the document SHALL show `Describe failed: <error>` without YAML.

#### Scenario: Unsupported resource type

- **WHEN** native describe is on
- **AND** the user presses `d` on a resource type without a native renderer
- **THEN** the title is `<name> — describe (kubectl fallback)`
- **AND** a kubectl failure shows `Describe failed: <error>` instead of YAML

### Requirement: Late describe results

A describe result that arrives after the user left the document SHALL be dropped and MUST NOT reopen or change any view.

#### Scenario: Close before describe finishes

- **WHEN** the user presses `d` and then `esc` before the description is ready
- **AND** the description result arrives
- **THEN** the previous view stays open unchanged

### Requirement: Diff baseline selection

`:diff` SHALL compare the selected object against its `kubectl.kubernetes.io/last-applied-configuration` annotation (label `last-applied`) when present, otherwise against the previous revision seen in this session (label `session: previous`).

#### Scenario: Last-applied baseline wins

- **WHEN** the selected object has a last-applied annotation and a previous session revision
- **AND** the user runs `:diff`
- **THEN** the document title is `<name> — diff (last-applied → live)`

#### Scenario: Session revision baseline

- **WHEN** the selected object has no last-applied annotation but changed during the session
- **AND** the user runs `:diff`
- **THEN** the document title is `<name> — diff (session: previous → live)`

### Requirement: Diff without baseline

`:diff` on an object with no last-applied annotation and no change seen this session SHALL flash the warning `nothing to diff: no last-applied annotation, and no change seen this session` and SHALL NOT open a document.

#### Scenario: No baseline available

- **WHEN** the selected object has no annotation and no previous revision
- **AND** the user runs `:diff`
- **THEN** the flash shows the `nothing to diff` warning

### Requirement: Diff rendering

The diff SHALL compare YAML with `managedFields`, the last-applied annotation, and `resourceVersion` removed from the live side, prefix each line with `-`, `+` or a space, and color added lines green and removed lines red. When nothing differs it SHALL still open and flash `no diff: live matches <label>`.

#### Scenario: Changed replica count

- **WHEN** the baseline has `replicas: 1` and the live object has `replicas: 3`
- **AND** the user runs `:diff`
- **THEN** a red line starting with `-` shows `replicas: 1`
- **AND** a green line starting with `+` shows `replicas: 3`

### Requirement: Diff baseline reset

In a diff view, `R` SHALL make the displayed live object the new baseline, retitle the view `<name> - diff (reset baseline → live)`, and flash `diff baseline reset to the displayed resource`. Automatic refresh SHALL keep the baseline until it is reset.

#### Scenario: Reset then refresh

- **WHEN** a diff view is open with automatic refresh on
- **AND** the user presses `R`
- **THEN** later refreshes diff the live object against the reset baseline

### Requirement: Events view

Pressing `E` on a selected object SHALL open a live view titled `<name> — events` that watches the object's events by UID (or by name and namespace without a UID), shows `loading events…` until the first sync, and flashes `events: <name>`.

#### Scenario: Events of a pod

- **WHEN** a pod row named `web` is selected
- **AND** the user presses `E`
- **THEN** a view titled `web — events` opens and lists that pod's events

### Requirement: Events view preconditions

`E` SHALL warn `no selection for events` when no row is selected and `events kind unavailable` when the cluster serves no events resource.

#### Scenario: No selection

- **WHEN** the table has no selected row
- **AND** the user presses `E`
- **THEN** the flash warns `no selection for events`

### Requirement: Events table layout

The events view SHALL show a header with `LAST SEEN`, `TYPE`, `REASON`, `COUNT` and `MESSAGE`, one row per event sorted newest first, times as `YYYY-MM-DD HH:MM:SS`, newlines in messages replaced by spaces, and `(no events)` when there are none.

#### Scenario: Two events

- **WHEN** an object has a Normal event at 10:00 and a Warning event at 10:05
- **AND** the events view is open
- **THEN** the Warning row is listed above the Normal row

### Requirement: Events time and count sources

Each event row SHALL take its time from the last timestamp, then the event time, then the first timestamp, then the creation time, preferring the series' last observed time for `events.k8s.io` events, and SHALL show a count of 1 when the event has none.

#### Scenario: Event without lastTimestamp

- **WHEN** an event has only `eventTime`
- **AND** the events view is open
- **THEN** its LAST SEEN column shows `eventTime`

### Requirement: Events watch errors

A failing events watch SHALL show `error: <error>` in the view, except an expired resource version (410) error, which SHALL be skipped while the watch resyncs.

#### Scenario: Watch forbidden

- **WHEN** the events watch returns a forbidden error
- **AND** the events view is open
- **THEN** the view shows a line `error: <error>`

### Requirement: Events palette command

The `:events` palette command SHALL open the events resource table, not the per-object events view; only `E` SHALL open the events view.

#### Scenario: Palette and E differ

- **WHEN** the user runs `:events` from a pod table
- **THEN** the table switches to the `events.k8s.io` events resource
- **AND** pressing `E` on a pod row later opens `<name> — events`

### Requirement: Document scrolling

In YAML, describe, decoded, diff and events views, `j`/down and `k`/up SHALL scroll one line, `pagedown`/`space`/`ctrl-f` and `pageup`/`ctrl-b` SHALL scroll 20 lines, `g`/home SHALL go to the top and the left edge, and `G`/end SHALL go to the bottom with the last page kept full.

#### Scenario: Page through a long document

- **WHEN** a 200-line YAML view is open at the top
- **AND** the user presses `ctrl-f`
- **THEN** the view starts at line 21

### Requirement: Horizontal scrolling

With wrap off, `h`/left and `l`/right SHALL scroll a document 5 terminal columns; with wrap on they SHALL have no effect.

#### Scenario: Scroll a long line

- **WHEN** wrap is off in a document with a 300-column line
- **AND** the user presses `l`
- **THEN** the view shifts 5 columns to the right

### Requirement: Document wrap

`w` SHALL toggle line wrap in document views with the flash `wrap: on` or `wrap: off`, keep the current source line at the top, and reset horizontal scroll. The choice SHALL persist for later documents in the session and SHALL start from the `detail_wrap` setting.

#### Scenario: Wrap persists

- **WHEN** the user presses `w` in a YAML view to turn wrap on
- **AND** then opens describe on another object
- **THEN** the describe view is wrapped

### Requirement: Document search

`/` SHALL open a case-insensitive plain-substring search that highlights matches while the full document stays visible, shows `/<query> [cur/total]` or `[no matches]` in the title, and on `enter` jumps to the first match at or after the current line, wrapping around.

#### Scenario: Search a YAML document

- **WHEN** a YAML view is open
- **AND** the user types `/image` and presses `enter`
- **THEN** all lines stay visible with `image` highlighted
- **AND** the view jumps to the next match and the title shows `[cur/total]`

### Requirement: Search match stepping

`n` and `N` SHALL move to the next and previous match, wrapping at the ends, and SHALL do nothing when no search is active.

#### Scenario: Step through matches

- **WHEN** a search has 3 matches and the current match is the third
- **AND** the user presses `n`
- **THEN** the current match becomes the first

### Requirement: Search clearing

`esc` while typing a search SHALL clear the query; `esc` in a document with an active search SHALL clear the search before it closes the view. Opening a new document SHALL clear the search, while a refresh SHALL keep it.

#### Scenario: Esc clears before closing

- **WHEN** a document has an active search
- **AND** the user presses `esc`
- **THEN** the search is cleared and the document stays open

### Requirement: Document copy

`c` in a document view SHALL copy the whole document to the clipboard, even with an active search, and flash `copied N lines`, or warn `nothing to copy` when the document is empty.

#### Scenario: Copy with an active search

- **WHEN** a 40-line YAML view has a search matching 2 lines
- **AND** the user presses `c`
- **THEN** all 40 lines are copied

### Requirement: Document fullscreen

`F` SHALL toggle a session-wide fullscreen mode for document views with the flash `fullscreen: on` or `fullscreen: off`, hiding header, status, hints, borders and scrollbars while keeping the title and any search or command prompt. It SHALL be independent of log fullscreen.

#### Scenario: Fullscreen survives a new document

- **WHEN** the user presses `F` in a YAML view
- **AND** then opens describe for another object
- **THEN** the describe view is also fullscreen

### Requirement: Document styling

YAML and describe documents SHALL color keys, section headers, comments, numbers, booleans and null-like values, and known status words, and MUST strip ANSI escape codes from command output.

#### Scenario: Colored YAML

- **WHEN** a YAML view shows `replicas: 3`
- **THEN** `replicas` and `3` are drawn in different colors

### Requirement: Return from documents

`q` SHALL close a document view, and `esc` SHALL close it when no search is active, stopping its event watch or refresh and returning to the view that opened it; returning to the table SHALL reselect the same row.

#### Scenario: Back to the same row

- **WHEN** the user opens YAML on the third row
- **AND** presses `q`
- **THEN** the table is shown with the third row selected

### Requirement: Automatic refresh toggle

In YAML, describe, decoded and diff views, `r` SHALL toggle automatic refresh: on reads the resource at once and then 5 seconds after each result, flashing `automatic refresh: on (5s)`; off flashes `automatic refresh: off`. Documents that cannot refresh SHALL warn `refresh is unavailable for this view`. Refresh SHALL start off for every new document.

#### Scenario: Turn refresh on

- **WHEN** a describe view is open
- **AND** the user presses `r`
- **THEN** the flash shows `automatic refresh: on (5s)` and the resource is read again

### Requirement: Refresh status indicator

The status line SHALL show `● refresh` while automatic refresh runs, `○ stopped` when the document can refresh but is not, and `○ static` for documents that cannot refresh.

#### Scenario: Stopped document

- **WHEN** a YAML view is open with automatic refresh off
- **THEN** the status shows `○ stopped`

### Requirement: Refresh failures

A failed refresh SHALL stop automatic refresh, keep the last content, and flash `refresh stopped: <error>`; when the resource was replaced by a new UID the error SHALL say `<plural>/<name> was replaced; return to the table and select the new resource`.

#### Scenario: Resource replaced

- **WHEN** automatic refresh runs on a pod's YAML
- **AND** the pod is recreated with a new UID
- **THEN** refresh stops with the `was replaced` message and the old YAML stays

### Requirement: Refresh lifecycle

Automatic refresh SHALL stop when the user leaves the document or opens help or the command palette, SHALL keep running during a document search, and results from a stopped refresh MUST be ignored.

#### Scenario: Palette stops refresh

- **WHEN** automatic refresh runs in a YAML view
- **AND** the user opens the command palette
- **THEN** refresh stops

### Requirement: Decoded secret refresh

Refreshing the decoded Secret view SHALL show rotated values and SHALL show `secret has no data` when the data was removed.

#### Scenario: Secret rotated

- **WHEN** a decoded Secret view has automatic refresh on
- **AND** the Secret value changes
- **THEN** the next refresh shows the new value

### Requirement: Text selection in documents

Document views SHALL release mouse capture so the terminal's own text selection works, with the mouse wheel scrolling the document.

#### Scenario: Select text with the mouse

- **WHEN** a YAML view is open
- **AND** the user drags with the mouse
- **THEN** the terminal selects text instead of sofka handling the click
