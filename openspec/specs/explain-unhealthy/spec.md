# explain-unhealthy Specification

## Purpose

Describe the Explain view (`X` / `:explain`): a deterministic, evidence-based list of findings that explains why the selected Kubernetes object is unhealthy, with jumps to blocking pods, their events and logs, and manual or automatic refresh.

## Requirements

### Requirement: Opening Explain

`X` in a resource table and the palette commands `:explain`, `:why` and `:diagnose` SHALL open the Explain view for the selected row, titled `<name> — explain`. While evidence is gathered the view MUST show `gathering evidence…` and the status line MUST show `explaining <name>…`.

#### Scenario: Explain a pod

- **WHEN** the user selects pod `api-1` and presses `X`
- **THEN** the Explain view opens titled `api-1 — explain`
- **AND** the status line shows `explaining api-1…` until findings arrive

### Requirement: Explain preconditions

Explain MUST NOT open without a resource view or a selected row, and MUST NOT open on Helm release or Helm history rows. It SHALL instead warn `select a resource first`, `no selection to explain`, or `explain is not available for Helm releases`.

#### Scenario: Helm row

- **WHEN** the user presses `X` on a Helm release row
- **THEN** sofka warns `explain is not available for Helm releases` and stays in the table

### Requirement: Fresh read before analysis

Opening Explain and each refresh SHALL read the object from the API before analysis. When the read fails, or the object now has a different UID, the view MUST show a single warning finding `evidence incomplete — <reason>` instead of findings from the old snapshot; a replaced object's reason MUST be `<plural>/<name> was replaced; return to the table and select the new resource`.

#### Scenario: Refresh sees the current object

- **WHEN** a healthy Deployment becomes unavailable on the API and the user presses `r` in Explain
- **THEN** the findings report the Deployment as unavailable without waiting for a watch update

#### Scenario: Read denied

- **WHEN** the read of the selected object is forbidden
- **THEN** the view shows one warning finding that contains the API error message

### Requirement: Evidence gathering

For Deployments, StatefulSets, DaemonSets and ReplicaSets, Explain SHALL list pods matching the full workload selector, including `matchExpressions`, in the object's namespace. For every kind it SHALL list events in the namespace and keep only those about the object or its pods, matched by UID, or by name when an event has no UID.

#### Scenario: Selector with expressions

- **WHEN** a Deployment selects `app=web` plus `tier In (frontend,api)`, `environment NotIn (test)`, `enabled` exists and `disabled` does not exist
- **THEN** the pod list request uses `labelSelector=app=web,tier in (api,frontend),environment notin (test),enabled,!disabled`

### Requirement: Incomplete evidence warning

When listing pods or events fails, Explain SHALL still analyse the data it has and MUST put a warning finding `evidence incomplete — listing <plural>: <error>` first.

#### Scenario: Events forbidden

- **WHEN** the user cannot list events and presses `X` on a Deployment
- **THEN** the first finding starts with `evidence incomplete — listing events:`

### Requirement: Finding presentation

Findings SHALL be shown indented by level and colored by severity: headings yellow and bold, good green, warn peach, critical red, informational text in the normal color, and event evidence muted. A finding that has a jump target MUST end with `  →`, and the title MUST show the finding count as `(<n> findings)`.

#### Scenario: Blocking pod marked

- **WHEN** a Deployment's findings list an unready pod
- **THEN** that pod line ends with `→`

### Requirement: Initial cursor

When findings arrive, the cursor SHALL land on the first finding that has a jump target, or on the first finding when none has one, and the `explaining …` progress message MUST be cleared.

#### Scenario: Cursor on first blocker

- **WHEN** findings for a degraded Deployment arrive with a blocking pod
- **THEN** the cursor is on the blocking pod line

### Requirement: Workload headline

For Deployments, StatefulSets, DaemonSets and ReplicaSets the first finding SHALL read `<Kind>/<name> is healthy` (good) when ready replicas reach a non-zero desired count, or when desired and current replicas are both zero. Otherwise it MUST read `is unavailable` when no replica is ready and `is degraded` when some are, both critical.

#### Scenario: Healthy Deployment

- **WHEN** a Deployment wants 3 replicas and has 3 ready
- **THEN** the first finding is `Deployment/api is healthy` in the good color

#### Scenario: Partly ready

- **WHEN** a Deployment wants 5 replicas and has 2 ready
- **THEN** the first finding reads `Deployment/api is degraded` in the critical color

