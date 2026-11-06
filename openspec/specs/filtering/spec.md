# filtering Specification

## Purpose

Define the `/` row filter of the resource table: the query grammar (text, fuzzy, regex, label, Boolean, comparison and selector terms), how terms are evaluated against rows, how errors and server-side selectors are shown, match highlighting, and how the filter persists across navigation.

## Requirements

### Requirement: Open the filter prompt

Pressing `/` in the resource table SHALL open the filter prompt with the active filter text, so the user edits the current filter instead of starting over.

#### Scenario: Slash opens the prompt

- **WHEN** the user presses `/` in a resource table
- **THEN** the filter prompt opens showing `/` followed by the current filter text
- **AND** subsequent characters are appended to the filter

### Requirement: Live local filtering while typing

Every edit in the filter prompt SHALL re-filter the table immediately and move the selection to the first row, without restarting the resource watch for local terms.

#### Scenario: Typing narrows rows live

- **WHEN** the pods table lists `alpha` and `beta`
- **AND** the user types `/` then `alp`
- **THEN** only `alpha` is listed while the prompt is still open
- **AND** the first row is selected and no new watch is started

#### Scenario: Watch updates are filtered live

- **WHEN** a filter is active
- **AND** a watched object changes so that it no longer matches
- **THEN** the row disappears from the table without user input

### Requirement: Filter prompt editing chords

The filter prompt SHALL support `ctrl-u` to clear the whole line, `ctrl-w` and `alt-Backspace` to delete the previous word, and `Backspace` or `ctrl-h` to delete one character, while keeping the prompt open.

#### Scenario: Word and line deletion

- **WHEN** the prompt contains `hello world`
- **AND** the user presses `alt-Backspace`
- **THEN** the filter is `hello `
- **AND** pressing `ctrl-w` then leaves an empty filter with the prompt still open

#### Scenario: Clear line

- **WHEN** the prompt contains `app=nginx running`
- **AND** the user presses `ctrl-u`
- **THEN** the filter is empty
- **AND** the prompt stays open

### Requirement: Esc in the prompt clears the filter

Pressing Esc in the filter prompt SHALL clear the filter, return to the table, drop any applied server-side selectors (restarting the watch only if selectors were applied), and record the empty filter in view history.

#### Scenario: Escape clears a label filter

- **WHEN** the prompt contains `label:missing` and no rows match
- **AND** the user presses Esc
- **THEN** the filter is empty and the table shows all rows again
- **AND** the mode returns to the table

### Requirement: Enter accepts a valid filter

Pressing Enter in the filter prompt with a valid filter SHALL return to the table, keep the filter active, apply any changed `-l`/`-f` selectors, and record the filter in view history.

#### Scenario: Accept local filter

- **WHEN** the prompt contains `alpha`
- **AND** the user presses Enter
- **THEN** the table is focused with the filter `alpha` still applied
- **AND** no new watch is started

### Requirement: Malformed filters block Enter

When the filter is malformed, Enter SHALL keep the prompt open and flash `filter: <error>` as a warning, and the prompt SHALL show `✗ <error>` in red after the text.

#### Scenario: Empty label pattern

- **WHEN** the user types `label:` and presses Enter
- **THEN** the prompt stays open
- **AND** a warning flash reads `filter: expected pattern after 'label:'`

### Requirement: Malformed terms do not blank the table

A malformed term SHALL be skipped while the remaining valid terms keep filtering rows, and only the first error SHALL be reported.

#### Scenario: Valid terms still apply

- **WHEN** the filter is `alpha label:`
- **THEN** rows are still filtered by `alpha`
- **AND** the filter error `expected pattern after 'label:'` is shown

### Requirement: Plain words must each match

Without structured markers the filter SHALL be split on whitespace and every word SHALL match. A word matches when its contiguous, case-insensitive text occurs in `<namespace> <name>` or in any single displayed cell.

#### Scenario: Contiguous match

- **WHEN** pods `istiod-7c9f` and `istio-cni-node` are listed
- **AND** the filter is `istiod`
- **THEN** only `istiod-7c9f` is listed

#### Scenario: Words match different places

- **WHEN** pod `alpha` is in namespace `default`
- **AND** the filter is `default alp`
- **THEN** `alpha` is listed

### Requirement: Text never matches across cells

