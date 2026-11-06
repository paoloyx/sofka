# resource-table Specification

## Purpose

Define the main resource table: how watched objects become rows, which columns each kind shows (curated, custom `[views]`, CRD printer and server Table columns), status cells and row colours, wide, compact and horizontally scrolled layouts, sorting, and the pod faults toggle.

## Requirements

### Requirement: Rows follow the live watch

The table SHALL list the objects of the active resource kind from a live watch, adding, updating and removing rows as watch events arrive without user input.

#### Scenario: New object appears

- **WHEN** the pods table is open
- **AND** the watch delivers a new pod `api`
- **THEN** a row for `api` appears without a refresh

### Requirement: Table title

The table border title SHALL show `<plural>` and `[<count>]` where count is the number of rows after filtering, followed when applicable by ` [faults]`, ` ←`, ` →`, the mark count ` ✓<n>` and the active filter.

#### Scenario: Title with count

- **WHEN** the pods table shows 3 rows
- **THEN** the title begins with ` pods [3]`

### Requirement: Namespace column

For namespaced kinds viewed across all namespaces or under a namespace pattern, the table SHALL prepend a NAMESPACE column; cluster-scoped kinds and single-namespace views SHALL NOT show it.

#### Scenario: All namespaces

- **WHEN** the pods view switches to all namespaces with `0`
- **THEN** the first column is NAMESPACE

### Requirement: Curated pod columns

Core pods SHALL show NAME, READY, STATUS, RESTARTS and AGE, with IP and NODE as wide-only columns; missing IP or node SHALL show `<none>`.

#### Scenario: Pod headers

- **WHEN** the user opens `:pods`
- **THEN** the headers include NAME, READY, STATUS, RESTARTS and AGE
- **AND** IP and NODE are hidden until wide mode is on

### Requirement: Curated workload columns

Deployments SHALL show NAME, READY, STATUS, UP-TO-DATE, AVAILABLE, AGE; replicasets NAME, DESIRED, CURRENT, READY, STATUS, AGE; statefulsets NAME, READY, STATUS, AGE; daemonsets NAME, DESIRED, CURRENT, READY, AVAILABLE, STATUS, AGE; jobs NAME, STATUS, COMPLETIONS, DURATION, AGE; cronjobs NAME, SCHEDULE, SUSPEND, ACTIVE, LAST-SCHEDULE, AGE.

#### Scenario: Deployment headers

- **WHEN** the user opens `:deployments`
- **THEN** the headers are NAME, READY, STATUS, UP-TO-DATE, AVAILABLE and AGE

### Requirement: Curated core resource columns

Services SHALL show NAME, TYPE, CLUSTER-IP, EXTERNAL-IP, PORTS, AGE; nodes NAME, STATUS, ROLES, TAINTS, VERSION, AGE with wide INTERNAL-IP, EXTERNAL-IP and LABELS; namespaces NAME, STATUS, AGE; configmaps NAME, DATA, AGE; secrets NAME, TYPE, DATA, AGE; endpoints NAME, ENDPOINTS, AGE.

#### Scenario: Node wide columns

- **WHEN** the nodes table is open and the user presses `w`
- **THEN** INTERNAL-IP, EXTERNAL-IP and LABELS columns appear

### Requirement: Curated columns for other built-in kinds

Events SHALL show NAME, LAST-SEEN, TYPE, REASON, OBJECT, MESSAGE, COUNT, AGE; HPAs NAME, REFERENCE, TARGETS, MINPODS, MAXPODS, REPLICAS, AGE; PVCs NAME, STATUS, VOLUME, CAPACITY, AGE; PVs NAME, CAPACITY, STATUS, CLAIM, AGE; ingresses NAME, CLASS, HOSTS, ADDRESS, AGE; HTTPRoutes NAME, HOSTNAMES, ROUTES, AGE; CRDs NAME, GROUP, KIND, VERSIONS, SCOPE, AGE.

#### Scenario: Event headers

- **WHEN** the user opens `:events`
- **THEN** LAST-SEEN is the second column and AGE the last

### Requirement: Curated columns match API group

Curated columns SHALL apply only when both the API group and the resource plural match a known kind, so a custom resource sharing a plural with a core kind SHALL NOT receive the core columns.

#### Scenario: Knative services

- **WHEN** the user opens `services.serving.knative.dev`
- **THEN** the table does not show the core Service network columns

### Requirement: NAME and AGE fallback