### Requirement: Rollout summary

Under a `Rollout` heading, workloads SHALL show `desired D → updated U → ready R (available A)`, and DaemonSets MUST show `desired D · ready R · available A` with available read from `status.numberAvailable` (0 when absent). The line MUST be critical when nothing is ready, warn when some are ready, and informational when all are ready.

#### Scenario: Deployment mid-rollout

- **WHEN** a Deployment has 5 desired, 3 updated, 2 ready
- **THEN** a finding contains `desired 5 → updated 3 → ready 2`

#### Scenario: DaemonSet availability

- **WHEN** a DaemonSet has 3 desired, 3 ready and `numberAvailable` 1
- **THEN** a finding reads `desired 3 · ready 3 · available 1`

### Requirement: Generation lag

When a workload's `metadata.generation` differs from `status.observedGeneration`, Explain SHALL add a warning `spec generation G not yet observed (controller at OG)`.

#### Scenario: Controller behind

- **WHEN** a Deployment has generation 7 and observedGeneration 6
- **THEN** a warning reads `spec generation 7 not yet observed (controller at 6)`

### Requirement: Workload condition findings

Workload conditions `Available=False`, `Progressing=False` and `ReplicaFailure=True` SHALL each produce `<Type>: <reason>: <message>`, dropping an empty part or showing `(no detail)` when both are empty. They MUST be critical for reason `ProgressDeadlineExceeded` or type `Available`, and warn otherwise.

#### Scenario: Progress deadline exceeded

- **WHEN** a Deployment has `Progressing=False` with reason `ProgressDeadlineExceeded`
- **THEN** a critical finding starts with `Progressing: ProgressDeadlineExceeded`

### Requirement: Blocking pods

Under a `Blocking objects` heading, a workload's unready pods SHALL be listed by name as `Pod/<name>  <ready>/<total>  <status>`, each with a jump target to that pod, followed by indented container problems for that pod. The heading MUST be omitted when every pod is ready.

#### Scenario: Image pull blocker

- **WHEN** a Deployment's pod `api-7df9-r2m9` in `prod` waits with `ImagePullBackOff`
- **THEN** a finding lists `Pod/api-7df9-r2m9` with a jump target to that pod in `prod`
- **AND** an indented finding mentions `ImagePullBackOff`

### Requirement: Pod headline

For a pod, the first finding SHALL read `Pod/<name> is healthy (<ready>/<total> ready)` when ready, otherwise `Pod/<name> is <status> (<ready>/<total> ready)`. A pod MUST count as not ready while it is terminating, even if its Ready condition is True.

#### Scenario: Terminating ready pod

- **WHEN** the user explains a ready pod `web` with a deletionTimestamp
- **THEN** the first finding is `Pod/web is Terminating (1/1 ready)` in the warn color

### Requirement: Pod status text

The pod status in Explain SHALL be `Terminating` when deletion started, else the first container waiting reason other than `ContainerCreating`, else the first terminated reason other than `Completed`, else the pod phase, else `Unknown`.

#### Scenario: Crash looping pod

- **WHEN** a running pod's container waits with `CrashLoopBackOff`
- **THEN** the headline reads `Pod/<name> is CrashLoopBackOff (0/1 ready)`

### Requirement: Pod severity

Pod statuses `CrashLoopBackOff`, `ImagePullBackOff`, `ErrImagePull`, `OOMKilled`, `Error`, `Evicted`, `CreateContainerConfigError`, `CreateContainerError` and `InvalidImageName` SHALL be critical, `Completed` and `Succeeded` informational, and every other unhealthy status MUST be a warning.

#### Scenario: Pending pod

- **WHEN** an unready pod's status is `Pending`
- **THEN** its headline uses the warn color

### Requirement: Container problems

Under a `Containers` heading, Explain SHALL list each container problem, or `all containers ready` when there is none. `ContainerCreating` and `PodInitializing` MUST be informational `<c>: <reason>`. Other waiting reasons MUST read `<c>: <reason> — <message>`, except CrashLoopBackOff, which MUST omit the message and, when a last termination is known, read `<c>: CrashLoopBackOff (last: <reason>, exit <code>, <n> restarts)`.

#### Scenario: Crash loop detail

- **WHEN** container `worker` is in CrashLoopBackOff after 7 restarts, last terminated with `Error` exit 1
- **THEN** a critical finding contains `last: Error, exit 1, 7 restarts`

