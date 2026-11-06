# timeline-notifications Specification

## Purpose

Describe the session-local timeline (`T` / `:timeline`) of state changes sofka observes for an object, and `:notify` watch notifications that report the same changes through the status line, terminal bell, desktop escape sequences or a notifier command configured in `[notify]`.

## Requirements

### Requirement: Opening the timeline

`T` in a resource table and the palette commands `:timeline`, `:tl` and `:history` SHALL open the timeline for the selected row. Without a resource view sofka MUST warn `select a resource first`, and without a selected row it MUST warn `no selection for timeline`.

#### Scenario: Open from the table

- **WHEN** the user selects pod `default/web` and presses `T`
- **THEN** the timeline view opens for `default/web`

#### Scenario: Empty table

- **WHEN** the user presses `T` in a view with no rows
- **THEN** sofka warns `no selection for timeline` and stays in the table

### Requirement: Timeline layout

The timeline SHALL be titled `<namespace>/<name> — timeline  (<n> events · session-local)` and list entries oldest first as `HH:MM:SS  <text>`, with the time in UTC and dimmed. Entries MUST be colored by severity: informational in the normal text color, good green, warn peach, bad red.

#### Scenario: Entry format

- **WHEN** a pod's phase changed from Pending to Running at 08:45:12 UTC
- **THEN** the timeline shows `08:45:12  phase Pending → Running` in green

### Requirement: Empty timeline

When no change has been recorded for the object, the timeline SHALL show `no changes observed yet — the timeline records what happens while sofka watches` and a count of 0 events.

#### Scenario: Untouched object

- **WHEN** the user opens the timeline for an object that has not changed this session
- **THEN** the view shows the `no changes observed yet` message and `(0 events · session-local)`

### Requirement: Timeline navigation

The timeline SHALL open with the newest entry selected. `j`/`down` and `k`/`up` MUST move the cursor, `g`/`home` and `G`/`end` MUST select the first and last entry, and `esc` or `q` MUST return to the previous screen, restoring the table selection.

#### Scenario: Back to the table

- **WHEN** the user presses `esc` in the timeline
- **THEN** the table is shown with the same row selected

### Requirement: Timeline updates live

While the timeline is open, changes that the table watch observes for the object SHALL appear in the view without reopening it.

#### Scenario: Change while viewing

- **WHEN** the timeline for a Deployment is open and its ready replicas drop
- **THEN** a new `ready replicas` entry appears at the bottom

### Requirement: Timeline source and scope

The timeline SHALL record changes from the watch of the currently open resource view only, keyed by resource type, namespace and name. History recorded for a kind MUST stay available after switching to another view and back, and MUST NOT be written to disk.

#### Scenario: History survives a view switch

- **WHEN** a pod change was recorded, the user switches to Services and back to Pods
- **THEN** the pod's timeline still shows the recorded change

### Requirement: Timeline baseline on first sight

The first version of an object seen by the watch SHALL be a silent baseline. sofka MUST record `<Kind> created` (good) only when the object is seen for the first time this session and was created at or after the session started, and a watch relist MUST NOT record a second creation.

#### Scenario: Old object

- **WHEN** the watch first lists a pod created before sofka started
- **THEN** no timeline entry is recorded

#### Scenario: New object seen twice

- **WHEN** a pod created after sofka started is listed, then listed again after a relist
- **THEN** its timeline has exactly one `Pod created` entry

### Requirement: Timeline deletion

When the watch reports an object deleted, the timeline SHALL record `<Kind> deleted` as a bad entry.

#### Scenario: Pod deleted

- **WHEN** pod `web` is deleted while the Pods view is open
- **THEN** its timeline ends with `Pod deleted` in red

### Requirement: Spec generation changes

A change of `metadata.generation` SHALL be recorded as `spec changed: generation A → B` (informational) for every kind.

#### Scenario: Deployment edited

- **WHEN** a Deployment's generation goes from 17 to 18
- **THEN** the timeline records `spec changed: generation 17 → 18`

### Requirement: Pod phase changes

A pod phase change to a non-empty phase SHALL be recorded as `phase <old> → <new>`, with `<none>` for a missing old phase. It MUST be good for `Running` and `Succeeded`, bad for `Failed`, and informational otherwise.

#### Scenario: Pod fails

- **WHEN** a pod's phase goes from Running to Failed
- **THEN** the timeline records `phase Running → Failed` in red

### Requirement: Pod restart changes

