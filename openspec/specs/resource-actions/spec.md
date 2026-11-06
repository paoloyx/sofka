# resource-actions Specification

## Purpose

Resource actions are the keys and commands that change cluster objects or open an interactive session in them from the table: delete, scale, restart, set image, edit, shell, attach, debug containers and node debug pods, CronJob trigger and suspend, and node cordon, uncordon and drain. This spec covers how each action picks its targets, the confirmations and warnings it shows, the request it sends, the messages it reports, and how a failed interactive command is shown and recovered from.

## Requirements

### Requirement: Action targets

A bulk action (delete, force-delete, restart, scale, cordon, uncordon, drain, CronJob trigger and suspend) SHALL act on the marked rows that are still in the current rows. When nothing is marked it SHALL act on the selected row. When rows are marked but none of them is in the current rows, the action MUST do nothing rather than fall back to the selected row.

#### Scenario: Marked rows win over the selection

- **WHEN** two deployments are marked and the cursor is on a third
- **AND** the user presses `r`
- **THEN** the confirmation reads `Restart 2 deployments?`

#### Scenario: Stale marks do not fall back

- **WHEN** the only marked key names a row that is no longer listed
- **AND** the user presses `r`
- **THEN** no confirmation opens
- **AND** the view stays in the table

### Requirement: Read-only mode blocks mutating actions

In read-only mode, delete, force-delete, scale, restart, set image, edit, shell, attach, `:debug`, `:debug-clean`, cordon, uncordon, drain and the `t` action menu on non-pod kinds SHALL be refused before any dialog opens, flashing `read-only mode — action disabled`. Read-only mode itself is owned by the safety capability.

#### Scenario: Scale refused in read-only mode

- **WHEN** read-only mode is on
- **AND** the user presses `s` on a deployment
- **THEN** no prompt opens
- **AND** the flash says the action is disabled by read-only mode

### Requirement: Kind-scoped action keys

The attach key SHALL apply only to pods; the cordon, uncordon and drain keys only to nodes; the set-image key only to pods, deployments, statefulsets, daemonsets, replicasets and replicationcontrollers. On any other kind the key SHALL first be offered to user bookmarks, workspaces and plugins bound to it, and only when none takes it SHALL the built-in action run and flash why it does not apply.

#### Scenario: Cordon on a pod

- **WHEN** the pod table is shown and no user binding uses `C`
- **AND** the user presses `C`
- **THEN** the flash reads `cordon/uncordon applies to nodes`

### Requirement: Delete confirmation

`ctrl-d` SHALL open a confirmation before deleting, labelled `Delete <kind> <name> in <ns>?` for one namespaced target (without ` in <ns>` for a cluster-scoped one) or `Delete <N> <plural>?` for several. `ctrl-k` SHALL open the same dialog with the verb `Force delete`. `y`, `Y` or `enter` accept; `esc`, `n`, `N` or `q` cancel. On Helm release views delete is a Helm uninstall instead, owned by the helm capability.

#### Scenario: Bulk delete label

- **WHEN** three pods are marked
- **AND** the user presses `ctrl-d`
- **THEN** the confirmation reads `Delete 3 pods?`

#### Scenario: Cancel keeps the object

- **WHEN** the delete confirmation is open
- **AND** the user presses `n`
- **THEN** the dialog closes and no delete request is sent

### Requirement: Delete force toggle

In a delete confirmation, `f` or `F` SHALL toggle between delete and force-delete and update the label's verb immediately. A force delete MUST send `gracePeriodSeconds=0`.

#### Scenario: Toggle force in the dialog

- **WHEN** the confirmation reads `Delete pod web in default?`
- **AND** the user presses `f`
- **THEN** the label reads `Force delete pod web in default?`

### Requirement: Delete cascade policy

A delete SHALL default to background propagation. In the delete confirmation, `c` or `C` SHALL cycle background, foreground and orphan, and the label MUST show ` (cascade: foreground)` or ` (orphan dependents)` before the `?` for the non-default policies. The delete request MUST carry the chosen `propagationPolicy`.