Text, quoted, fuzzy and regex patterns SHALL be tested against `<namespace> <name>` and then against each displayed cell separately, so a pattern never matches text spanning two cells and a regex anchor such as `^` anchors to the start of a cell.

#### Scenario: Regex anchors to the cell

- **WHEN** pods `web-1` and `auth-api-0` are listed
- **AND** the filter is `/^web/`
- **THEN** only `web-1` is listed

### Requirement: Pipe alternation in a word

A `|` inside a text word SHALL list alternatives, and a row matching any alternative SHALL match. Empty alternatives SHALL be ignored, and a word made only of pipes SHALL be an error `expected text around '|'`.

#### Scenario: Either name

- **WHEN** the filter is `istiod|istio-cni-node`
- **THEN** pods named `istiod-7c9f` and `istio-cni-node` are both listed

#### Scenario: Trailing pipe tolerated

- **WHEN** the filter is `istiod|`
- **THEN** the filter has no error and matches `istiod-7c9f`

### Requirement: Quoted literal text

A double-quoted term SHALL match its contents as contiguous case-insensitive text including spaces, with Unicode case folding; an unclosed quote on the last term SHALL be accepted, and `""` SHALL be an error `expected text between the quotes`.

#### Scenario: Quoted message text

- **WHEN** events have messages `Kube started` and `other message`
- **AND** the filter is `"KUBE"`
- **THEN** only the event with `Kube started` is listed

### Requirement: Fuzzy terms with smart case

A term prefixed with `~` SHALL match its characters in order but not necessarily adjacent; it SHALL ignore case unless the needle contains an uppercase character. `~` alone SHALL be an error `expected text after '~'`.

#### Scenario: Loose match

- **WHEN** the filter is `~khc`
- **THEN** pod `kube-httpcache-0` is listed

### Requirement: Regex terms

A term wrapped in slashes `/pattern/` SHALL be a case-insensitive regular expression that can contain spaces and quotes. A term with only a leading slash SHALL be plain text. `//` SHALL be an error `expected a pattern between the slashes`, and an invalid regex SHALL be an error `bad regex '<pattern>'`.

#### Scenario: Regex with spaces

- **WHEN** pod `auth-api-0` is in namespace `default`
- **AND** the filter is `/default auth/`
- **THEN** `auth-api-0` is listed

#### Scenario: Unclosed slash is text

- **WHEN** the filter is `/healthz`
- **THEN** it is treated as the plain text `/healthz`

### Requirement: Negated terms

A term prefixed with `!` SHALL keep only rows the term does not match. `!` alone SHALL be an error `expected text after '!'`.

#### Scenario: Exclude canary

- **WHEN** pods `auth-api-0` and `auth-api-canary` are listed
- **AND** the filter is `auth !canary`
- **THEN** only `auth-api-0` is listed

### Requirement: Label terms

`label:<pattern>` SHALL test each key and each value of the object's own `metadata.labels` separately, case-insensitively, including labels not shown in the table; text, quoted, `|` and `/regex/` patterns SHALL be supported. Names, annotations and pod template labels SHALL NOT be searched.

#### Scenario: Hidden label value

- **WHEN** pod `alpha` has label `test.example.com/type=example100` and an annotation with the same value
- **AND** the filter is `label:example100`
- **THEN** only `alpha` is listed
- **AND** the plain filter `example100` lists no pods

#### Scenario: No match across key and value

- **WHEN** pod `alpha` has label `ab=cd`
- **AND** the filter is `label:abcd`
- **THEN** `alpha` is not listed

### Requirement: Negated and empty label terms

`!label:<pattern>` SHALL keep objects without any matching key or value, including objects without labels, and `label:/^$/` SHALL match objects with an empty label value.

#### Scenario: Negated label

- **WHEN** pods `alpha` (`release=Stable`), `beta` (`release=canary`) and `gamma` (no labels) exist
- **AND** the filter is `!label:canary`
- **THEN** `alpha` and `gamma` are listed and `beta` is not

### Requirement: Label term errors

An empty label pattern SHALL be an error `expected pattern after 'label:'`, and a fuzzy label pattern SHALL be an error `label patterns cannot be fuzzy; drop the '~'`.

#### Scenario: Invalid label patterns keep the editor open

