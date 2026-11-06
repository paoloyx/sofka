## ADDED Requirements

### Requirement: CronJob STATUS cell

CronJob STATUS SHALL be the first match of `Terminating` when deleting, `Suspended` when `spec.suspend` is true, `Running` with active jobs, `Failed` when the last schedule time is set and the last successful time is missing or earlier, else `Scheduled`.

#### Scenario: Last run failed

- **WHEN** a CronJob that is not suspended and has no active jobs was last scheduled at 10:00 and last succeeded at 09:00
- **THEN** its STATUS shows `Failed` and the row is red

### Requirement: Workloads view columns

The `:workloads` view (alias `:wk`) SHALL show NAME, KIND, READY, STATUS and AGE. READY SHALL show ready containers for pods, ready/desired replicas for deployments and statefulsets, `numberReady/desiredNumberScheduled` for daemonsets, `succeeded/completions` for jobs and nothing for cronjobs, and STATUS SHALL be the STATUS cell of the row's own kind.

#### Scenario: Mixed kinds

- **WHEN** the workloads view lists deployment `api` with 0 of 2 replicas ready, daemonset `agent` with 3 of 3 ready, cronjob `nightly` and a running pod `web`
- **THEN** their KIND, READY and STATUS cells are `Deployment` `0/2` `Unavailable`, `DaemonSet` `3/3` `Ready`, `CronJob` with an empty READY and `Scheduled`, and `Pod` `1/1` `Running`

### Requirement: Workloads view rows

The workloads view SHALL list the live Deployments, StatefulSets, DaemonSets, CronJobs, Jobs and Pods of the namespace together, skipping kinds the cluster does not serve, and SHALL hide pods owned by a ReplicaSet, StatefulSet, DaemonSet or Job and jobs owned by a CronJob. `O` SHALL toggle those owned rows, flash `showing owned pods and jobs` or `showing top-level workloads`, and select the first row.

#### Scenario: Toggle owned rows

- **WHEN** the workloads view has deployments `web` and `api`, statefulset `db`, daemonset `agent`, cronjob `nightly`, job `migrate` and pod `web`
- **AND** pod `web-abc` is owned by a ReplicaSet and job `nightly-1` by cronjob `nightly`
- **THEN** seven rows are listed, without `web-abc` and `nightly-1`
- **AND** pressing `O` lists nine rows and pressing it again lists seven

### Requirement: Column default values

A `path` column's `default` SHALL be shown instead of `<none>` when every path is missing or null, and SHALL be formatted, sorted, filtered and coloured like a read value; on a `condition` column it SHALL stand in for a missing condition. Present values, empty strings included, SHALL be kept. A `default` on a `builtin` or `metric` column SHALL be ignored with the warning `views."<key>": column <HEADER>: default applies only to path columns; ignored`.

#### Scenario: Missing phase uses the default

- **WHEN** a certificates view column `PHASE` has `path = "/status/phase"`, `type = "status"` and `default = "Pending"`
- **AND** one certificate has no phase and another a null phase
- **THEN** both PHASE cells show `Pending` and both rows are coloured as pending
- **AND** the filter `pending` lists exactly those two certificates

#### Scenario: Missing condition uses the default

- **WHEN** a `condition` column `SYNCED` has `path = "Synced"` and `default = "False"`
- **AND** a certificate has no Synced condition
- **THEN** its SYNCED cell shows `False` and the row is red
- **AND** the filter `synced=false` lists it

#### Scenario: Formatted default

- **WHEN** a `quantity` column `REQ` with `format = "cpu"` and `default = "0"` finds no value
- **THEN** the cell shows `0m`
- **AND** the filter `req<100m` lists the row

### Requirement: Regex value extraction

A `path` column's `regex` SHALL show the first capture group of the first match, or the whole match when the pattern has no group, taken from the value the paths selected; no match, an unmatched first group or a non-string value SHALL keep the usual display, and a `default` SHALL be shown as written. Sorting, filters and status colours SHALL use the extracted value, and `number`, `quantity` and `time` columns SHALL parse it after extraction.

#### Scenario: Extract a node group