#### Scenario: Cycle to orphan

- **WHEN** the delete confirmation is open with the default policy
- **AND** the user presses `c` twice
- **THEN** the label ends with ` (orphan dependents)?`
- **AND** accepting sends `propagationPolicy: Orphan`

### Requirement: Managed object delete warning

When a delete target is managed by Flux (toolkit owner labels) or has an owner reference, the delete confirmation SHALL add `⚠ managed by <owner> — recreated on delete`, naming `Flux <Kind>/<name>` or the controller owner `<Kind>/<name>`. For several targets it SHALL read `⚠ <m>/<total> managed (e.g. <owner>) — recreated on delete`.

#### Scenario: Deleting a ReplicaSet-owned pod

- **WHEN** the selected pod is owned by `ReplicaSet/web-7f9`
- **AND** the user presses `ctrl-d`
- **THEN** the confirmation includes `⚠ managed by ReplicaSet/web-7f9 — recreated on delete`

### Requirement: Delete execution and reporting

An accepted delete SHALL send one DELETE per target, show `deleting <name>…` (or `deleting <N> <plural>…`) while running, and flash `deleted <name>` (or `deleted <N> <plural>`) when every request succeeded. Each failed target MUST flash `delete <name> failed: <error>` as an error and suppress the success message. Marks SHALL be cleared and the action recorded in `:journal` as `delete` or `force-delete`.

#### Scenario: One of two deletes fails

- **WHEN** two marked pods are deleted and the second request is forbidden
- **THEN** the flash reports `delete <second> failed: ...` as an error
- **AND** no `deleted 2 pods` message is shown

### Requirement: Cordon and uncordon

`C` on a node SHALL cordon it and `U` SHALL uncordon it, without a confirmation, by sending a merge patch setting `spec.unschedulable` to `true` or `false` for every target node. Progress SHALL read `cordoning <name>…` and success `cordoned <name>` or `cordoned <N> nodes`, with `uncordoning`/`uncordoned` for `U`. A failure MUST flash `cordoning <name> failed: <error>` (or `uncordoning ...`). The action is recorded in `:journal`.

#### Scenario: Cordon two nodes

- **WHEN** two nodes are marked
- **AND** the user presses `C`
- **THEN** both nodes receive `spec.unschedulable: true`
- **AND** the flash reads `cordoned 2 nodes`

### Requirement: Restart workloads

`r` on deployments, statefulsets and daemonsets SHALL confirm before restarting, labelled `Restart <name> in <ns>?` or `Restart <N> <plural>?`, and pass the `restart` guardrail (plain confirmation by default). Accepting SHALL send a strategic merge patch setting the pod template annotation `kubectl.kubernetes.io/restartedAt` to the current time on every target.

#### Scenario: Restart one deployment

- **WHEN** deployment `web` in `default` is selected
- **AND** the user presses `r` then `y`
- **THEN** `web` is patched with a `kubectl.kubernetes.io/restartedAt` template annotation

### Requirement: Restart reporting continues past failures

A restart SHALL patch every confirmed target even when one fails, show `restarting <label>…` while running, and flash `restarted <label>` when all succeed. Failures MUST be reported as one error flash joining `restart <name> in <ns> failed: <error>` entries with `; `. Marks are cleared after an accepted restart and kept when it is cancelled or refused by a guardrail.

#### Scenario: Two of three restarts fail

- **WHEN** three marked deployments are restarted and two patches fail
- **THEN** the third deployment is still patched
- **AND** the error flash names both failed deployments

### Requirement: Scale prompt

`s` on a kind with a scale subresource SHALL open a prompt `Scale <name> to replicas (current <N>):` for a single core or apps deployment, statefulset, replicaset or replicationcontroller, `Scale <name> to replicas:` for other single targets, and `Scale <N> <plural> to replicas:` for several. A kind without a scale subresource MUST flash `resource does not support scale with PATCH`.

#### Scenario: Custom resource without current count