A kind without curated columns, without a configured view and without usable CRD printer or server Table columns SHALL show only NAME and AGE.

#### Scenario: Unknown kind

- **WHEN** the user opens a resource whose API does not support Tables and has no CRD columns
- **THEN** the headers are NAME and AGE

### Requirement: Pod STATUS cell

The pod STATUS cell SHALL show `Terminating` for a pod being deleted; otherwise init progress or failure while initializing, then a container waiting reason, then a container termination reason or signal/exit code, then `SchedulingGated` for gated pods, then the pod reason (such as `Evicted`) or phase, or `Unknown`.

#### Scenario: Crash loop reason

- **WHEN** a pod container is waiting with reason `CrashLoopBackOff`
- **THEN** the STATUS cell shows `CrashLoopBackOff`

### Requirement: Workload STATUS cell

Deployment, statefulset, replicaset and daemonset STATUS SHALL be `Terminating` when deleting, `Progressing` while the spec generation is unobserved or replicas are rolling, `ScaledDown` at zero desired and current, `Stalled` on ReplicaFailure or ProgressDeadlineExceeded, `Unavailable` with no ready replicas or Available=False, `Ready` when all are ready, else `Degraded`.

#### Scenario: Scaled to zero

- **WHEN** a deployment has `spec.replicas = 0` and no current replicas
- **THEN** its STATUS shows `ScaledDown`

### Requirement: Node STATUS cell

Node STATUS SHALL be `Ready` when the Ready condition is True, `NotReady` when it is False or Unknown, `Unknown` without a Ready condition, and SHALL append `,SchedulingDisabled` when the node is cordoned.

#### Scenario: Cordoned ready node

- **WHEN** a node is Ready and has `spec.unschedulable = true`
- **THEN** its STATUS shows `Ready,SchedulingDisabled`

### Requirement: Node ROLES cell

Node ROLES SHALL combine role names from `node-role.kubernetes.io/<role>` label suffixes, the `kubernetes.io/role` value and any `[node_roles]` `label_prefixes`/`label_keys` sources, sorted, de-duplicated and comma-joined, or `<none>` when no role is found.

#### Scenario: Combined roles

- **WHEN** a node has `node-role.example.com/control-plane` and `node-role.kubernetes.io/worker` labels
- **AND** `[node_roles] label_prefixes = ["node-role.example.com/"]` is configured
- **THEN** its ROLES cell shows `control-plane,worker`

### Requirement: Event LAST-SEEN

The event LAST-SEEN cell SHALL show the elapsed time since the most recent reported occurrence for core and `events.k8s.io` Events, while AGE SHALL keep showing creation age.

#### Scenario: Recent occurrence

- **WHEN** an event was created 2 hours ago and last occurred 5 minutes ago
- **THEN** LAST-SEEN shows `5m` and AGE shows `2h`

### Requirement: Time cells advance

AGE, event LAST-SEEN, Helm UPDATED, the DURATION of unfinished jobs and cronjob LAST-SCHEDULE SHALL advance with wall time without a watch event.

#### Scenario: Age ticks

- **WHEN** a pod row shows AGE `59s`
- **AND** no watch event arrives for that pod
- **THEN** after a few seconds the AGE cell shows a later elapsed time

### Requirement: Row colour from status

Each row SHALL be coloured from its status cell: failures red, pending and `Init:` states peach, `Completed`, `Succeeded`, `ScaledDown` and similar dimmed, `Terminating` mauve, and other states blue; a `,SchedulingDisabled` suffix SHALL keep the readiness colour, and the status cell SHALL use a status badge colour.

#### Scenario: Failure row

- **WHEN** a pod shows STATUS `CrashLoopBackOff`
- **THEN** the whole row is red

#### Scenario: Cordoned not-ready node

- **WHEN** a node shows `NotReady,SchedulingDisabled`
- **THEN** the row is red

### Requirement: Running but not ready is transitional

A row whose status is `Running` but whose READY count is not complete, or a pod with a readiness gate that is not True, SHALL be coloured as `PodInitializing` (transitional) instead of healthy.

#### Scenario: Half-ready pod

- **WHEN** a pod shows STATUS `Running` and READY `1/2`
- **THEN** the row uses the transitional colour, not blue

### Requirement: AGE column is dimmed

The AGE column SHALL be rendered dimmed relative to other cells and capped at 7 characters wide.

#### Scenario: Dimmed age

- **WHEN** the pods table is drawn
- **THEN** AGE cells use the dim style

### Requirement: Stable STATUS width

