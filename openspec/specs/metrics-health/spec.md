# metrics-health Specification

## Purpose

Describe how sofka shows live resource usage and health: metrics-server CPU and memory columns, request and limit utilization, node pod counts and trends, per-container metrics, `[thresholds]` severity bands, node roles, and the health STATUS text of workloads, Jobs, storage and nodes.

## Requirements

### Requirement: Default pod metric columns

The core Pods view SHALL append `CPU` and `MEM` columns after `AGE`. A default metric column MUST be skipped when the view already has a column with the same header or the same metric source, and MUST NOT be added when a `[views]` overlay sets `replace = true` with at least one valid column.

#### Scenario: Overlay column keeps the defaults

- **WHEN** a `[views]` overlay for pods adds a `%CPU/R` column before `AGE`
- **THEN** the Pods header reads `NAME READY STATUS RESTARTS %CPU/R AGE CPU MEM`

#### Scenario: Replacing view drops defaults

- **WHEN** a pods view sets `replace = true` with its own valid columns
- **THEN** no default `CPU` or `MEM` column is appended

### Requirement: Default node metric columns

The core Nodes view SHALL append the columns `PODS`, `CPU`, `MEM`, `%CPU` and `%MEM` in that order, with the same skip rules as the pod defaults.

#### Scenario: Node columns in order

- **WHEN** the user opens the Nodes view without view overrides
- **THEN** the trailing columns are `PODS CPU MEM %CPU %MEM`

### Requirement: Metrics polling

While the current view is core Pods or Nodes and the metrics API is discovered, sofka SHALL poll `pods.metrics.k8s.io` per watched namespace, or `nodes.metrics.k8s.io` cluster-wide, every 5 seconds. Other views MUST NOT poll metrics.

#### Scenario: Pods view polls pod metrics

- **WHEN** the Pods view is open in namespace `prod` on a cluster with metrics-server
- **THEN** sofka lists pod metrics for `prod` every 5 seconds

#### Scenario: Services view does not poll

- **WHEN** the user switches to the Services view
- **THEN** no metrics list request is sent

### Requirement: Metrics degrade without metrics-server

When the metrics API is not discovered, sofka MUST NOT send metrics requests and SHALL show `-` in every usage cell. Request and limit totals MUST still be shown because they come from the pod spec.

#### Scenario: Cluster without metrics-server

- **WHEN** the Pods view opens on a cluster without the metrics API
- **THEN** the `CPU` and `MEM` cells show `-`
- **AND** a `CPU/R` column still shows the pod's summed CPU request

### Requirement: Metrics poll errors

A failed metrics list SHALL be recorded as a metrics error, reported only through the diagnostics report (see the diagnostics capability). The next successful poll MUST clear the error, and data from namespaces that did list successfully MUST still be shown.

#### Scenario: Recovery clears the error

- **WHEN** one metrics poll fails and the next succeeds
- **THEN** the metrics error is cleared and the cells show the new sample

### Requirement: Metrics reset on view and context change

Changing view or context SHALL clear pod metrics, container metrics and node pod counts, and samples that arrive for an earlier view MUST be discarded.

#### Scenario: Late sample after a view switch

- **WHEN** a metrics sample for the previous view arrives after the user switched views
- **THEN** the sample is ignored and the new view shows `-` until its own sample arrives

### Requirement: Pod usage aggregation

A pod's CPU and memory usage SHALL be the sum of its containers' usage. CPU MUST accept `n`, `u` and `m` suffixes and bare cores; memory MUST accept decimal (`k`, `M`, `G`, `T`, `P`, `E`) and binary (`Ki` … `Ei`) suffixes, rounding fractional bytes up.

#### Scenario: Two containers summed

- **WHEN** a pod has containers using `100m` and `250000000n` CPU
- **THEN** its `CPU` cell shows `350m`

### Requirement: Usage cell formatting

CPU cells SHALL show whole millicores (`250m`). Memory cells SHALL show `NMi` below 1Gi and `X.YGi` at or above it. Percent cells SHALL show a rounded `N%`. A measured zero MUST show `0m` or `0Mi`, and a missing sample MUST show `-`.

#### Scenario: Gigabyte memory