- **WHEN** one custom resource `a` with a scale subresource is selected
- **AND** the user presses `s`
- **THEN** the prompt reads `Scale a to replicas:`

### Requirement: Scale validation and execution

The scale input MUST be a non-negative 32-bit integer; anything else SHALL flash `invalid replica count` and send nothing. A valid value SHALL send a merge patch `{"spec":{"replicas":N}}` to each target's scale subresource without further confirmation, show `scaling <label> → N…`, flash `scaled <label> → N` on success or `scale <name> failed: <error>` on failure, clear marks and record `scale to N` in `:journal`.

#### Scenario: Negative replica count

- **WHEN** the scale prompt is open
- **AND** the user enters `-1`
- **THEN** the flash reads `invalid replica count`
- **AND** no request is sent

### Requirement: Set image picker

`i` SHALL open a picker of the target's container names. Choosing one SHALL open a prompt `New image for <container>:` prefilled with its current image. A target with no containers MUST flash `no containers found`; an unsupported kind MUST flash `set image applies to pods and workload controllers`.

#### Scenario: Prefilled image

- **WHEN** a deployment whose container `app` runs `nginx:1.25` is selected
- **AND** the user presses `i` then `enter` on `app`
- **THEN** the prompt reads `New image for app:` with `nginx:1.25` filled in

### Requirement: Set image execution

Submitting a non-empty image SHALL send a strategic merge patch replacing that container's image under `spec.containers` for a pod or `spec.template.spec.containers` for a controller, with no confirmation. It SHALL show `setting image: <container> → <image>…`, then `image set: <container> → <image>` or `set image failed: <error>`, and record `set-image <container>=<image>` in `:journal`. An empty input MUST flash `no image given`.

#### Scenario: Empty image

- **WHEN** the set image prompt is cleared
- **AND** the user presses `enter`
- **THEN** the flash reads `no image given`

### Requirement: Edit in the editor

`e` SHALL suspend the TUI and run `kubectl edit <resource> <name>` with `-n <ns>` for namespaced objects, spelling grouped kinds as `<plural>.<version>.<group>` and pinning the active context. The edit is recorded in `:journal`.

#### Scenario: Edit a deployment

- **WHEN** deployment `web` in `default` is selected
- **AND** the user presses `e`
- **THEN** sofka runs `kubectl edit deployments.v1.apps web -n default` against the active context

### Requirement: Flux-managed edit warning

Before editing an object carrying Flux toolkit owner labels, `e` SHALL ask `⚠ Managed by Flux <Kind>/<name> — your edit will be reverted on the next reconcile. Edit anyway?` and run the editor only when accepted. Objects that are not Flux-managed MUST open the editor without this warning.

#### Scenario: Unmanaged object

- **WHEN** a ConfigMap with no Flux labels is selected
- **AND** the user presses `e`
- **THEN** the editor opens without a confirmation

### Requirement: Shell into a pod

`s` on a pod SHALL suspend the TUI and run `kubectl exec -it -n <ns> <pod> [-c <container>] -- sh -c "command -v bash >/dev/null 2>&1 && exec bash || exec sh"` against the active context. It passes the `shell` guardrail, which asks `Shell into <name>?` only when a rule requires it. The session is recorded in `:journal`.

#### Scenario: Shell prefers bash

- **WHEN** a pod is selected and no guardrail matches
- **AND** the user presses `s`
- **THEN** the TUI suspends and the exec tries `bash` before falling back to `sh`

### Requirement: Shell container choice

Without an explicitly chosen container, a pod shell SHALL use the container named by the `kubectl.kubernetes.io/default-container` annotation when that container exists, otherwise the first container. Choosing `s` in the containers picker SHALL target the selected container.

#### Scenario: Default container annotation

- **WHEN** a pod annotates `kubectl.kubernetes.io/default-container: app` and has containers `istio-proxy` and `app`
- **AND** the user presses `s` on the pod row
- **THEN** the exec passes `-c app`

### Requirement: Attach to a pod