- **WHEN** a nodes view column `NODEGROUP` reads the EKS nodegroup label with `regex = '^(.*)-[0-9]{18}[0-9a-f]{8}$'`
- **AND** node `a` has the label `default-a-20250314092215118700000003` and node `b` has `system-b-2025031409221512340000000c`
- **THEN** NODEGROUP shows `default-a` for node `a`
- **AND** the filter `nodegroup=system-b` lists only node `b`

#### Scenario: Unmatched and missing values

- **WHEN** a column `OS` reads `/status/nodeInfo/osImage` with `regex = '^(.*?) \('` and `default = "none (yet)"`
- **THEN** `Bottlerocket OS 1.66.0 (aws-k8s-1.36)` shows `Bottlerocket OS 1.66.0`
- **AND** `Ubuntu 24.04` is shown unchanged and a node without an OS image shows `none (yet)`

#### Scenario: Numeric extraction

- **WHEN** a `number` column `PODS` with `regex = '[0-9]+'` reads `12 pods`, `3 pods` and `none`
- **THEN** the cells show `12`, `3` and `none`
- **AND** the filter `pods>5` lists only the node with `12 pods`

### Requirement: Regex column warnings

A column whose `regex` does not compile, that sets both `format` and `regex`, or that sets `regex` on a `builtin`, `metric` or `condition` column SHALL be skipped with the warning `views."<key>": column <HEADER>: <reason>; column skipped`, where the reason is `invalid regex '<pattern>' (<error>)`, `set only one of format or regex` or `regex applies only to path columns`.

#### Scenario: Invalid regex

- **WHEN** a view column `INVALID` sets `regex = '('`
- **THEN** a configuration warning contains `column INVALID: invalid regex '('`
- **AND** the column is skipped

### Requirement: Named view layouts

A view key with a `:<name>` suffix, on any key form and before an optional `@<namespace>` (`pods:images@matlab`), SHALL define a named layout next to the kind's default view. With a layout active, sofka SHALL try `<key>:<name>@<namespace>` for every key form, then `<key>:<name>`, then the default view. A layout SHALL set only `columns`, `replace`, `pin` and `sort`, SHALL NOT inherit columns from the base key, and its name SHALL ignore case.

#### Scenario: Namespace variant of a layout

- **WHEN** `[views."v1/pods:images"]` shows IMAGE and `[views."pods:images@prod"]` shows PRODIMAGE
- **AND** layout `images` is active
- **THEN** pods in namespace `prod` show PRODIMAGE and not IMAGE
- **AND** pods in namespace `dev` show IMAGE

### Requirement: Named layout warnings

A layout name that is empty, `default` or `wide`, or that contains whitespace, `@`, `:` or `/` SHALL ignore the view with the warning `views."<key>": invalid layout name '<name>' (must not be empty, default, or wide, or contain spaces, '@', ':' or '/'); view ignored`. `drill`, `refs`, `children` and `node` SHALL always come from the base key; setting one on a layout SHALL warn `views."<key>": <field> is ignored on a named layout; set it on the base view`.

#### Scenario: Reserved layout name

- **WHEN** the config defines `[views."pods:wide"]`
- **THEN** a configuration warning reports an invalid layout name and the view is ignored

### Requirement: Selecting a named layout

`:<resource> -o <name>`, `:<resource>:<name>` and `:layout <name>` SHALL switch the kind to that layout and apply its `sort`; `default` SHALL return to the default columns and `wide` SHALL turn on wide mode. `:layout` without a name SHALL list the kind's layouts with `default` first. A missing layout SHALL warn `<plural> has no layout '<name>'` and keep the current one. The layout SHALL be remembered per kind for the session and shown in the header as `<plural>:<name>`.

#### Scenario: Open pods in a layout

- **WHEN** `[views."v1/pods:images"]` replaces the columns with NAME and IMAGE and sets `sort = "IMAGE:desc"`
- **AND** the user runs `:pods -o images`
- **THEN** the table shows IMAGE and not STATUS, sorted by IMAGE descending

#### Scenario: Unknown layout

- **WHEN** the user runs `:pods -o nope`
- **THEN** a warning flash contains `no layout 'nope'` and the columns stay unchanged

#### Scenario: Layout command

- **WHEN** pods are open and the user runs `:layout slim`
- **THEN** the slim columns are shown and the flash reads `layout: pods:slim`
- **AND** `:layout` alone lists `default`, `images` and `slim`