- **WHEN** the user enters `label: api`, `label:""`, `label://` or `label:/[/` and presses Enter
- **THEN** the prompt stays open with an error shown
- **AND** no new watch is started

### Requirement: Structured mode markers

The filter SHALL switch from plain words to the structured grammar when it contains `&&`, `||`, `-l`, `-f`, a leading `!` or `(`, a term starting with `"`, a `label:` term, a `/regex/` term, or a `key<op>value` comparison.

#### Scenario: Comparison enables structured mode

- **WHEN** the filter is `label:cd status=Running`
- **THEN** rows must satisfy both the label term and the comparison

### Requirement: AND and OR expressions

Spaces and `&&` SHALL combine terms with AND, `||` SHALL combine with OR, and AND SHALL bind more tightly than OR. `&&` or `||` without terms on both sides SHALL be an error `expected terms on both sides of &&` (or `||`).

#### Scenario: OR of label terms with exclusion

- **WHEN** pods `alpha` (`ab=cd`), `beta` (`role=worker`) and `gamma` (`ab=canary`) exist
- **AND** the filter is `(label:cd || label:worker) && !label:canary`
- **THEN** `alpha` and `beta` are listed

### Requirement: Boolean groups and negated groups

Parentheses SHALL group terms and `!( ... )` SHALL negate a group. An empty group SHALL be an error `empty Boolean group`, an unclosed group `unclosed group`, a stray `)` `unexpected ')' in filter`, and nesting deeper than 32 levels `filter nesting exceeds 32 levels`.

#### Scenario: Negated group

- **WHEN** pods `alpha` (`ab=cd`), `beta` (`role=worker`) and `gamma` (`ab=canary`) exist
- **AND** the filter is `!(label:cd || label:worker)`
- **THEN** only `gamma` is listed

#### Scenario: Group with literal and regex

- **WHEN** pods `auth-api-0`, `auth-api-canary`, `web-1` exist
- **AND** the filter is `("auth" || /^web/) && !/canary/`
- **THEN** `auth-api-0` and `web-1` are listed

### Requirement: Comparison terms

A `key<op>value` term SHALL compare a row value using `=`, `==`, `!=`, `>`, `>=`, `<` or `<=`; keys SHALL be case-insensitive, quoted values SHALL be accepted to include spaces, and an empty value SHALL be an error `missing value in '<term>'`.

#### Scenario: Status comparison

- **WHEN** pods with STATUS `Running` and `Pending` are listed
- **AND** the filter is `status=running`
- **THEN** only the `Running` pod is listed

#### Scenario: Text equality is whole-cell

- **WHEN** nodes have ROLES `worker` and `control-plane,worker`
- **AND** the filter is `roles=worker`
- **THEN** only the node with ROLES `worker` is listed

### Requirement: Comparison keys name displayed columns

A comparison key SHALL resolve to the displayed column with that header, ignoring case; `namespace`, `ns` and `metadata.namespace` SHALL read the namespace, and `metadata.name`, `spec.nodeName` and `status.phase` SHALL read those fields even when not displayed. `status` SHALL fall back to the phase when the kind has no STATUS column.

#### Scenario: Well-known path without a column

- **WHEN** pods run on `node-3` and `node-4`
- **AND** the filter is `spec.nodeName=node-3`
- **THEN** only pods on `node-3` are listed

### Requirement: Numeric comparisons on cells

When a comparison value is numeric, the cell's leading number before a space, `/` or `(` SHALL be compared, and `restarts` with a non-numeric value SHALL be an error `bad restart count`.

#### Scenario: Restarts threshold

- **WHEN** pods with RESTARTS `2` and `7 (3m ago)` are listed
- **AND** the filter is `restarts>=5`
- **THEN** only the pod with 7 restarts is listed

### Requirement: CPU and memory comparisons

`cpu` SHALL compare live usage in millicores (suffixes `n`, `u`, `m`; a bare number is cores) and `mem` or `memory` SHALL compare live usage in bytes with quantity suffixes, for pods and nodes only. Invalid values SHALL be errors `bad cpu quantity '<v>'` or `bad memory quantity`.

#### Scenario: CPU threshold updates with metrics

- **WHEN** the filter is `("auth" && cpu>500m)` and no metrics have arrived
- **AND** a metrics sample reports 600m for `auth-api-0`
- **THEN** `auth-api-0` becomes listed without further input

