# selection-clipboard Specification

## Purpose

Defines how users mark table rows for bulk actions (space, ctrl-space and
Shift-arrow ranges), which rows a bulk action targets, when marks are cleared,
and how sofka copies names and column values to the system clipboard with the
`c` and `Y` keys.

## Requirements

### Requirement: Toggle a row mark

`space` in the resource table SHALL toggle the mark on the selected row, identified by its namespace and name, and move the cursor to the next row.

#### Scenario: Mark and advance

- **WHEN** row 2 is selected and unmarked
- **AND** the user presses `space`
- **THEN** row 2 is marked
- **AND** the cursor is on row 3

#### Scenario: Unmark

- **WHEN** the selected row is marked
- **AND** the user presses `space`
- **THEN** the row is no longer marked

### Requirement: Marked row display

Marked rows SHALL render bold in the skin's mark colour, and the table title MUST show " ✓N" with the number of marked rows.

#### Scenario: Title count

- **WHEN** two rows are marked
- **THEN** the table title contains "✓2"

### Requirement: Mark a range with ctrl-space

`ctrl-space` (also received as `ctrl-@`) SHALL mark every row between the last row marked with `space` and the cursor, inclusive, in the order currently displayed. When that anchor row is no longer marked or not visible, it MUST mark only the cursor row; with no selected row it MUST mark nothing.

#### Scenario: Range from anchor

- **WHEN** the user marks row 2 with `space`
- **AND** moves the cursor to row 5 and presses `ctrl-space`
- **THEN** rows 2, 3, 4 and 5 are marked

#### Scenario: No anchor

- **WHEN** no row was marked with `space`
- **AND** the user presses `ctrl-space` on row 4
- **THEN** only row 4 is marked

#### Scenario: Anchor hidden by filter

- **WHEN** the anchor row is hidden by the current filter
- **AND** the user presses `ctrl-space`
- **THEN** only the cursor row is marked

### Requirement: Shift-arrow range selection

`Shift+Down` and `Shift+Up` SHALL extend a range of marked rows from the row where the range started to the new cursor row, clamped at the first and last rows. Moving back toward the start MUST unmark rows that leave the range, the range MUST be able to cross the start row, and marks made before the range started MUST be kept.

#### Scenario: Extend down

- **WHEN** the cursor is on row 3 with no range active
- **AND** the user presses `Shift+Down` twice
- **THEN** rows 3, 4 and 5 are marked
- **AND** the cursor is on row 5

#### Scenario: Contract

- **WHEN** a range covers rows 3 to 5 with the cursor on row 5
- **AND** the user presses `Shift+Up`
- **THEN** rows 3 and 4 are marked and row 5 is not

#### Scenario: Earlier marks kept

- **WHEN** row 10 was marked with `space`
- **AND** the user makes a Shift-arrow range over rows 2 to 4 and contracts it
- **THEN** row 10 stays marked

### Requirement: Range end and restart

Any key other than the range keys SHALL end the current Shift-arrow range, and so MUST a mouse click on a row; the next `Shift+arrow` starts a new range at the cursor. A change to row order, membership or identity (filter, sort, add, delete or replace) MUST also start a new range, while a status-only update of the same rows MUST NOT.

#### Scenario: Other key ends the range

- **WHEN** a range covers rows 3 to 5
- **AND** the user presses `j` and then `Shift+Down`
- **THEN** a new range starts at row 6
- **AND** rows 3 to 5 stay marked

#### Scenario: Status update keeps the range

- **WHEN** a range covers rows 3 to 5
- **AND** a watch update changes only the status of row 4
- **AND** the user presses `Shift+Down`
- **THEN** the same range extends to row 6

### Requirement: Range on an empty table

Shift-arrow range keys SHALL do nothing when the table has no rows.

#### Scenario: Empty table

- **WHEN** the table has no rows
- **AND** the user presses `Shift+Down`
- **THEN** nothing is marked

### Requirement: Rebindable range keys

The Shift-arrow range keys SHALL be configurable as `range_down` and `range_up` under `[keys.table]`.

#### Scenario: Custom range key

- **WHEN** `[keys.table] range_down = ["ctrl-j"]` is configured
- **AND** the user presses `ctrl-j`
- **THEN** the range extends down by one row

### Requirement: Marks cleared by esc

`esc` in the resource table SHALL clear all marks before doing anything else.

#### Scenario: Esc with marks and filter

- **WHEN** three rows are marked and a filter is active
- **AND** the user presses `esc`
- **THEN** no rows are marked
- **AND** the filter is still active

### Requirement: Marks cleared on view change

Every restart of the table's resource watch, such as switching the resource kind, namespace or context, drilling down or going back, SHALL clear all marks and end any Shift-arrow range.

#### Scenario: Switch kind

- **WHEN** two pods are marked
- **AND** the user runs `:svc`
- **THEN** no rows are marked

### Requirement: Marks kept through filtering

Changing the filter SHALL keep existing marks, including marks on rows the filter hides.

#### Scenario: Filter and unfilter

- **WHEN** pod `web-1` is marked
- **AND** the user changes the filter to `db` and then edits it back to empty
- **THEN** `web-1` is still marked

### Requirement: Bulk action targets

Actions that accept several objects SHALL target the marked rows that are visible in the current filtered table; when no rows are marked they MUST target the selected row. Marked rows that are hidden or no longer exist MUST be skipped, without falling back to the selected row.