STATUS columns SHALL use a fixed width of 26 characters, or 27 for nodes, so status changes do not move other columns; a configured column `width` SHALL take priority.

#### Scenario: Node status width

- **WHEN** the nodes table is drawn
- **THEN** the STATUS column is 27 characters wide

### Requirement: Column widths use all filtered rows

Column widths SHALL be computed from every filtered row, not only visible ones, and elapsed-time cells SHALL reserve at least 7 characters, so vertical scrolling and clock ticks do not move columns.

#### Scenario: Scrolling keeps widths

- **WHEN** a long row name exists far below the visible page
- **THEN** the NAME column is wide enough for it before scrolling down

### Requirement: Custom view lookup

A `[views."<key>"]` entry SHALL configure a kind's table; keys SHALL be tried most specific first: `apiVersion/plural`, `group/plural`, plural, then lowercase kind, and the first match SHALL be used without inheriting from other keys.

#### Scenario: Most specific wins

- **WHEN** both `[views.pods]` and `[views."v1/pods"]` define columns
- **THEN** the pods table uses the `v1/pods` columns only

### Requirement: Namespace-specific views

A view key with `@<namespace>` SHALL apply only when a namespaced kind is viewed in that single namespace, and all `@namespace` keys SHALL be tried before unqualified keys; all-namespaces mode and cluster-scoped kinds SHALL use only unqualified keys.

#### Scenario: Namespace layout

- **WHEN** `[views."v1/pods"]` adds OWNERKIND and `[views."pods@matlab"]` adds TENANT
- **AND** the user views pods in namespace `matlab`
- **THEN** TENANT is shown and OWNERKIND is not

### Requirement: Invalid namespace suffix

A view key whose `@` suffix is not a valid namespace name (1 to 63 lowercase letters, digits or hyphens, alphanumeric at both ends) SHALL be ignored with the warning `views."<key>": invalid namespace suffix; view ignored`.

#### Scenario: Uppercase namespace

- **WHEN** the config defines `[views."pods@Prod"]`
- **THEN** a config warning reports an invalid namespace suffix and the view is ignored

### Requirement: Overlay view columns

By default view columns SHALL overlay the curated columns: a column with a matching header SHALL replace it in place and new columns SHALL be inserted before a trailing AGE column.

#### Scenario: New column before AGE

- **WHEN** a pods view adds column `OWNERKIND`
- **THEN** OWNERKIND appears just before AGE

### Requirement: Replace view columns

With `replace = true` and at least one usable column, only the declared columns SHALL be shown in declaration order; `replace = true` with no valid columns SHALL overlay instead with the warning `views."<key>": replace = true with no valid columns; overlaying instead`.

#### Scenario: Replaced layout

- **WHEN** a pods view sets `replace = true` with builtin NAME, STATUS and AGE columns
- **THEN** the pods table shows only NAME, STATUS and AGE (plus NAMESPACE across namespaces)

### Requirement: Path column types

A `path` column SHALL accept `type` `text` (default), `status`, `number`, `quantity`, `time` or `condition`; typed columns SHALL sort by value, `time` SHALL render elapsed (`3d4h`) or future (`in 30d`) time, and an unknown type SHALL fall back to text with a warning.

#### Scenario: Time column

- **WHEN** a certificates view has `EXPIRES` with `path = "/status/notAfter"` and `type = "time"`
- **THEN** a future expiry renders as `in <duration>`

### Requirement: Condition columns

A `condition` column's `path` SHALL be a condition type name looked up by name in `status.conditions`, rendering its status; a JSON Pointer, list or empty name SHALL skip the column with a warning.

#### Scenario: Ready condition

- **WHEN** a column `READY` has `path = "Ready"` and `type = "condition"`
- **THEN** the cell shows the Ready condition status such as `True`

### Requirement: Status columns colour the row

The first `status` or `condition` view column SHALL colour its badge and the row, matching known words such as `Healthy`, `Progressing`, `Degraded` and `Failed` in any letter case.

#### Scenario: Lowercase health

- **WHEN** a custom resource status column shows `unhealthy`
- **THEN** the row is coloured as a failure

### Requirement: Fallback paths

A `path` list SHALL try each JSON Pointer in order and use the first value that exists and is not null; empty strings, `0`, `false` and empty collections SHALL stop the search, and `<none>` SHALL be shown when all are missing.

#### Scenario: Second path used