- **WHEN** a pod uses 1.5Gi of memory
- **THEN** its `MEM` cell shows `1.5Gi`

#### Scenario: Missing sample

- **WHEN** no metrics sample exists for a pod
- **THEN** its `CPU` and `MEM` cells show `-`

### Requirement: Sorting metric columns

Sorting by a metric column SHALL compare numeric values. Rows without a value MUST sort first in ascending order and last in descending order.

#### Scenario: Descending CPU sort

- **WHEN** pods are sorted by `CPU` descending and one pod has no sample
- **THEN** the pod without a sample is listed last

### Requirement: Filtering metric columns

Filters such as `cpu>=500m` or `mem<1Gi` SHALL compare numeric values. A row with an unknown value MUST NOT match any numeric metric filter, including negated (`!(cpu>=500m)`) and inequality (`cpu!=1`) forms. Invalid memory quantities such as `1K`, `1Xi`, `-1Mi`, `NaN` or `inf` MUST produce a filter error.

#### Scenario: Unknown usage never matches

- **WHEN** the filter is `!(cpu>=500m)` and a pod has no metrics sample
- **THEN** that pod is hidden

#### Scenario: Invalid memory quantity

- **WHEN** the user types the filter `mem>1Xi`
- **THEN** sofka reports a filter error

### Requirement: Live re-sort on new samples

When a new metrics sample or node pod count arrives and the active sort or filter uses a metric column, the rows SHALL be re-sorted and re-filtered with the new values.

#### Scenario: Usage change moves a row

- **WHEN** pods are sorted by `CPU` descending and a new sample makes the last pod the busiest
- **THEN** that pod moves to the top

### Requirement: Node utilization percentages

`%CPU` and `%MEM` SHALL show node usage divided by `status.allocatable`. When allocatable is missing or zero, or usage is unknown, the cell MUST show `-`.

#### Scenario: Different node sizes

- **WHEN** two nodes each use `1000m` CPU and have 2 and 4 allocatable cores
- **THEN** their `%CPU` cells show `50%` and `25%`
- **AND** sorting `%CPU` descending lists the 2-core node first

### Requirement: Configurable metric sources

A `[views]` column with `metric = "<source>"` SHALL show that metric. Supported sources MUST be `cpu` and `memory` (pods and nodes); `cpu-request`, `cpu-limit`, `memory-request`, `memory-limit` and the four matching `-utilization` sources (pods); and `node-pods`, `node-cpu-utilization`, `node-memory-utilization`, `node-cpu-trend` and `node-memory-trend` (nodes).

#### Scenario: Request column on pods

- **WHEN** a pods view defines a column `CPU/R` with `metric = "cpu-request"`
- **THEN** each pod row shows its total CPU request in that column

### Requirement: Unsupported metric source warning

A metric column whose source does not apply to the view's resource SHALL be skipped with the configuration warning `view {group}/{plural}: column {header} is not available for this resource; column skipped`.

#### Scenario: Node metric on pods

- **WHEN** a pods view defines a column with `metric = "node-pods"`
- **THEN** the column is not shown and the warning names that column

### Requirement: Pod request and limit totals

Pod request and limit totals SHALL sum app containers and native sidecars (init containers with `restartPolicy: Always`), excluding regular init containers and pod overhead. A pod-level `spec.resources` value MUST take priority. A request total MUST show `-` only when no included container sets it; a limit total MUST show `-` when any included container lacks that limit.

#### Scenario: Sidecar counted in requests

- **WHEN** a pod has container CPU requests `4` and `100m` and a native sidecar requesting `100m`
- **THEN** its `CPU/R` total is `4200m`

#### Scenario: Missing limit

- **WHEN** one of a pod's two containers has no memory limit
- **THEN** its `MEM/L` cell shows `-`

### Requirement: Request and limit utilization

Utilization columns SHALL show usage as a rounded percentage of the request or limit total. The cell MUST show `-` when the total is missing or zero or usage is unknown.

#### Scenario: Full request utilization

- **WHEN** a pod requesting `4200m` CPU uses `4200m`
- **THEN** its `%CPU/R` cell shows `100%` in the critical color

### Requirement: Metric column filters by header