### Requirement: Age comparisons

`age` SHALL compare the object's creation age using `s`, `m`, `h`, `d` and `w` units, combined units such as `1d2h` included; a bare integer SHALL mean seconds, and an invalid duration SHALL be an error `bad duration`. Age filters SHALL re-evaluate every second without a watch event.

#### Scenario: Young pods

- **WHEN** pods aged 30 minutes and 5 hours are listed
- **AND** the filter is `age<2h`
- **THEN** only the 30-minute-old pod is listed

### Requirement: Missing values are unknown

A comparison whose value is missing (no metrics sample, no such column, missing field) SHALL NOT match, including under `!=` and negation; unknown SHALL NOT be treated as zero.

#### Scenario: Pod without metrics

- **WHEN** pod `unknown` has no metrics sample and pod `idle` has 0m
- **AND** the filter is `cpu!=1`
- **THEN** `idle` is listed and `unknown` is not

### Requirement: Metric and formatted columns compare source values

Comparisons on metric columns and on `format = "cpu"`/`"memory"` view columns SHALL compare the underlying value, not the rounded display; CPU numbers without suffix SHALL mean cores and percentage columns SHALL accept an optional `%` suffix.

#### Scenario: Allocatable CPU column

- **WHEN** a nodes view has column `CPU/A` with `format = "cpu"` showing `4000m`
- **AND** the filter is `cpu/a>=4`
- **THEN** the node is listed

### Requirement: Server Table column comparisons

Comparisons on server Table columns SHALL compare numerically only for integer and number columns; other server columns SHALL compare as text without reading a leading number.

#### Scenario: Numeric server column

- **WHEN** a server Table column `SESSIONS` is of type integer
- **AND** the filter is `sessions>5`
- **THEN** only rows whose SESSIONS value exceeds 5 are listed

### Requirement: Helm release names

In Helm release views text terms SHALL match the release name shown in the table rather than the name of the storage Secret.

#### Scenario: Release name

- **WHEN** the helm view lists release `myapp` stored in Secret `sh.helm.release.v1.myapp.v1`
- **AND** the filter is `myapp`
- **THEN** the release is listed
- **AND** the filter `sh.helm.release` lists nothing

### Requirement: Label and field selectors

`-l <selector>` and `-f <selector>` SHALL define Kubernetes label and field selectors; `in (...)`/`notin (...)` sets and quoted selectors SHALL be accepted, repeated flags SHALL be joined with commas, and selectors SHALL be applied only when Enter is pressed.

#### Scenario: Selector sent on Enter

- **WHEN** the user types `-l env -f spec.nodeName=node-3` and presses Enter
- **THEN** a new watch request carries `labelSelector=env` and `fieldSelector=spec.nodeName=node-3`
- **AND** a flash reads `server-side filter: -l env -f spec.nodeName=node-3`

### Requirement: Selector syntax errors

A selector flag without a value SHALL be an error `expected selector after -l` (or `-f`), a set operator without parentheses SHALL be `expected selector set in parentheses`, a `-f` selector without `=` SHALL be `invalid selector after -f`, and an unclosed quote or set SHALL be `unclosed quote or selector set`.

#### Scenario: Field selector without operator

- **WHEN** the filter is `-f spec.nodeName`
- **AND** the user presses Enter
- **THEN** the prompt stays open with error `invalid selector after -f`

### Requirement: Selectors stay outside Boolean groups

Selectors SHALL NOT appear inside OR branches or parentheses; inside an OR branch the error SHALL be `place selectors outside OR groups: -l app=api (a || b)` and inside a group `selectors must be outside Boolean groups`.

#### Scenario: Selector inside OR rejected

- **WHEN** the filter is `-l app=api || worker`
- **THEN** the filter shows the error `place selectors outside OR groups: -l app=api (a || b)`

#### Scenario: Selector with grouped local terms

- **WHEN** the filter is `-l app=api (status=Pending || restarts>=5)`
- **THEN** the filter is valid

### Requirement: Local edits keep applied selectors

Editing only local terms SHALL NOT restart the watch; applied selectors SHALL stay in effect, and clearing all selectors then pressing Enter SHALL restart the watch unscoped and flash `server-side filter cleared`.

#### Scenario: Local edit after selector