An increase in a pod's total restart count SHALL be recorded as the warning `container restart (<old> → <new> total)`. The total MUST include app containers, init containers and native sidecars, and MUST keep completed init container restarts so that finishing initialization does not reset it.

#### Scenario: Init restart during initialization

- **WHEN** an init container's restarts go from 2 to 3 while the pod moves to Running
- **THEN** the timeline records `container restart (2 → 3 total)`
- **AND** a later app container restart records `container restart (3 → 4 total)`

### Requirement: Pod waiting reason changes

A change in a pod's first container waiting reason, ignoring `ContainerCreating` and `PodInitializing`, SHALL be recorded as the bad entry `container waiting: <reason>`. When a waiting reason clears, sofka MUST record the good entry `container no longer waiting`.

#### Scenario: Crash loop starts

- **WHEN** a pod's container starts waiting with `CrashLoopBackOff`
- **THEN** the timeline records `container waiting: CrashLoopBackOff`

### Requirement: Workload readiness changes

For Deployments, StatefulSets, ReplicaSets and DaemonSets, a change in ready replicas SHALL be recorded as `ready replicas A → B (of N)`, as a warning when it drops and good when it rises. DaemonSets MUST use `numberReady` and `desiredNumberScheduled`; other workloads MUST use `readyReplicas` and `replicas`.

#### Scenario: Replicas drop

- **WHEN** a Deployment with 3 replicas goes from 3 ready to 1 ready
- **THEN** the timeline records the warning `ready replicas 3 → 1 (of 3)`

### Requirement: Condition transitions

A status change of a condition present in both versions SHALL be recorded as `<Type>: <old> → <new> (<reason>)`, omitting an empty reason. The entry MUST be good for `True`, bad for `False`, and a warning otherwise. Workloads MUST track `Available` and `Progressing`; pods and all other kinds MUST track `Ready`.

#### Scenario: Certificate expires

- **WHEN** a Certificate's Ready condition goes from True to False with reason `Expired`
- **THEN** the timeline records exactly one entry `Ready: True → False (Expired)` in red

### Requirement: Unchanged versions record nothing

A new object version without any of the tracked changes SHALL NOT add a timeline entry.

#### Scenario: Identical status

- **WHEN** a Deployment update leaves generation, ready replicas and conditions unchanged
- **THEN** its timeline gains no entry

### Requirement: Timeline bounds

The timeline SHALL keep at most 50 entries per object, dropping the oldest, and history for at most 2000 objects. When a new object needs history at capacity, the object whose history changed least recently MUST be evicted.

#### Scenario: Busy object

- **WHEN** a Deployment's generation changes 99 times
- **THEN** its timeline holds the 50 newest entries

#### Scenario: Object eviction

- **WHEN** 2000 objects have history and a new object records a change
- **THEN** the least recently changed object's history is dropped and recently changed history is kept

### Requirement: Timeline cleared on context switch

Switching context SHALL clear all timeline history.

#### Scenario: New cluster

- **WHEN** the user switches to another context
- **THEN** timelines show no entries from the previous cluster

### Requirement: Toggling notifications

The palette commands `:notify` and `:bell` SHALL start a notification watch for the selected row and flash `notify on: <namespace>/<name> — changes flash + bell (<n> active)`. Running it again on the same row MUST stop that watch and flash `notify off: <namespace>/<name> (<n> still active)`.

#### Scenario: Toggle on and off

- **WHEN** the user runs `:notify` twice on pod `default/web`
- **THEN** the first run flashes `notify on` and the second flashes `notify off`

### Requirement: Notification preconditions

`:notify` MUST NOT start in Helm release or Helm history views, warning `notify is not available for Helm views`. Without a resource view it MUST warn `select a resource first`, without a selected row `no selection to notify on`, and for an object without a name `object has no name to watch`.

#### Scenario: Helm view

- **WHEN** the user runs `:notify` in the Helm releases view
- **THEN** sofka warns `notify is not available for Helm views`

### Requirement: Single-object notification watch

Each active notification SHALL watch only its object, by `metadata.name` in the object's namespace, independently of the open view. It MUST keep running across view switches until toggled off, the context changes, or sofka exits, and watch errors MUST NOT produce notifications.

#### Scenario: Notify while browsing

- **WHEN** notify is on for pod `default/web` and the user switches to Services
- **THEN** the notification watch keeps running and later changes to `web` still notify

### Requirement: Notification content