#### Scenario: Remembered per kind

- **WHEN** pods use layout `images` and the user opens deployments and then pods again
- **THEN** pods still use layout `images`

### Requirement: Layout survives reload

`:reload` SHALL keep the active layout while the configuration still defines it; otherwise the kind SHALL return to its default view with the warning `layout '<name>' no longer exists for <plural>; using the default`.

#### Scenario: Layout removed from the config

- **WHEN** pods use layout `images` and the configuration no longer defines it
- **AND** the user runs `:reload`
- **THEN** pods show the default columns
- **AND** the warning `layout 'images' no longer exists for pods; using the default` is recorded

### Requirement: Pinned columns

A view's `pin` SHALL list headers, matched ignoring case and surrounding spaces, that render first in list order and stay fixed while the other columns keep their order and scroll; `pin = []` SHALL fix no column. NAMESPACE SHALL be accepted without being declared and, when shown but not pinned, SHALL be the first scrolling column. A header the view lacks even in wide mode SHALL be ignored with the warning `view <view>: pin column <HEADER> is not in this view; ignored`.

#### Scenario: Pin status before name

- **WHEN** `[views."v1/pods"]` sets `pin = ["status", " NAME "]`
- **THEN** the columns start with STATUS, NAME and READY
- **AND** pressing Right leaves STATUS and NAME in place
- **AND** clicking the first header sorts by STATUS

#### Scenario: Unknown pin header

- **WHEN** `[views."v1/pods"]` sets `pin = ["NOPE", "NODE", "STATUS"]`
- **THEN** the warning `view pods: pin column NOPE is not in this view; ignored` is recorded
- **AND** STATUS is the first column, with the wide column NODE fixed before it once wide mode shows it

#### Scenario: Pinned columns fill the table

- **WHEN** the pinned columns are wider than the table
- **THEN** Right does not scroll the table

### Requirement: Workload faults rules

With the faults filter on, deployments, daemonsets and replicasets (`apps` or `extensions`) and statefulsets SHALL be listed when terminating or when their STATUS is neither `Ready` nor `ScaledDown`; jobs when their STATUS is not `Running`, `Completing` or `Completed`; and cronjobs when their STATUS is `Terminating`, `Suspended` or `Failed`.

#### Scenario: Unhealthy deployments

- **WHEN** deployments `ready`, `scaled-down`, `degraded` with 1 of 3 replicas ready and `unavailable` with 0 of 1 ready exist
- **AND** the user presses `ctrl-z`
- **THEN** only `degraded` and `unavailable` are listed

#### Scenario: Failed and pending jobs

- **WHEN** faults-only is on and jobs `running`, `complete`, `failed` and `pending` exist
- **THEN** only `failed` and `pending` are listed

#### Scenario: Faulty cronjobs

- **WHEN** faults-only is on and cronjobs exist that never ran, last succeeded, last failed, are running and are suspended
- **THEN** only the last-failed and the suspended cronjobs are listed

### Requirement: GitOps faults rules

With the faults filter on, Flux Kustomizations, HelmReleases, GitRepositories, HelmRepositories, OCIRepositories and Buckets SHALL be listed when terminating, suspended by `spec.suspend` or without a True Ready condition; ResourceSets, FluxInstances and ResourceSetInputProviders when terminating, annotated `fluxcd.controlplane.io/reconcile: disabled` or not Ready; Argo CD Applications when terminating, not `Healthy` or not `Synced`; and ApplicationSets when terminating or not `Synced`.

#### Scenario: Flux faults

- **WHEN** Kustomizations `ready`, `failed` with Ready False, `reconciling` with Ready Unknown, `no-status`, `suspended` and `terminating` exist
- **AND** the user presses `ctrl-z`
- **THEN** every Kustomization except `ready` is listed

#### Scenario: Argo CD faults

- **WHEN** Applications `healthy`, `degraded`, `progressing`, `out-of-sync` and `terminating` exist
- **AND** the user presses `ctrl-z`
- **THEN** only `degraded`, `progressing`, `out-of-sync` and `terminating` are listed

### Requirement: Event message byte sizes