`a` on a pod SHALL suspend the TUI and run `kubectl attach -it -n <ns> <pod>` against the active context, with no confirmation and no guardrail. On another kind it MUST flash `attach is only available for pods`.

#### Scenario: Attach

- **WHEN** a pod is selected
- **AND** the user presses `a`
- **THEN** the TUI suspends and `kubectl attach -it` runs for that pod

### Requirement: Suspended command result

When an interactive command (shell, attach, edit, debug, plugin) returns successfully, sofka SHALL restore the TUI and flash `Command completed.`. While suspended, SIGINT and SIGQUIT MUST reach the child, not end sofka.

#### Scenario: Shell exits cleanly

- **WHEN** the user exits a pod shell with status 0
- **THEN** the TUI returns and the flash reads `Command completed.`

### Requirement: Command failure dialog

When an interactive command fails, sofka SHALL open a `Command failed` dialog reading `Command failed (<status>).` followed by the last 16 KiB of the command's standard error, and flash `Command failed.`. The dialog SHALL stay until `esc` or `enter` dismisses it, and `PgUp`, `PgDn`, `Up` and `Down` scroll it.

#### Scenario: Failure stays visible

- **WHEN** `kubectl exec` exits non-zero with an error on stderr
- **THEN** the dialog shows the status and the error text
- **AND** it remains open until the user presses `esc` or `enter`

### Requirement: Missing shell recovery

When a failed shell's standard error shows the container has no shell (`exec: "sh"` or `exec: "/bin/sh"` with `executable file not found` or `no such file or directory`), the failure dialog SHALL offer `d Start debug container`. `d` SHALL open the debug image prompt for the same pod and container. Other failures MUST NOT offer `d`.

#### Scenario: Distroless container

- **WHEN** a shell fails with `exec: "sh": executable file not found in $PATH`
- **AND** the user presses `d` in the dialog
- **THEN** the prompt `Debug image for <pod> (--target <container>, ⏎ to accept):` opens

### Requirement: Recovery failures keep the original error

If the recovery debug session fails, the dialog SHALL show the original error followed by `Recovery failed:` and the new error. If read-only mode or a guardrail refuses the recovery, the refusal text MUST be appended to the dialog rather than replacing the original error.

#### Scenario: Recovery refused in read-only mode

- **WHEN** read-only mode is on and the missing-shell dialog is open
- **AND** the user presses `d`
- **THEN** the dialog still shows the original error with the read-only refusal appended

### Requirement: Ephemeral debug container

`:debug` (also `:ephemeral`, `:dbg`) on a pod SHALL open the prompt `Debug image for <pod> (⏎ to accept):`, prefilled with `[debug] image` (default `busybox:latest`); `d` in the containers picker SHALL add `--target <container>`. An empty image MUST flash `no debug image given`. On a kind that is neither pod nor node it MUST flash `debug: select a pod (ephemeral container) or node (debug pod)`.

#### Scenario: Debug from the containers picker

- **WHEN** the containers picker for pod `web` is on container `app`
- **AND** the user presses `d`
- **THEN** the prompt reads `Debug image for web (--target app, ⏎ to accept):`

### Requirement: Debug container launch

Submitting the debug image SHALL pass the `debug` guardrail, which confirms with `Start debug container in <ns>/<pod> with image <image>?` only when a rule requires it, then suspend the TUI and run `kubectl debug -it -n <ns> <pod> --image=<image> [--target=<container>]` followed by `[debug] command` or the bash-or-sh fallback. The launch is recorded in `:journal`.

#### Scenario: Configured debug command

- **WHEN** `[debug] command = ["bash"]` is set
- **AND** the user accepts the debug image prompt
- **THEN** the command ends with `-- bash`

### Requirement: Node debug pod

`:debug` on a node SHALL always confirm with a preview `⚠ Node debug pod on <node> (image <image> in <ns>[, profile <p>]) — grants the host filesystem (/host) and host PID/network/IPC namespaces. Launch?`, using `[debug] node_image` (default `busybox:latest`), `node_namespace` (default `default`) and optional `profile`, under the `node-debug` guardrail. Accepting SHALL run `kubectl debug node/<node> -it -n <ns> --image=<image> [--profile=<p>]`.