A filter SHALL address a custom metric column by its header, case-insensitively, and compare it numerically with the column's unit.

#### Scenario: Custom headers in filters

- **WHEN** pods have columns `LOAD` (cpu-request-utilization), `CPU/R` and `MEM/R`
- **THEN** the filters `load>=75%`, `cpu/r>4` and `mem/r>=1Gi` each match the pod using 100% of a `4200m` request with a 1.1Gi memory request

### Requirement: Node pod counts

The `PODS` column SHALL count each node's non-terminated pods (phase not `Succeeded` or `Failed`) from one cluster-wide pod watch. It MUST show `-` until the first count arrives, then `0` for nodes without pods, and counts MUST update at most once per second as pods move or are deleted.

#### Scenario: Node without pods

- **WHEN** the first pod count arrives and node `idle` runs no pods
- **THEN** the `PODS` cell for `idle` shows `0`

#### Scenario: Pod deleted

- **WHEN** a running pod on node `a` is deleted
- **THEN** the `PODS` count for `a` drops by one

### Requirement: Node pod count fallback

When the pod watch is forbidden but listing is allowed, sofka SHALL fall back to listing pods every 10 seconds without further watch attempts. Other watch errors MUST be retried with a backoff starting at 800ms and doubling up to 30 seconds.

#### Scenario: List-only permissions

- **WHEN** the user can list but not watch pods cluster-wide
- **THEN** the `PODS` column refreshes every 10 seconds from a pod list

### Requirement: Node trend columns

The `node-cpu-trend` and `node-memory-trend` columns SHALL show 12 characters, each a 25-second bin with the newest on the right, drawing the bin's peak percentage of allocatable as one of ` ▁▂▃▄▅▆▇█`. A bin without a sample, or a node without allocatable, MUST show `·`. These columns MUST NOT appear unless configured in a view.

#### Scenario: Empty history

- **WHEN** a node trend column is first shown
- **THEN** its cell shows twelve `·` characters until samples arrive

### Requirement: Node trend sorting and filtering

Sorting, filtering and coloring of a node trend column SHALL use the latest sampled percentage.

#### Scenario: Filter on the latest value

- **WHEN** a node's latest sample is 100% CPU and 50% memory
- **THEN** `cpu-trend>=100` matches it
- **AND** `mem-trend>60` does not match it

### Requirement: Trend history is session-local

Node trend history SHALL reset on view change, context change, and when a node's UID changes. Trend history MUST NOT be written to disk.

#### Scenario: Node recreated

- **WHEN** a node is deleted and recreated with the same name
- **THEN** its trend history starts empty

### Requirement: Container picker metrics

Enter on a pod SHALL open a container picker titled `Containers · <QoS>`, with columns `NAME READY STATE RESTARTS CPU %R/L MEM %R/L`. Each `%R/L` cell MUST show `request%/limit%`, using `-` for a missing side, and `-/-` when usage is unknown.

#### Scenario: Request and limit percentages

- **WHEN** a container uses 50% of its CPU request and 25% of its CPU limit
- **THEN** its CPU `%R/L` cell shows `50%/25%`

#### Scenario: No limit

- **WHEN** a container uses 13% of its memory request and has no memory limit
- **THEN** its MEM `%R/L` cell shows `13%/-`

### Requirement: Pod QoS class

The container picker title SHALL show the pod's `status.qosClass`. When it is absent, sofka MUST derive `Guaranteed`, `Burstable` or `BestEffort` from the container requests and limits.

#### Scenario: Derived BestEffort

- **WHEN** a pod has no `status.qosClass` and no container sets requests or limits
- **THEN** the picker title reads `Containers · BestEffort`

### Requirement: Container utilization colors

In the container picker, each `%R/L` cell SHALL be colored from the limit percentage, or from the request percentage when no limit exists, using the utilization band: red at or above critical, yellow at or above warn, green below warn. A cell without a base MUST use the muted color. RESTARTS MUST use the restarts band.

#### Scenario: Limit drives the color

- **WHEN** a container is at 95% of its limit and 40% of its request
- **THEN** its `%R/L` cell is red

### Requirement: Container usage trend