The events MESSAGE column SHALL show each standalone `<digits> bytes` count of at least 1024 in binary units with one decimal (`80474558 bytes` as `76.7 MiB`), moving to the next unit when the rounded value reaches 1024; smaller counts and counts joined to a preceding word character or `.` SHALL stay as written. Filters SHALL match the shown text, while describe and YAML SHALL keep the original message.

#### Scenario: Image size

- **WHEN** an event message reads `Image size: 80474558 bytes.`
- **THEN** the MESSAGE cell shows `Image size: 76.7 MiB.`
- **AND** the filter `"Image size: 76.7 MiB."` lists the event

#### Scenario: Small and promoted counts

- **WHEN** event messages contain `512 bytes`, `1048575 bytes` and `1048524 bytes`
- **THEN** the cells show `512 bytes`, `1.0 MiB` and `1023.9 KiB`

## MODIFIED Requirements

### Requirement: Rows follow the live watch

The table SHALL list the objects of the active resource kind from a live watch, adding, updating and removing rows as watch events arrive without user input. When RBAC refuses the watch but allows list, the rows SHALL follow a list repeated every 2 seconds instead, applying the adds, changes and deletes between lists.

#### Scenario: New object appears

- **WHEN** the pods table is open
- **AND** the watch delivers a new pod `api`
- **THEN** a row for `api` appears without a refresh

#### Scenario: Watch forbidden

- **WHEN** RBAC allows listing pods but refuses watching them
- **AND** one list returns pods `a` and `b` and the next returns an updated `b` and a new `c`
- **THEN** the row for `a` is removed, `b` shows the update and `c` is added

### Requirement: Curated workload columns

Deployments SHALL show NAME, READY, STATUS, UP-TO-DATE, AVAILABLE, AGE; replicasets NAME, DESIRED, CURRENT, READY, STATUS, AGE; statefulsets NAME, READY, STATUS, AGE; daemonsets NAME, DESIRED, CURRENT, READY, AVAILABLE, STATUS, AGE; jobs NAME, STATUS, COMPLETIONS, DURATION, AGE; cronjobs NAME, STATUS, SCHEDULE, SUSPEND, ACTIVE, LAST-SCHEDULE, AGE.

#### Scenario: Deployment headers

- **WHEN** the user opens `:deployments`
- **THEN** the headers are NAME, READY, STATUS, UP-TO-DATE, AVAILABLE and AGE

#### Scenario: CronJob headers

- **WHEN** the user opens `:cronjobs`
- **THEN** the headers are NAME, STATUS, SCHEDULE, SUSPEND, ACTIVE, LAST-SCHEDULE and AGE

### Requirement: Fallback paths

A `path` list SHALL try each JSON Pointer in order and use the first value that exists and is not null; empty strings, `0`, `false` and empty collections SHALL stop the search, and when all are missing the column's `default` SHALL be shown, or `<none>` without one.

#### Scenario: Second path used

- **WHEN** a NODEPOOL column lists a karpenter label then an EKS nodegroup label
- **AND** a node has only the EKS label
- **THEN** NODEPOOL shows the EKS nodegroup value

### Requirement: Quantity formats

`format = "cpu"` or `"memory"` on a `quantity` path column SHALL show CPU as whole millicores below one core and as cores with at most two decimals, trailing zeros dropped, from one core up, and memory as whole Mi below 1 Gi and Gi with one decimal otherwise, a value whose Mi rounds to 1024 showing `1.0Gi`; zero SHALL show `0m` or `0Mi`, invalid values SHALL keep their original text and sorting SHALL use the unrounded value.

#### Scenario: Memory display

- **WHEN** a `memory` formatted column reads `16374956Ki`
- **THEN** the cell shows `15.6Gi`

#### Scenario: CPU display

- **WHEN** a `cpu` formatted column reads `250m`, `1234m`, `1500m` and `4`
- **THEN** the cells show `250m`, `1.23`, `1.5` and `4`

### Requirement: View column validation warnings

Invalid view columns SHALL be skipped with an in-app configuration warning and never stop the TUI, including empty names, duplicate headers, invalid JSON Pointers, unknown formats, unknown metrics and sources not available for the resource. An unknown key SHALL be ignored with the warning `views."<key>": unknown key '<name>' ignored`, prefixed for a column by `column <HEADER>: ` or `column #<n>: `, while the rest still applies. `:config` SHALL list every view warning after startup, reload and context switches.