### Requirement: Terminated and unready containers

A container terminated with a reason other than `Completed` SHALL read `<c>: terminated <reason> (exit <code>)`. A running container that is not ready MUST read `<c>: running but not ready (readiness probe failing?)`. A ready container with restarts MUST read `<c>: <n> restarts (last: <reason>, exit <code>)` as a warning.

#### Scenario: Probe failure and OOM history

- **WHEN** container `app` is running but not ready and ready container `cache` restarted twice after `OOMKilled` exit 137
- **THEN** the findings include `app: running but not ready` and a `cache` finding with `OOMKilled` and `exit 137`

### Requirement: Unschedulable pods

A pod whose `PodScheduled` condition is False SHALL get a critical finding `not scheduled: <reason>: <message>`.

#### Scenario: Insufficient CPU

- **WHEN** a pending pod is unschedulable with message `0/3 nodes are available: 3 Insufficient cpu.`
- **THEN** a finding contains `not scheduled` and `Insufficient cpu`

### Requirement: Pod placement

When a pod has `spec.nodeName`, Explain SHALL add a `Placement` heading with `node <name>`.

#### Scenario: Scheduled pod

- **WHEN** the explained pod runs on node `worker-2`
- **THEN** the findings include `node worker-2` under `Placement`

### Requirement: Ready condition for other kinds

For kinds other than workloads and pods, Explain SHALL report the Ready condition: True as `<Kind>/<name> is Ready` (good), False as `is not Ready` (critical), any other status as `readiness is Unknown` (warn), followed by the reason and message. Without a Ready condition it MUST read `<Kind>/<name> exposes no Ready condition to assess`.

#### Scenario: Failed certificate

- **WHEN** a cert-manager Certificate has `Ready=False` with message `order errored`
- **THEN** the first finding contains `is not Ready` in the critical color
- **AND** a finding contains `order errored`

### Requirement: Other conditions for other kinds

For kinds other than workloads and pods, every non-Ready condition that is False or Unknown SHALL produce a warning `<Type>: <detail>`. For Nodes, `MemoryPressure`, `DiskPressure`, `PIDPressure` and `NetworkUnavailable` MUST warn when True or Unknown and MUST NOT warn when False.

#### Scenario: Healthy node

- **WHEN** a Ready node reports all pressure conditions False
- **THEN** the first finding is `Node/worker is Ready` and no pressure finding is shown

#### Scenario: Memory pressure

- **WHEN** a Ready node reports `MemoryPressure=True`
- **THEN** a warning finding starts with `MemoryPressure`

### Requirement: Recent warning events

Under a `Recent evidence` heading, Explain SHALL show at most 6 Warning events, newest first, as `<HH:MM:SS>  <reason>: <message>`. Normal events MUST be omitted, events with equal times MUST keep the API order, and both `v1` and `events.k8s.io/v1` events MUST be supported.

#### Scenario: Only warnings, newest first

- **WHEN** the object has Warning events `BackOff` at 14:03 and `Failed` at 14:05 and a Normal event at 14:06
- **THEN** the evidence lists `Failed: newer` before `BackOff: older`
- **AND** the Normal event is not shown

### Requirement: Explain navigation keys

In Explain, `j`/`down` and `k`/`up` SHALL move the cursor and `g`/`home` and `G`/`end` MUST jump to the first and last finding.

#### Scenario: Move to the last finding

- **WHEN** the user presses `G` in Explain
- **THEN** the last finding is selected

### Requirement: Jump to a blocking pod

`⏎` on a finding with a pod target SHALL open the Pods view in the pod's namespace filtered to `metadata.name=<pod>`. On a finding without a target it MUST warn `no resource to jump to on this line`, and on a non-pod target it MUST warn `can only jump to pods here`.

#### Scenario: Enter on blocker

- **WHEN** the user presses `⏎` on the `Pod/web-b` finding
- **THEN** the Pods view opens in `default` with field selector `metadata.name=web-b`

### Requirement: Evidence events and logs

`E` SHALL open the events of the selected finding's target, and `l` SHALL stream logs of the selected finding's target pod. When the selected finding has no target, both MUST fall back to the object being explained.

#### Scenario: Events for the root object

- **WHEN** the headline finding is selected and the user presses `E`
- **THEN** the events view opens for the explained object

### Requirement: Returning from evidence