- **WHEN** a NODEPOOL column lists a karpenter label then an EKS nodegroup label
- **AND** a node has only the EKS label
- **THEN** NODEPOOL shows the EKS nodegroup value

### Requirement: Image tag format

`format = "image-tag"` on a text path column SHALL show the image tag, `latest` when untagged, and `-` for digest-only references, ignoring registry ports.

#### Scenario: Registry with port

- **WHEN** the image is `registry:5000/app:1.2.3`
- **THEN** the cell shows `1.2.3`

### Requirement: Quantity formats

`format = "cpu"` or `"memory"` on a `quantity` path column SHALL show whole millicores, or whole Mi below 1 Gi and Gi with one decimal otherwise; invalid values SHALL keep their original text and sorting SHALL use the unrounded value.

#### Scenario: Memory display

- **WHEN** a `memory` formatted column reads `16374956Ki`
- **THEN** the cell shows `15.6Gi`

### Requirement: Builtin and metric view sources

Each view column SHALL set exactly one of `path`, `builtin` or `metric`; `builtin` SHALL reuse a curated column of the kind under the configured `name`, and `type` SHALL apply only to path columns.

#### Scenario: Two sources rejected

- **WHEN** a column sets both `path` and `builtin`
- **THEN** it is skipped with the warning `set exactly one of path, metric, or builtin; column skipped`

### Requirement: View column validation warnings

Invalid view columns SHALL be skipped with an in-app configuration warning and never stop the TUI, including empty names, duplicate headers, invalid JSON Pointers, unknown formats, unknown metrics and sources not available for the resource.

#### Scenario: Unavailable builtin

- **WHEN** a pods view declares `builtin = "TAINTS"`
- **THEN** the warning `view /pods: column TAINTS is not available for this resource; column skipped` is recorded

### Requirement: Column width and alignment options

View columns SHALL accept `width` for a fixed width and `align` of `left`, `center` or `right`; an unknown alignment SHALL fall back to left with a warning.

#### Scenario: Right-aligned column

- **WHEN** a column sets `align = "right"`
- **THEN** its cells are right-aligned

### Requirement: CRD printer columns

A custom resource without curated columns or an explicit view with columns SHALL use its CRD `additionalPrinterColumns`, with `priority > 0` columns wide-only and Ready-condition JSONPath filters turned into condition columns; unrepresentable JSONPath columns SHALL be skipped.

#### Scenario: Printer columns appear

- **WHEN** the user opens a custom resource whose CRD declares a `READY` printer column on `.status.conditions[?(@.type=="Ready")].status`
- **THEN** the table shows a READY condition column

### Requirement: Server Table columns

When a grouped resource has no curated columns, no view columns and no usable CRD printer columns, and the namespace is not a pattern, sofka SHALL request columns from the Kubernetes Table API and show them in server order with uppercase headers; an API without Table support SHALL keep NAME and AGE.

#### Scenario: Aggregated API

- **WHEN** the user opens `caliconodestatuses.projectcalico.org`
- **THEN** the table shows the server-provided columns

### Requirement: Server Table column layout

Server columns with `priority > 0` SHALL be wide-only, NAME SHALL be added when the server omits it, NAME and AGE SHALL use object metadata, integer and number columns SHALL be right-aligned and sort numerically, and date columns SHALL sort by timestamp.

#### Scenario: NAME added

- **WHEN** the server Table has no Name column
- **THEN** NAME is the first column

### Requirement: Server Table refresh

Server Table cells SHALL follow Table watches capped at 30 seconds, then re-list; without watch support sofka SHALL poll, starting at most one list cycle every 5 seconds with pages of up to 500 rows, scoped by the active namespace and selectors.

#### Scenario: Paged list

- **WHEN** the Table API returns more than 500 rows
- **THEN** sofka requests further pages with `limit=500`

### Requirement: Server Table cell identity

Server cells SHALL be shown only when their UID and resourceVersion match the watched object, and SHALL show `<none>` until a matching Table update arrives; a failed Table request SHALL show an error and clear server cells while keeping the rows.

#### Scenario: Object changed

- **WHEN** a watched object receives a newer resourceVersion before the Table update
- **THEN** its server cells show `<none>`

### Requirement: Wide mode

Pressing `w` SHALL toggle wide-only columns for the session, flash `wide columns: on` or `wide columns: off`, and re-apply a remembered or configured sort whose column becomes available.

#### Scenario: Toggle wide

- **WHEN** the user presses `w` in the pods table
- **THEN** IP and NODE appear and the flash reads `wide columns: on`