- **WHEN** `-l env` is applied
- **AND** the user changes only local terms after it
- **THEN** no new API request is made and rows are filtered locally

### Requirement: Filter location indicator

The table title SHALL show ` /<filter>` (red on error) followed by ` ·local` for local terms only, ` ·server` for selectors only, ` ·server+local` for both, or ` ·pending ⏎` while typed selectors differ from those applied.

#### Scenario: Server plus local

- **WHEN** the filter `-l env api` has been applied
- **THEN** the title ends with ` ·server+local`

### Requirement: Pending selector hint in the prompt

While selectors in the prompt differ from those applied and the filter has no error, the prompt SHALL show a yellow hint `<Enter key> apply server-side` after the text.

#### Scenario: Pending hint

- **WHEN** the user types `-l app=api` without pressing Enter
- **THEN** the prompt shows the apply server-side hint
- **AND** the title shows ` ·pending ⏎`

### Requirement: Selector errors from the API

Selectors SHALL be sent to Kubernetes unchanged; an unsupported selector SHALL surface as a watch error and SHALL NOT be silently dropped or replaced by an unfiltered list.

#### Scenario: Unsupported field selector

- **WHEN** the user applies `-f spec.unknown=x` on a resource that rejects it
- **THEN** a watch error is shown
- **AND** the table is not refilled with unfiltered objects

### Requirement: Match highlighting in NAME

The NAME cell SHALL highlight in bold the characters matched by the first positive text, quoted, regex or fuzzy term; contiguous patterns highlight one span, fuzzy patterns highlight scattered characters, and label terms SHALL NOT drive highlighting.

#### Scenario: Label term does not highlight

- **WHEN** the filter is `label:cd alpha`
- **THEN** the five characters `alpha` in the NAME cell are highlighted
- **AND** the filter `label:cd` alone highlights nothing

### Requirement: Esc in the table clears the filter

When the table has no marked rows, Esc in the table SHALL clear an active filter (removing applied selectors and widening the watch) before any other Esc behaviour.

#### Scenario: Escape clears the text filter

- **WHEN** the table shows rows filtered by `api`
- **AND** the user presses Esc in the table
- **THEN** the filter is empty and all rows are listed

### Requirement: Filter survives refresh and namespace changes

A refresh and a namespace change (via `0`, the namespace picker or the palette) SHALL keep the active filter.

#### Scenario: Namespace switch keeps filter

- **WHEN** the filter is `label:example100`
- **AND** the user switches to namespace `qa` and then to all namespaces
- **THEN** the filter is still `label:example100`

### Requirement: Root view switch clears the filter

Switching the root resource kind through the palette SHALL clear the filter.

#### Scenario: New kind starts unfiltered

- **WHEN** the deployments table has filter `web`
- **AND** the user runs `:services`
- **THEN** the services table has no filter

### Requirement: Drill-down keeps only selectors

Drilling into a child view SHALL clear local terms and keep only applied selectors, re-quoted as `-l '<labels>' -f '<fields>'`; the child watch SHALL combine them with the drill's own selectors. Returning with Esc SHALL restore the parent filter.

#### Scenario: Drill keeps label selector

- **WHEN** deployments are filtered by `-l env=prod label:example100`
- **AND** the user presses Enter on deployment `web` selecting `app=web`
- **THEN** the pods view filter is `-l 'env=prod'` and the watch label selector is `app=web,env=prod`
- **AND** pressing Esc twice returns to deployments with the original filter

### Requirement: History restores root filters

Back and forward view history (`[` and `]`) SHALL restore the filter saved with each root view entry.

#### Scenario: Back restores filter

- **WHEN** deployments were filtered by `-l env=prod label:example100` before running `:services`
- **AND** the user presses `[`
- **THEN** the deployments table shows that filter again
- **AND** pressing `]` returns to services with an empty filter

### Requirement: Palette filter suffix

A palette resource command SHALL accept a trailing `/<filter>` after scope options; everything after the slash SHALL be parsed with the row filter grammar and its selectors SHALL scope the first watch.

#### Scenario: Palette with filter

- **WHEN** the user runs `:pods -n prod /-l app=api status=Running`
- **THEN** the pods view in `prod` opens with that filter
- **AND** the first watch request carries `labelSelector=app=api`