A notification SHALL report the same changes the timeline records, joined into one message per watch event as `<kind>/<name>: <change> · <change>`. The state at watch start MUST NOT notify; an object that appears after the initial sync MUST notify `<kind>/<name>: created`, and a deletion MUST notify `<kind>/<name>: deleted`.

#### Scenario: Several changes at once

- **WHEN** one update to pod `web` changes its phase and its Ready condition
- **THEN** a single notification lists both changes joined by `·`

### Requirement: Notification status line

Each notification SHALL appear in the status line as `🔔 <message>`, styled as information, not as an error.

#### Scenario: Ready flips

- **WHEN** the notification `pod/web: Ready: True → False` arrives
- **THEN** the status line shows `🔔 pod/web: Ready: True → False`

### Requirement: Notification delivery batching

Notifications that arrive together SHALL be delivered once, joined with `·` and shortened to at most 300 characters, and each message MUST be delivered only once.

#### Scenario: Burst of notifications

- **WHEN** 100 notifications arrive in one batch
- **THEN** one delivery is made with at most 300 characters

### Requirement: Bell and desktop escape sequences

Each delivery SHALL write a terminal bell when `[notify] bell` is true (default), then the `[notify] desktop` sequence: `osc777` (default) writes `ESC ]777;notify;sofka;<message> BEL`, `osc9` writes `ESC ]9;sofka: <message> BEL`, `both` writes both, and `off` writes neither. Control characters in the message MUST be removed.

#### Scenario: Default delivery

- **WHEN** a notification `pod/web: Ready` is delivered with default `[notify]` settings
- **THEN** sofka writes a bell followed by `ESC ]777;notify;sofka;pod/web: Ready BEL`

#### Scenario: Injected escape removed

- **WHEN** the message contains `ESC ]0;evil BEL`
- **THEN** the written sequence contains `]0;evil` without the control characters

### Requirement: Unknown desktop protocol

An unknown `[notify] desktop` value SHALL produce the warning `notify: desktop '<value>' is not osc777/osc9/both/off; using osc777` and MUST behave as `osc777`.

#### Scenario: Unsupported value

- **WHEN** `[notify] desktop = "growl"` is configured
- **THEN** sofka warns once and sends OSC 777 notifications

### Requirement: Notifier command

When `[notify] command` is set, each delivery SHALL run that argv directly, without a shell and with no standard input or output. `$MESSAGE` MUST be replaced by the message in every argument after the executable; without a placeholder the message MUST be appended as the last argument. The bell and escape sequences MUST still be written.

#### Scenario: Placeholder argument

- **WHEN** `command = ["notify-send", "sofka", "$MESSAGE"]` and the message is `a; rm -rf /`
- **THEN** sofka runs `notify-send` with the arguments `sofka` and `a; rm -rf /`

#### Scenario: No placeholder

- **WHEN** `command = ["terminal-notifier", "-title", "sofka"]` and the message is `hi`
- **THEN** `hi` is passed as the last argument

### Requirement: Invalid notifier command

A `[notify] command` with an empty executable SHALL produce the warning `notify: command has an empty executable; ignored` and MUST be ignored. A command that fails to start MUST warn `notify command '<exe>' failed: <error>`.

#### Scenario: Empty executable

- **WHEN** `command = [""]` is configured
- **THEN** sofka warns and runs no configured command

### Requirement: herdr auto-detection

When no valid `[notify] command` is set and sofka runs inside a herdr pane, each delivery SHALL run `herdr notification show sofka --body <message>`. A configured command MUST take priority over herdr.

#### Scenario: Inside herdr

- **WHEN** sofka runs in a herdr pane without `[notify] command`
- **THEN** a notification runs `herdr notification show sofka --body <message>`

### Requirement: Notify configuration reload

`:reload` and context switches SHALL apply the current `[notify]` settings, including per-cluster and per-context overrides, to later deliveries.

#### Scenario: Bell disabled by reload

- **WHEN** the user sets `[notify] bell = false` and runs `:reload`
- **THEN** later notifications do not ring the bell

### Requirement: Context switch stops notifications

Completing a context switch SHALL stop every notification watch, drop undelivered notifications, and ignore notifications from the previous context that arrive later. Notifications MUST keep running while the switch is still connecting.

#### Scenario: Late message from the old cluster

- **WHEN** the user switches context and a notification from the old watch arrives afterwards
- **THEN** the status line is unchanged and nothing is delivered
- **AND** `:notify` on a row in the new context starts a new watch