#### Scenario: Marked rows targeted

- **WHEN** pods `a` and `b` are marked and pod `c` is selected
- **AND** the user presses `ctrl-d`
- **THEN** the confirmation reads "Delete 2 pods"

#### Scenario: Hidden marks skipped

- **WHEN** pod `a` is marked but hidden by the filter and pod `c` is selected
- **AND** the user starts a bulk action
- **THEN** neither pod `a` nor pod `c` is targeted

### Requirement: Bulk labels

Confirmations for several targets SHALL name the count and kind, such as "Delete 2 pods", and the `t` action menu MUST read "N marked <kind>" when more than one row is marked, or "current selection" otherwise.

#### Scenario: Action menu with marks

- **WHEN** three deployments are marked
- **AND** the user opens the action menu with `t`
- **THEN** the menu header names "3 marked deployments"

### Requirement: Marks cleared after bulk actions

Running a confirmed action on the targets, such as delete, drain, restart, scale, Helm uninstall, Flux or Argo CD actions or a CronJob trigger, SHALL clear all marks.

#### Scenario: Delete marked pods

- **WHEN** two pods are marked
- **AND** the user confirms deleting them
- **THEN** no rows are marked

### Requirement: Copy the row name

`c` in the resource table SHALL copy the selected row's name to the clipboard and flash "copied: <name>"; with no selected row it MUST do nothing.

#### Scenario: Copy a pod name

- **WHEN** pod `web-1` is selected
- **AND** the user presses `c`
- **THEN** the clipboard contains `web-1`
- **AND** the status line shows "copied: web-1"

### Requirement: Copy picker contents

`Y` in the resource table SHALL open a copy picker listing each displayed column of the selected row as a header and full value, including NAMESPACE when shown and skipping empty values. The values MUST be captured when the picker opens so watch updates do not change them. With no selected row it MUST flash "no row selected" and stay in the table.

#### Scenario: Open the picker

- **WHEN** a pod row with NAME, READY and IP columns is selected
- **AND** the user presses `Y`
- **THEN** the picker lists NAME, READY and IP with their values

#### Scenario: Empty table

- **WHEN** no row is selected
- **AND** the user presses `Y`
- **THEN** the status line shows "no row selected"

### Requirement: Copy picker filtering and navigation

Typing in the copy picker SHALL fuzzy-match against "<header> <value>" and put the cursor on the best match. `Up`/`ctrl-p` and `Down`/`ctrl-n` MUST move, and `esc` MUST clear a non-empty filter first and close the picker otherwise.

#### Scenario: Find by value

- **WHEN** the picker lists IP `10.0.0.7`
- **AND** the user types `10.0`
- **THEN** the IP entry is highlighted

### Requirement: Copy picker accept

`enter` in the copy picker SHALL copy the highlighted value, close the picker and flash "copied <HEADER>: <value>", showing at most 60 characters of the value followed by `…` when longer.

#### Scenario: Long value truncated

- **WHEN** the highlighted LABELS value is 100 characters long
- **AND** the user presses `enter`
- **THEN** the clipboard contains the full value
- **AND** the status line shows the first 60 characters followed by `…`

### Requirement: Clipboard copy runs in the background

Copying SHALL run in the background with the status "copying to clipboard…" and then show the success message, or the error "no clipboard target found (pbcopy/xclip/wl-copy/OSC 52)" when every method fails. A result that arrives after a context switch or other view reset MUST be ignored.

#### Scenario: No clipboard target

- **WHEN** no clipboard tool succeeds and OSC 52 cannot be written
- **AND** the user presses `c`
- **THEN** the status line shows the error "no clipboard target found (pbcopy/xclip/wl-copy/OSC 52)"

### Requirement: Clipboard tool order

sofka SHALL try clipboard targets in this order: `clip.exe` (only when `WSL_INTEROP` or `WSL_DISTRO_NAME` is set, sent as UTF-16LE with a byte-order mark), `pbcopy`, `wl-copy`, `xclip -selection clipboard`, `xsel --clipboard --input`, then OSC 52. A tool MUST count as successful only when writing the text and its exit status both succeed.

#### Scenario: WSL

- **WHEN** `WSL_DISTRO_NAME` is set and `clip.exe` succeeds
- **AND** the user copies a name
- **THEN** the text is sent to `clip.exe` and no other tool is tried

#### Scenario: Failing tool falls through

- **WHEN** `wl-copy` exits with an error and `xclip` succeeds
- **AND** the user copies a name
- **THEN** the copy succeeds through `xclip`

### Requirement: OSC 52 fallback

When no clipboard tool succeeds, sofka SHALL write the OSC 52 sequence `ESC ] 52 ; c ; <base64 text> BEL` to `/dev/tty`, or to standard output when `/dev/tty` cannot be opened.

#### Scenario: Remote terminal

- **WHEN** sofka runs over SSH without clipboard tools
- **AND** the user copies `web-1`
- **THEN** the terminal receives an OSC 52 sequence with `web-1` base64-encoded

### Requirement: Shared clipboard mechanism

Copy actions in other views (logs, documents) SHALL use the same clipboard targets, order and fallback as the table copy keys.

#### Scenario: Copy from logs

- **WHEN** the user presses `c` in the logs view
- **THEN** the log text is copied through the same clipboard targets