#### Scenario: Unavailable builtin

- **WHEN** a pods view declares `builtin = "TAINTS"`
- **THEN** the warning `view /pods: column TAINTS is not available for this resource; column skipped` is recorded

#### Scenario: Misspelled column key

- **WHEN** a `v1/pods` column `NODE` sets `fromat` and the config adds a nested `[views."v1/pods".images]` table
- **THEN** the warnings `views."v1/pods": unknown key 'images' ignored` and `views."v1/pods": column NODE: unknown key 'fromat' ignored` are recorded
- **AND** the NODE column still applies

#### Scenario: Misspelled name key

- **WHEN** the second column of a pods view sets `naem` instead of `name`
- **THEN** the warning `views."pods": column #2: unknown key 'naem' ignored` is recorded before `views."pods": column with empty name skipped`

### Requirement: Horizontal scrolling

Left and Right SHALL scroll the table columns by five text positions, clamped to the available overflow, while the fixed columns stay in place: the view's `pin` columns, or without `pin` NAME and the columns before it. When all columns fit or the fixed columns fill the table, Left and Right SHALL do nothing, and a view change SHALL reset the offset.

#### Scenario: Scroll right

- **WHEN** the table is wider than the terminal
- **AND** the user presses Right
- **THEN** the non-frozen columns shift by five positions and NAME stays in place

### Requirement: Faults toggle

In a pods view, the workloads view or a view of a kind covered by the workload or GitOps faults rules, `ctrl-z` SHALL toggle a faults-only filter and SHALL keep the selected object while it is still listed, else select the first row. Faulty pods SHALL be those terminating, not Running (Succeeded hidden), not Ready, with a non-True readiness gate, with missing or unready containers, or with a failed init container or unready native sidecar; the workloads view SHALL judge each row by its own kind.

#### Scenario: Faulty pods only

- **WHEN** pods include a healthy Running pod and pods in `CrashLoopBackOff`, `Pending` and `Succeeded`
- **AND** the user presses `ctrl-z`
- **THEN** only the `CrashLoopBackOff` and `Pending` pods are listed
- **AND** the title shows ` [faults]` in red

#### Scenario: Selection kept

- **WHEN** pods `a-ok`, `b-crash`, `c-ok` and `d-crash` are listed and `d-crash` is selected
- **AND** the user presses `ctrl-z`
- **THEN** only `b-crash` and `d-crash` are listed and `d-crash` stays selected

#### Scenario: Filtered-out selection

- **WHEN** faults-only is off and the healthy pod `c-ok` is selected
- **AND** the user presses `ctrl-z`
- **THEN** the first faulty row `b-crash` is selected

#### Scenario: Workloads judged per kind

- **WHEN** the workloads view lists a ready deployment, a deployment without ready replicas, a failed job, a pending pod and healthy rows
- **AND** the user presses `ctrl-z`
- **THEN** only the unready deployment, the failed job and the pending pod are listed

### Requirement: Faults filter combines and tracks updates

The faults filter SHALL combine with the `/` filter, re-evaluate on watch updates, persist for the session, and apply only while a view lists a covered kind as itself, so rollout history revisions stay visible; `ctrl-z` SHALL have no faults effect in other kinds.

#### Scenario: Pod recovers

- **WHEN** faults-only is on and a listed pod becomes Ready
- **THEN** the pod disappears from the table

#### Scenario: Other kinds unaffected

- **WHEN** faults-only is on and the user switches to services
- **THEN** all services are listed and no ` [faults]` marker is shown

#### Scenario: Persists across covered kinds

- **WHEN** faults-only is turned on in deployments
- **AND** the user switches to jobs
- **THEN** only faulty jobs are listed

### Requirement: Configured ctrl-z bindings take precedence

When a bookmark, workspace or in-scope plugin is bound to `ctrl-z`, pressing it in a view where the faults filter applies SHALL run that binding instead of toggling faults.

#### Scenario: Bookmark on ctrl-z

- **WHEN** a bookmark for `services` is bound to `ctrl-z`
- **AND** the user presses `ctrl-z` in pods
- **THEN** the services view opens and faults-only stays off