### Requirement: Compact mode

`ctrl-e` SHALL toggle compact mode, collapsing the header and footer into one info line, and `compact_mode = true` SHALL start in compact mode; the choice SHALL survive reloads and context switches.

#### Scenario: Compact toggle

- **WHEN** the user presses `ctrl-e` in the table
- **THEN** the multi-line header and footer are replaced by one info line

### Requirement: Horizontal scrolling

Left and Right SHALL scroll the table columns by five text positions, clamped to the available overflow, while NAME and the columns before it stay fixed; when all columns fit Left and Right SHALL do nothing, and a view change SHALL reset the offset.

#### Scenario: Scroll right

- **WHEN** the table is wider than the terminal
- **AND** the user presses Right
- **THEN** the non-frozen columns shift by five positions and NAME stays in place

### Requirement: Horizontal scroll indicators

The title SHALL show ` ←` when the table is scrolled right and ` →` when more columns lie beyond the right edge.

#### Scenario: Both arrows

- **WHEN** the table is scrolled partly to the right with more columns remaining
- **THEN** the title shows both ` ←` and ` →`

### Requirement: Scrollbars

Long tables SHALL show thin scrollbars on their borders during keyboard or mouse scrolling, hidden 700 ms after scrolling stops and whenever the content fits.

#### Scenario: Scrollbar hides

- **WHEN** the user scrolls a long table and stops
- **THEN** the scrollbar disappears after 700 ms

### Requirement: Default row order

Without a sort column rows SHALL be ordered by namespace then name in natural order, where digit runs compare numerically.

#### Scenario: Natural order

- **WHEN** pods `web-10` and `web-2` exist
- **THEN** `web-2` is listed before `web-10`

### Requirement: Sort picker

`S` SHALL open a sort picker listing `default (ns/name)` then the displayed column headers, with the cursor on the active sort; typing SHALL fuzzy-filter headers, the first Esc SHALL clear the typed filter and the next close the picker, and `no columns to sort by` SHALL be flashed when there are no headers.

#### Scenario: Pick a column

- **WHEN** the user presses `S`, selects `RESTARTS` and presses Enter
- **THEN** rows sort ascending by RESTARTS
- **AND** the flash reads `sort by RESTARTS ↑ asc`

### Requirement: Re-picking inverts

Choosing the active sort column again in the picker SHALL invert its direction, and a newly chosen column SHALL sort ascending.

#### Scenario: Re-pick

- **WHEN** rows sort ascending by AGE
- **AND** the user picks AGE again
- **THEN** the flash reads `sort by AGE ↓ desc`

### Requirement: Clear sort

Choosing `default (ns/name)` SHALL clear the sort, flash `sort by default (ns/name)`, forget any remembered sort for the kind, and keep the cleared state across column updates, wide mode, reload and refresh in that view.

#### Scenario: Default entry

- **WHEN** a sort is active and the user picks `default (ns/name)`
- **THEN** rows return to namespace/name order

### Requirement: Invert sort

`I` SHALL invert the active sort direction and flash `sort by <HEADER> ↑ asc` or `↓ desc`; with no active sort it SHALL flash `press S to pick a sort column first`.

#### Scenario: No sort yet

- **WHEN** no sort column is active
- **AND** the user presses `I`
- **THEN** a warning flash reads `press S to pick a sort column first`

### Requirement: Sort by age

`A` SHALL sort by AGE like picking it in the sort picker, inverting on repeat; when the view has no AGE column it SHALL flash `view has no AGE column` and keep the current sort.

#### Scenario: Repeat inverts

- **WHEN** the user presses `A` twice
- **THEN** rows sort by AGE descending

### Requirement: Sort indicator

The sorted column header SHALL show ` ↑` for ascending or ` ↓` for descending, and rows tied on the sort value SHALL fall back to namespace then name ascending.

#### Scenario: Descending arrow

- **WHEN** rows sort descending by RESTARTS
- **THEN** the header shows `RESTARTS ↓`

### Requirement: Time and numeric sort semantics

AGE SHALL sort youngest first ascending with unknown last; LAST-SEEN, Helm UPDATED and LAST-SCHEDULE SHALL sort most recent first ascending with unknown last; job DURATION, Helm REVISION and count columns such as READY, RESTARTS and DATA SHALL sort numerically, while Flux READY sorts as text.

#### Scenario: Restart counts

- **WHEN** pods have RESTARTS `10` and `9`
- **THEN** ascending sort lists `9` before `10`