After opening events or logs from Explain, `esc` or `q` SHALL return to Explain with its findings, and another `esc` MUST return to the table with the original row selected.

#### Scenario: Two escapes back to the table

- **WHEN** the user opens logs with `l` from Explain and presses `esc` twice
- **THEN** the first `esc` shows Explain and the second shows the table with the same row selected

### Requirement: Pending report survives evidence views

Opening events or logs from Explain SHALL keep a pending evidence request active, and its findings MUST update Explain without leaving the evidence view.

#### Scenario: Findings arrive while viewing events

- **WHEN** the user presses `E` before the findings arrive
- **THEN** the events view stays open when the findings arrive
- **AND** returning with `esc` shows the new findings

### Requirement: Closing Explain

`esc` or `q` in Explain SHALL return to the previous screen, cancel any pending evidence request, and clear its progress message unless a newer status message replaced it. A reply that arrives after closing MUST be ignored.

#### Scenario: Close before findings arrive

- **WHEN** the user presses `q` while evidence is still gathering
- **THEN** the table is shown with no progress message
- **AND** the late findings do not reopen or change anything

### Requirement: Navigation cancels the report

Jumping to a pod with `⏎` or leaving Explain for a palette destination SHALL cancel the pending report, and a late reply MUST NOT change the destination view.

#### Scenario: Palette exit from evidence

- **WHEN** the user opens events from Explain, then runs `:timeline`
- **THEN** the pending explain request is cancelled and the timeline stays open

### Requirement: Latest request wins

Only the most recent Explain request SHALL update the findings. A reply to an earlier request, including one for a previously explained object, MUST be ignored.

#### Scenario: Older object's reply arrives late

- **WHEN** the user explains `web`, closes, explains `zzz`, and the `web` reply arrives after the `zzz` reply
- **THEN** the view keeps the `zzz` title and findings

### Requirement: Manual refresh

`r` in Explain SHALL gather evidence again for the same object while keeping the previous findings visible until the new ones arrive. When automatic refresh is on, `r` MUST restart it with an immediate read.

#### Scenario: Refresh keeps old findings meanwhile

- **WHEN** the user presses `r` in Explain
- **THEN** the previous findings stay on screen until the new report arrives

#### Scenario: Newest error wins

- **WHEN** a refresh succeeds late after a newer refresh failed with `latest access denied`
- **THEN** the view shows only the warning containing `latest access denied`

### Requirement: Automatic refresh

`R` in Explain SHALL toggle automatic refresh, flashing `automatic refresh: on (5s)` or `automatic refresh: off`. When on, Explain MUST read immediately and then 5 seconds after each result. Automatic refresh MUST be off when Explain opens.

#### Scenario: Toggle on and off

- **WHEN** the user presses `R` twice in Explain
- **THEN** automatic refresh turns on and then off

### Requirement: Automatic refresh failures

An automatic refresh that fails to read the object, finds a replaced object, or cannot list evidence SHALL keep the last findings, show the error, and stop automatic refresh.

#### Scenario: Object deleted

- **WHEN** the explained object is deleted while automatic refresh is on
- **THEN** the previous findings stay visible, an error is shown, and refresh stops

### Requirement: Automatic refresh stops in evidence views

Opening events or logs from Explain SHALL stop automatic refresh, and returning to Explain MUST NOT restart it.

#### Scenario: Return after events

- **WHEN** automatic refresh is on and the user presses `E`, then `q`
- **THEN** Explain is shown with automatic refresh off

### Requirement: Refresh keeps the selected finding

On refresh, the selection SHALL follow a finding with the same target even when its position or text changes, and a finding without a target MUST match by identical content. When the selected finding disappears, the selection MUST clear with the warning `selected finding is no longer available; select another finding`.

#### Scenario: Selected pod moves

- **WHEN** the selected `Pod/web-b` finding moves and changes status text after a refresh
- **THEN** `Pod/web-b` stays selected and `⏎` opens it

### Requirement: Lost selection blocks implicit actions

While a refresh has cleared the selection, `⏎`, `E` and `l` SHALL warn `select a finding before opening its resource, events, or logs` instead of falling back to the explained object, and later automatic refreshes MUST NOT pick a new selection.

#### Scenario: Events after the selection vanished

- **WHEN** the selected finding disappeared and the user presses `E`
- **THEN** Explain stays open with the warning
- **AND** after `g` selects a finding, `E` opens events