#### Scenario: Preview before launch

- **WHEN** a node `worker-1` is selected
- **AND** the user runs `:debug`
- **THEN** the confirmation names the image, the namespace and the host access granted
- **AND** nothing runs until the user accepts

### Requirement: Node debugger cleanup

`:debug-clean` (also `:debug-cleanup`, `:dbgclean`) SHALL confirm `Delete node debugger pod(s) on <nodes> launched this session?` and then delete pods named `node-debugger-*` scheduled on the nodes debugged this session, in the namespace each was launched in. It SHALL flash `removed <N> node debugger pod(s)`, or an error `debug-clean: removed <N>, <M> failed — ...`. With no node debug this session it MUST flash `no node debuggers launched this session`.

#### Scenario: Nothing to clean

- **WHEN** no node debug pod was launched this session
- **AND** the user runs `:debug-clean`
- **THEN** the flash reads `no node debuggers launched this session`

### Requirement: CronJob action menu

`t` on a CronJob SHALL open a menu with `Trigger now`, `Suspend`, `Resume` and `Cancel`, acting on the marked CronJobs or the selected one. `esc`, `q` or `Cancel` close it without changes. The Flux and Argo CD entries of the same menu are owned by the flux and argocd capabilities.

#### Scenario: Trigger is first

- **WHEN** a CronJob is selected
- **AND** the user presses `t`
- **THEN** the menu opens with `Trigger now` selected

### Requirement: CronJob trigger now

`Trigger now` SHALL create a `batch/v1` Job named `<cronjob name, first 42 chars>-manual-<unix seconds in hex>` from the CronJob's `jobTemplate`, copying its labels and annotations, adding `cronjob.kubernetes.io/instantiate: manual` and an owner reference to the CronJob, like `kubectl create job --from`. It SHALL work on suspended CronJobs and report `triggering <label>…` then `triggered <label>` or `trigger <name> failed: <error>`.

#### Scenario: Trigger a suspended CronJob

- **WHEN** CronJob `backup` has `spec.suspend: true`
- **AND** the user chooses `Trigger now`
- **THEN** a Job `backup-manual-<hex>` is created from its template

### Requirement: CronJob suspend and resume

`Suspend` and `Resume` SHALL send a merge patch setting `spec.suspend` to `true` or `false` on every target, reporting `suspending <label>…` then `suspended <label>` (or the resume wording), and record the change in `:journal`.

#### Scenario: Resume marked CronJobs

- **WHEN** two suspended CronJobs are marked
- **AND** the user chooses `Resume`
- **THEN** both receive `spec.suspend: false`

### Requirement: Drain options form

`D` on a node SHALL open a drain form for the marked or selected nodes, sorted by name, with `Ignore DaemonSets` (on), `Force: permit pods with no live controller` (off), `Delete emptyDir data: lose local data` (off), `Disable eviction: bypass PodDisruptionBudgets` (off), `Grace period` (pod default) and `Timeout` (0, unlimited). `Tab`/`Down` and `Shift-Tab`/`Up` move, `Space` toggles, and `Ctrl-U` or `Ctrl-W` clears a duration.

#### Scenario: Defaults

- **WHEN** the user presses `D` on a node
- **THEN** only `Ignore DaemonSets` is checked
- **AND** the timeout reads `0 (unlimited)`

### Requirement: Drain option validation

The grace period MUST be whole seconds of zero or more, or empty for the pod default; otherwise `enter` SHALL show `Grace period must be whole seconds >= 0, or empty for the pod default`. The timeout MUST be `0` or a duration of `s`, `m` and `h` parts such as `1h30m`; otherwise it SHALL show `Timeout must be 0 or a duration such as 30s, 5m, or 1h30m`, or `Timeout is too large`. Duration input is capped at 32 characters.

#### Scenario: Day unit rejected