### Requirement: Typed view columns sort by type

User view, printer and server columns SHALL sort by their declared type (number, quantity, time, text) rather than display text, and a user column that redefines a curated header SHALL sort by its own values.

#### Scenario: Quantity sort

- **WHEN** a quantity column holds `500m` and `2`
- **THEN** ascending sort lists `500m` first

### Requirement: Configured initial sort

A view `sort = "<HEADER>"`, `"<HEADER>:asc"` or `"<HEADER>:desc"` SHALL set the initial sort when no sort is active; an invalid direction SHALL fall back to ascending with a warning, and a missing column SHALL flash `view sort column '<HEADER>' not found`.

#### Scenario: Missing configured column

- **WHEN** `[views.pods] sort = "NOPE"` is configured
- **AND** the user opens pods
- **THEN** a warning flash reads `view sort column 'NOPE' not found`

### Requirement: Global default sort

`[views."*"].sort` SHALL supply the initial sort for views without their own `sort`, SHALL NOT supply columns, and SHALL be silently ignored until the column is available, rechecked when printer columns arrive or wide mode changes.

#### Scenario: Wildcard sort

- **WHEN** `[views."*"] sort = "AGE:desc"` is configured
- **AND** the user opens a kind with an AGE column and no own sort
- **THEN** rows sort by AGE descending

### Requirement: Remembered sorts

With `remember_sort` enabled (the default) a sort picked with `S`, `I`, `A` or a header click SHALL be saved per kind in `<state-dir>/sort.toml` and restored on the next visit, taking priority over a configured sort; a save error SHALL flash `failed to save sort state: <error>`.

#### Scenario: Restored after revisit

- **WHEN** the user sorts pods by RESTARTS and opens services then pods again
- **THEN** pods sort by RESTARTS again

### Requirement: Remember sort disabled

With `remember_sort = false` sort changes SHALL apply only to the active view, SHALL NOT be written to disk, and opening the view again SHALL apply the configured sort; existing saved entries SHALL remain on disk.

#### Scenario: Temporary sort

- **WHEN** `remember_sort = false` and the user sorts pods by RESTARTS
- **AND** the user opens another kind then pods again
- **THEN** pods use the configured or default order

### Requirement: Sort waits for hidden columns

A user or remembered sort whose column is hidden or not yet available SHALL wait while the table uses natural order, and SHALL return with its direction when the column reappears unless the user cleared or replaced it.

#### Scenario: Wide-only column

- **WHEN** pods were sorted by NODE in wide mode and the user turns wide mode off
- **AND** turns wide mode on again
- **THEN** rows sort by NODE again

### Requirement: Root view switch resets sort

Switching the root resource kind SHALL clear the current sort and apply the new kind's remembered or configured sort.

#### Scenario: New kind sort

- **WHEN** pods are sorted by RESTARTS and the user runs `:services`
- **THEN** services use their own remembered, configured or default order

### Requirement: Faults toggle

In a pods view `ctrl-z` SHALL toggle a faults-only filter showing only pods that are terminating, not Running (Succeeded hidden), not Ready, with a non-True readiness gate, with missing or unready containers, or with a failed init container or unready native sidecar, and SHALL select the first row.

#### Scenario: Faulty pods only

- **WHEN** pods include a healthy Running pod and pods in `CrashLoopBackOff`, `Pending` and `Succeeded`
- **AND** the user presses `ctrl-z`
- **THEN** only the `CrashLoopBackOff` and `Pending` pods are listed
- **AND** the title shows ` [faults]` in red

### Requirement: Faults filter combines and tracks updates

The faults filter SHALL combine with the `/` filter, re-evaluate on watch updates, persist for the session, and apply only while the kind is pods; `ctrl-z` SHALL have no faults effect in other kinds.

#### Scenario: Pod recovers

- **WHEN** faults-only is on and a listed pod becomes Ready
- **THEN** the pod disappears from the table

#### Scenario: Other kinds unaffected

- **WHEN** faults-only is on and the user switches to services
- **THEN** all services are listed and no ` [faults]` marker is shown

### Requirement: Configured ctrl-z bindings take precedence

When a bookmark, workspace or in-scope plugin is bound to `ctrl-z`, pressing it in pods SHALL run that binding instead of toggling faults.

#### Scenario: Bookmark on ctrl-z

- **WHEN** a bookmark for `services` is bound to `ctrl-z`
- **AND** the user presses `ctrl-z` in pods
- **THEN** the services view opens and faults-only stays off