When the container picker is at least 84 columns wide and has room, it SHALL show a trend section for the selected container headed `Selected container: last 5 min, 5 s bins; · = no sample`, with 60-sample CPU and MEM sparklines labelled `CPU 0..{max}m` and `MEM 0..{max}`.

#### Scenario: Trend labels

- **WHEN** the selected container's peak CPU sample is `100m`
- **THEN** the trend section shows `CPU 0..100m`

### Requirement: Container trend reset

The container trend SHALL restart when the selected container changes, the pod UID changes, the pod is deleted, or the picker closes. A failed metrics poll MUST record a gap shown as `·`, not a zero, and samples for an earlier view MUST be ignored.

#### Scenario: Metrics error leaves a gap

- **WHEN** a metrics poll fails while the container picker is open
- **THEN** the newest trend bin shows no sample instead of zero

### Requirement: Built-in thresholds

sofka SHALL use built-in threshold bands of restarts warn 1 / critical 5, CPU warn `200m` / critical `1`, memory warn `256Mi` / critical `1Gi`, and utilization warn 75 / critical 90 percent.

#### Scenario: Default restart band

- **WHEN** no `[thresholds]` are configured and a pod has 5 restarts
- **THEN** its `RESTARTS` cell uses the critical color

### Requirement: Threshold band evaluation

A value SHALL be critical when it is at or above the critical bound, otherwise warn when at or above the warn bound, otherwise normal. Critical MUST be checked first, so a critical bound below the warn bound still escalates.

#### Scenario: Inclusive bound

- **WHEN** the CPU warn bound is `200m` and a pod uses exactly `200m`
- **THEN** its `CPU` cell uses the warn color

### Requirement: Threshold configuration layering

`[thresholds]` SHALL override built-in bands per bound, and `[thresholds.resources.<key>]` SHALL override the global values per bound. An omitted bound MUST keep the lower layer's value. Per-cluster and per-context config overrides MUST merge in the same way.

#### Scenario: Only warn overridden

- **WHEN** the config sets `[thresholds.restarts] warn = 3` and no critical bound
- **THEN** the restarts band is warn 3 / critical 5

### Requirement: Per-resource threshold keys

Per-resource threshold keys SHALL be lowercase and resolve most-specific first: `apiVersion/plural`, then `group/plural`, then `plural`, then the lowercased kind.

#### Scenario: Plural key

- **WHEN** the config has `[thresholds.resources.pods.memory] warn = "512Mi"`
- **THEN** pod `MEM` cells use 512Mi as the warn bound while nodes keep the global bound

### Requirement: Invalid threshold warnings

An invalid or non-positive threshold quantity SHALL be ignored with a warning such as `thresholds.cpu.warn: invalid quantity 'banana' (ignored)` or `thresholds.resources.pods.memory.warn: …`, and the lower layer's value MUST be kept.

#### Scenario: Unparsable CPU bound

- **WHEN** `[thresholds.cpu] warn = "banana"` is configured
- **THEN** sofka warns `thresholds.cpu.warn: invalid quantity 'banana' (ignored)`
- **AND** the CPU warn bound stays `200m`

### Requirement: Threshold reload

`:reload` and context switches SHALL recompile `[thresholds]`. `:reload` MUST show threshold warnings; at startup they MUST be printed to standard error only.

#### Scenario: Reload applies new bands

- **WHEN** the user edits `[thresholds.cpu] warn` and runs `:reload`
- **THEN** CPU cells are recolored with the new bound

### Requirement: Threshold cell colors

`RESTARTS` SHALL use the restarts band (unparsable counts as 0), CPU and memory cells the cpu and memory bands, and percentage cells the utilization band. `RESTARTS`, `CPU` and `MEM` MUST be peach at warn and red at critical, otherwise the row color; percentage cells MUST be red, yellow, green, or muted when unknown. A marked row's highlight MUST override threshold colors.

#### Scenario: Warn memory

- **WHEN** a pod uses `300Mi` with default thresholds
- **THEN** its `MEM` cell is peach

### Requirement: Workload health status