- **WHEN** the timeout field contains `1d`
- **AND** the user presses `enter`
- **THEN** the timeout error is shown and the form stays open

### Requirement: Drain review and confirmation

`enter` on a valid form SHALL show a review titled `Confirm node drain` listing every target node and option, asking `Confirm drain with these options?`. The `drain` guardrail (plain confirmation by default) is checked when the form opens and again at review; a typed rule SHALL ask `Type '<expected>' to confirm:`. `esc` or `Ctrl-C` in the form or review MUST cancel without quitting sofka.

#### Scenario: Ctrl-C in review

- **WHEN** the drain review is shown
- **AND** the user presses `Ctrl-C`
- **THEN** the review closes and sofka keeps running

### Requirement: Drain execution order

An accepted drain SHALL process nodes one at a time: cordon the node with a merge patch, list its pods by `spec.nodeName` in pages of 500, request removal of each eligible pod, and poll until none remain before moving to the next node. Mirror pods and `Succeeded` or `Failed` pods are not eligible; DaemonSet pods are skipped when `Ignore DaemonSets` is on. Nodes MUST remain cordoned afterwards. The drain is recorded in `:journal`.

#### Scenario: Two nodes in order

- **WHEN** a drain of `a` and `b` is accepted
- **THEN** `b` is cordoned only after every eligible pod on `a` is gone

### Requirement: Drain pod validation

Before removing pods, a drain MUST stop with an error when an eligible pod is a DaemonSet pod while `Ignore DaemonSets` is off, has no UID, uses `emptyDir` without `Delete emptyDir data`, has no controller without `Force`, or names a controller that cannot be resolved, is missing, or has a different UID. These checks SHALL run before any eviction on that node.

#### Scenario: emptyDir pod blocks the drain

- **WHEN** a node runs a controller-managed pod with an `emptyDir` volume and `Delete emptyDir data` is off
- **THEN** the drain stops with `pod has emptyDir data that removal would delete`
- **AND** no pod on that node is evicted

### Requirement: Drain pod removal

A drain SHALL remove each pod with a `policy/v1` Eviction carrying the pod's UID as a precondition and the chosen grace period. With `Disable eviction` it SHALL send a pod DELETE with the same UID precondition instead. It MUST NOT fall back from eviction to deletion. A 404 for that pod counts as already removed.

#### Scenario: Eviction pins the UID

- **WHEN** a drain evicts pod `web-1`
- **THEN** the Eviction request includes `preconditions.uid` equal to the pod's UID

### Requirement: Drain retries and stop on failure

Removal requests failing with 408, 429, 500, 502, 503 or 504, a timeout or a transport error SHALL be retried with a backoff from 1s doubling up to 10s, showing `Retry in <N>s: ...`. Any other error MUST stop the whole drain with `<pod>: pod removal failed` without touching later nodes. Each API call is limited to 30 seconds.

#### Scenario: PDB blocks eviction

- **WHEN** an eviction returns 429
- **THEN** the progress shows a retry countdown
- **AND** the eviction is retried

### Requirement: Drain progress, cancel and result

While draining, only `esc`/`Ctrl-C` (cancel) and `PgUp`/`PgDn` work. Cancel SHALL stop pending requests and end with `canceled; accepted requests cannot be reversed`; an expired timeout ends with `timed out`. The result SHALL read `drained` or `drain stopped: <error>`, then `Completed`, `Incomplete`, `Unstarted` and `Remain cordoned` node lists. `enter` or `esc` closes it.

#### Scenario: Bulk drain stops on the second node

- **WHEN** a drain of `a`, `b` and `c` fails on `b`
- **THEN** the result lists `Completed: a`, `Incomplete: b` and `Unstarted: c`

### Requirement: Uncertain cordon reporting

When a drain stops while the outcome of a cordon request is unknown, the result SHALL add a note telling the user to check the incomplete node.

#### Scenario: Timeout during cordon

- **WHEN** the drain timeout expires while the cordon request for `b` is pending
- **THEN** the result ends with `The last cordon request may have succeeded. Check the incomplete node.`