Deployment, StatefulSet, ReplicaSet and DaemonSet STATUS SHALL be the first match of: `Terminating` (deletion started), `Progressing` (generation not observed), `ScaledDown` or `Progressing` (zero desired), `Stalled` (ReplicaFailure or ProgressDeadlineExceeded), `Unavailable` (none ready), `Progressing` (rollout incomplete), `Unavailable` (Available=False), `Ready`, else `Degraded`.

#### Scenario: Rollout before availability

- **WHEN** a Deployment wants 3 replicas, has 3 current, 1 updated, 1 ready and `Available=False`
- **THEN** its STATUS is `Progressing`

#### Scenario: Ready but not available

- **WHEN** a Deployment has 3 desired, 3 updated, 3 ready and `Available=False`
- **THEN** its STATUS is `Unavailable`

#### Scenario: Stalled rollout

- **WHEN** a Deployment's Progressing condition has reason `ProgressDeadlineExceeded`
- **THEN** its STATUS is `Stalled`

### Requirement: Workload replica counting

Desired replicas SHALL be `spec.replicas` (default 1), or `desiredNumberScheduled` for DaemonSets. StatefulSets and DaemonSets with an `OnDelete` update strategy MUST count all desired replicas as updated, a StatefulSet partition MUST count as updated, and a ReplicaSet's updated count MUST be its current replicas.

#### Scenario: OnDelete StatefulSet

- **WHEN** a StatefulSet uses `OnDelete` and all replicas are ready
- **THEN** its STATUS is `Ready` even though `updatedReplicas` is lower

### Requirement: Job health status

Job STATUS SHALL be the first match of `Terminating`, `Failed` (Failed or FailureTarget condition True), `Completed` (Complete True), `Completing` (SuccessCriteriaMet True), `Suspended` (`spec.suspend` or Suspended True), `Running` (active pods), else `Pending`.

#### Scenario: Failed without active pods

- **WHEN** a Job has `Failed=True` and no active pods
- **THEN** its STATUS is `Failed` and the row uses the error color

### Requirement: Storage health status

PersistentVolume and PersistentVolumeClaim STATUS SHALL show `Terminating` once deletion has started, even while a protection finalizer keeps the object, and otherwise `status.phase`.

#### Scenario: Protected claim being deleted

- **WHEN** a bound PVC has a deletionTimestamp and the `kubernetes.io/pvc-protection` finalizer
- **THEN** its STATUS is `Terminating`

### Requirement: Node health status

Node STATUS SHALL be `Ready` when the Ready condition is True, `NotReady` when it has any other status, or `Unknown` when there is no Ready condition, with `,SchedulingDisabled` appended when `spec.unschedulable` is true.

#### Scenario: Cordoned node

- **WHEN** a Ready node is cordoned
- **THEN** its STATUS is `Ready,SchedulingDisabled`

### Requirement: Node roles

The Nodes `ROLES` column SHALL list roles from `node-role.kubernetes.io/<role>` label suffixes and non-empty `kubernetes.io/role` values, plus suffixes of `[node_roles] label_prefixes` and values of `label_keys`, sorted, deduplicated and comma-joined, or `<none>` when there are none. Filtering and sorting MUST use the same text.

#### Scenario: Configured label key

- **WHEN** `[node_roles] label_keys = ["node.example.com/pool"]` is set and a node has `node.example.com/pool=gpu` and `node-role.kubernetes.io/worker`
- **THEN** its ROLES cell is `gpu,worker` and the filter `roles=worker` matches it

### Requirement: Node role configuration warnings

An empty entry in `[node_roles] label_prefixes` SHALL be ignored with the warning `node_roles.label_prefixes: empty prefix ignored`. Per-cluster and per-context overrides MUST replace each array they set, and `:reload` MUST apply changes.

#### Scenario: Empty prefix

- **WHEN** `label_prefixes = [""]` is configured
- **THEN** sofka shows the warning `node_roles.label_prefixes: empty prefix ignored`

### Requirement: Custom ROLES columns

A view column named `ROLES` with its own `path` SHALL show that path's value, while a column with `builtin = "ROLES"` MUST use the node role rules and `[node_roles]` settings.

#### Scenario: Builtin roles column

- **WHEN** a nodes view adds a column with `builtin = "ROLES"`
- **THEN** that column shows the same roles as the default ROLES column
