# argocd Specification

## Purpose

Argo CD controls and inspection in sofka, read entirely from the Argo CD custom resources with no Argo CD API server or `argocd` binary: the `t` suspend/resume/sync menu for Applications and ApplicationSets, and the `:argocd` view explaining an Application's state, managed resources, blockers, causes, owned workloads and destination cluster.

## Requirements

### Requirement: Argo CD action menu opens with t

Pressing `t` on `applications` or `applicationsets` in group `argoproj.io` SHALL open a popup menu titled " ArgoCD: current selection " or " ArgoCD: N marked <plural> ". Applications SHALL get "Suspend", "Resume", "Sync now", "Cancel"; ApplicationSets SHALL get "Suspend", "Resume", "Cancel". Read-only mode SHALL refuse with "read-only mode — action disabled".

#### Scenario: Application menu

- **WHEN** the user presses `t` on an Argo CD Application row
- **THEN** a menu with Suspend, Resume, Sync now and Cancel opens

#### Scenario: ApplicationSet menu has no sync

- **WHEN** the user presses `t` on an ApplicationSet row
- **THEN** the menu lists Suspend, Resume and Cancel only

### Requirement: Argo CD menu requires an explicit choice

The Argo CD menu SHALL act only on Enter over an item; `esc` or `q` SHALL close it without action, and "Cancel" SHALL do nothing. Actions SHALL apply to the marked rows or else the selected row, SHALL clear marks when they start, and SHALL NOT ask for confirmation.

#### Scenario: Suspend marked Applications

- **WHEN** two Applications are marked and the user chooses Suspend
- **THEN** both Applications are patched and the marks are cleared

### Requirement: Suspending an Application stashes its automated policy

Suspend on an Application SHALL GET the live object, then merge-patch `spec.syncPolicy.automated` to null and, when `automated` was present, store its JSON base64-encoded in annotation `sofka.io/argocd-automated`, preserving `prune`, `selfHeal` and `allowEmpty`. Without `automated`, no annotation SHALL be written.

#### Scenario: Suspend an auto-syncing Application

- **WHEN** an Application with `automated: {prune: true, selfHeal: true}` is suspended
- **THEN** `automated` becomes null and `sofka.io/argocd-automated` holds the original block

### Requirement: Resuming an Application restores its automated policy

Resume on an Application SHALL restore `spec.syncPolicy.automated` from `sofka.io/argocd-automated` and remove that annotation. Without the annotation it SHALL set `automated` to `{}`.

#### Scenario: Resume without a stash

- **WHEN** an Application lacking the stash annotation is resumed
- **THEN** `spec.syncPolicy.automated` is set to `{}`

### Requirement: ApplicationSet suspend uses create-only

Suspend on an ApplicationSet SHALL set `spec.syncPolicy.applicationsSync` to `create-only` and store the previous value (default `sync`) base64-encoded in annotation `sofka.io/argocd-applications-sync`. Resume SHALL restore the stashed value and remove the annotation, or set `sync` when no stash exists.

#### Scenario: Suspend an ApplicationSet without a policy

- **WHEN** an ApplicationSet with no `applicationsSync` is suspended
- **THEN** it becomes `create-only` and the stash records `sync`

### Requirement: Sync now patches the operation field

Sync now SHALL merge-patch `{"operation":{"sync":{}}}` onto each target Application, the same request the Argo CD API server makes, leaving the revision to the controller.

#### Scenario: Sync one Application

- **WHEN** the user chooses Sync now on Application `web`
- **THEN** the Application receives the `operation.sync` patch
- **AND** the flash reads "sync requested: web"

### Requirement: Argo CD action messages and failures

Progress SHALL read "suspending <name>…", "resuming <name>…" or "syncing <name>…" ("<verb> N <plural>…" for several); success "suspended", "resumed" or "sync requested: " plus the name or "N <plural>". A failed suspend/resume SHALL flash "<verb> <name> failed: <error>" per target and still try the rest; failed syncs SHALL be joined with "; " into one error. Actions SHALL be journaled as suspend, resume or sync.

#### Scenario: Suspend fails for one target

- **WHEN** reading or patching one of two marked Applications fails
- **THEN** an error flash "suspending <name> failed: <error>" is shown and the other is still patched

### Requirement: AUTO-SYNC column reflects suspension

The Applications table SHALL show an AUTO-SYNC column reading "on" when `spec.syncPolicy.automated` is set, "suspended" when it is absent and the sofka stash annotation exists, and "manual" otherwise. The REVISION column SHALL abbreviate a 40+ character hexadecimal SHA to 7 characters.

#### Scenario: Suspended Application in the table

- **WHEN** an Application was suspended by sofka
- **THEN** its AUTO-SYNC cell reads "suspended"

### Requirement: ApplicationSet table columns

The ApplicationSets table SHALL show NAME, STATUS ("Error" when condition ErrorOccurred is True, else "Synced" when ResourcesUpToDate is True, else "Unknown"), GENERATORS (comma-joined generator types), APPS (count of `status.resources`) and AGE.

#### Scenario: Errored ApplicationSet

- **WHEN** an ApplicationSet has ErrorOccurred=True
- **THEN** its STATUS reads "Error"

### Requirement: Argo CD view opens from the palette

`:argocd` and `:argo` SHALL open the view for the selected row, titled "<name> — Argo CD", with status "Argo CD: <Kind>/<name>…" while loading. It SHALL flash "select a resource first" with no kind, "this cluster has no Argo CD Application CRD" when the CRD is absent, "Argo CD view is not available for Helm releases" on Helm views, and "no selection for the Argo CD view" with no row.

#### Scenario: Open on an Application

- **WHEN** the user selects Application `web` and runs `:argocd`
- **THEN** the view titled "web — Argo CD" opens

### Requirement: Argo CD view keys

The view SHALL support `j`/`k` and arrows to move, `g`/`G` (Home/End) for first and last line, `r` to refresh, `c` to expand children, Enter to jump, and `esc`/`q` to close.

#### Scenario: Close with q

- **WHEN** the view is open and the user presses `q`
- **THEN** the mode it was opened from is shown again

### Requirement: Selection is re-read before reporting

Opening or refreshing the view SHALL GET the selected object again so the report reflects the cluster now. A failed read or a replaced object SHALL produce an "evidence incomplete — <error>" finding.

#### Scenario: Read denied

- **WHEN** the GET of the selected Application is forbidden
- **THEN** the findings start with "evidence incomplete —"

### Requirement: Application headline with health duration

The headline SHALL read "<subject>: <sync> / <health>" (or just one when the other is empty, or "<subject>: no status yet"), where the subject is "Application/<name>" or "<Kind>/<name> is managed by Application/<app>" when opened from a managed object. When `status.health.lastTransitionTime` parses and is not in the future it SHALL append " (since <duration>)".

#### Scenario: Out-of-sync but healthy

- **WHEN** an Application reports OutOfSync and Healthy with no transition time
- **THEN** the headline reads "Application/web: OutOfSync / Healthy"

### Requirement: Application section

Under an "Application" heading the view SHALL show "project <project>", "destination <cluster>/<namespace>" where the cluster is "this cluster", a kubeconfig context name, the raw server or name, or "unknown", "auto-sync: on (prune, selfHeal)", "on (prune)", "on (selfHeal)", "on", "suspended" or "manual", and "revision <rev>" unless several sources carry their own revisions.

#### Scenario: Local destination

- **WHEN** the Application deploys to `https://kubernetes.default.svc` namespace `default`
- **THEN** the view shows "destination this cluster/default"

### Requirement: Source section lists every source

Under a "Source" heading the view SHALL list each source in `spec.sources` order, or `spec.source`, as its repoURL followed by a detail line of "path", "chart", "ref" and "targetRevision" joined with " · ". With several sources each SHALL add "revision <rev>" from the matching `status.sync.revisions` entry. Without a source it SHALL show "no source in spec".

#### Scenario: Multi-source Application

- **WHEN** an Application has a chart source and a values repository source
- **THEN** each source shows its own deployed revision

### Requirement: Managed resources section

A "Managed resources (N)" heading SHALL list up to 50 entries of `status.resources[]` as "<Kind>/<name>: <sync> / <health>", qualifying with the group as "<Kind>.<group>/<name>" when two entries would otherwise read the same. It SHALL show "none reported" when empty and "… and N more" beyond 50; summary rows SHALL NOT be jump targets.

#### Scenario: Managed Service

- **WHEN** the Application lists Service `web` as OutOfSync and Healthy
- **THEN** the view shows "Service/web: OutOfSync / Healthy" under "Managed resources (1)"

### Requirement: Blocking section explains what stops a sync

Under a "Blocking" heading the view SHALL list, in order: "auto-sync suspended, so this Application will not sync itself"; "last sync Failed|Error — <message>"; the most serious condition (ComparisonError, InvalidSpecError, SyncError, UnknownError, then warnings) as "<Type> — <message>"; up to five "<Kind>/<name> is Degraded|Missing" lines; and up to five "<Kind>/<name> is OutOfSync" lines, each list ending "… and N more" when longer.

#### Scenario: Drift is a blocker

- **WHEN** Service `web` is OutOfSync
- **THEN** the Blocking section shows "Service/web is OutOfSync"

### Requirement: Unexplained health is called out

When health is Degraded, Missing or Progressing and no failed sync, condition or unhealthy resource explains it, the Blocking section SHALL add "<health>, but no managed resource reports it" followed by " — <health message>" when present. Drift and suspension SHALL NOT count as an explanation.

#### Scenario: Degraded with only drift

- **WHEN** an Application is Degraded and its only reported problem is an OutOfSync resource
- **THEN** the view shows "Degraded, but no managed resource reports it"

### Requirement: Nothing-blocking summary

When nothing blocks, the Blocking section SHALL show "synced and healthy at <rev>" for Synced and Healthy, "no status reported yet" when both are empty, or "<sync> / <health> at <rev>" otherwise, omitting " at <rev>" when no revision is known.

#### Scenario: Healthy Application

- **WHEN** an Application is Synced and Healthy at a 40-character SHA
- **THEN** the Blocking line reads "synced and healthy at <first 7 characters>"

### Requirement: Cause search for unexplained health

When health is Degraded or Missing and the status names no cause, the destination is the connected cluster, and a managed namespaced resource can own children, the view SHALL search those resources' descendants in each resource's own namespace, showing "Argo CD: looking for the cause…", and insert up to five "<Kind>/<name>: <state>" jumpable rows below the unexplained line for states that are failed, unknown, pending, evicted, OOMKilled, BackOff or error reasons.

#### Scenario: Crash-looping pod found

- **WHEN** a Degraded Application manages a Deployment whose pod is in CrashLoopBackOff
- **THEN** a row "Pod/<name>: CrashLoopBackOff" appears under the unexplained line

#### Scenario: No search when explained

- **WHEN** the status already lists a Degraded managed resource
- **THEN** no cause search is started

#### Scenario: No search in another cluster

- **WHEN** the Application's destination is a remote cluster
- **THEN** no cause search is started

### Requirement: Cursor stays put when rows are inserted

When cause rows or expanded children are inserted above the cursor, the cursor SHALL shift so it stays on the same line.

#### Scenario: Cursor on a drift line

- **WHEN** the cursor is on a drift line and cause rows are inserted above it
- **THEN** the cursor remains on that drift line

### Requirement: Expanding owned workloads with c

`c` on a local managed-resource row SHALL toggle a tree of its descendants, found by `ownerReferences` up to two levels using the adjacent-view child rules including `[views."…"].children`, showing "Argo CD: children of <name>…" while loading. Each child SHALL be a jumpable "<Kind>/<name>: <state>" row; with none it SHALL show "owns nothing". Pressing `c` again SHALL collapse it.

#### Scenario: Deployment tree

- **WHEN** the user presses `c` on a managed Deployment
- **THEN** its ReplicaSet and Pods appear indented underneath with their states
- **AND** pressing `c` again removes them

### Requirement: Descendant states

A descendant's state SHALL be a pod's first container waiting reason, else its phase, else a Job's "Failed", "Complete" or "Completing", else "<ready>/<desired> ready" from `spec.replicas`. ReplicaSets scaled to zero that own no pods SHALL be hidden.

#### Scenario: Superseded ReplicaSet hidden

- **WHEN** a Deployment owns an old ReplicaSet with zero replicas and no pods
- **THEN** that ReplicaSet is not listed

### Requirement: Expansion refusals and errors

`c` on a row without a local target SHALL flash the no-jump reason; on a child row "expand a managed resource, not one of its children"; on a cluster-scoped resource "cluster-scoped resources have no owned workloads to show"; on an unknown kind "this row's kind is not known to the cluster". A failed parent read SHALL show "reading <name>: <error>"; a failed listing SHALL lead with "listing <plural>: <error>". Rows SHALL expand independently.

#### Scenario: Expand on a heading

- **WHEN** the cursor is on the "Managed resources" heading and the user presses `c`
- **THEN** nothing expands

### Requirement: Following tracking metadata to the Application

Opened on a non-Argo object, the view SHALL name the Application from annotation `argocd.argoproj.io/tracking-id` (text before the first `:`), else label `app.kubernetes.io/instance` or `argocd.argoproj.io/instance`, treating "<namespace>_<name>" as namespaced. Without a namespace it SHALL list Applications by `metadata.name` across namespaces and prefer the one listing the object in `status.resources[]`.

#### Scenario: Managed Deployment

- **WHEN** the view opens on a Deployment tracked by Application `web`
- **THEN** the headline reads "Deployment/<name> is managed by Application/web: …"

#### Scenario: Two instances with the same name

- **WHEN** two Applications named `web` exist and only one lists the object
- **THEN** the view reports the one that lists it

### Requirement: Tracking edge cases

A tracking annotation SHALL be trusted even when no same-named Application lists the object. A bare instance label of exactly 63 characters SHALL be treated as a possible truncation and matched by name prefix, scanning up to 20 pages of 500 Applications; exhausting them SHALL report "looking up Application <prefix>…: more than 20 pages" as incomplete evidence.

#### Scenario: Truncated instance label

- **WHEN** a managed object's instance label is the first 63 characters of a longer Application name
- **THEN** the view finds the Application that lists the object

### Requirement: Unmanaged and dangling references

An object with no tracking metadata SHALL report "<Kind>/<name> is not managed by Argo CD" with "no tracking-id annotation or instance label found". An instance label naming no claiming Application SHALL report not managed, noting Helm sets that label too. A tracking annotation whose Application is missing SHALL warn "<Kind>/<name> is tracked by <app>, which was not found" plus the namespace searched.

#### Scenario: Plain Deployment

- **WHEN** the view opens on a Deployment without Argo tracking
- **THEN** it reads "Deployment/web is not managed by Argo CD"

### Requirement: ApplicationSet report

Opened on an ApplicationSet, the view SHALL show "ApplicationSet/<name>: <status>" plus the ErrorOccurred message, a "Generators" heading listing generator types with `matrix`/`merge` unwrapped one level (or "none configured"), and "Applications (N)" listing up to 50 produced Applications from `status.resources[]` as jump targets, or "none generated".

#### Scenario: Git generator

- **WHEN** the view opens on an ApplicationSet with a git generator and one produced Application
- **THEN** it shows "Generators", "git", "Applications (1)" and the Application row
- **AND** Enter on that row opens the applications table filtered to its name

### Requirement: Destination resolution

A `spec.destination.server` SHALL be local when it is `https://kubernetes.default.svc` (optionally `:443`) or the connected server, else resolved to the kubeconfig context with that server. A `spec.destination.name` SHALL match a context of that name, then a context whose cluster entry has that name, then a unique cluster entry ending in `/<name>`; `in-cluster` SHALL be local unless a context has that name. A context serving the connected server SHALL be local.

#### Scenario: EKS ARN cluster entry

- **WHEN** the destination name is `eks-dev-general` and one kubeconfig cluster entry ends in `cluster/eks-dev-general`
- **THEN** the destination resolves to that entry's context

#### Scenario: Ambiguous tail

- **WHEN** two cluster entries in different accounts end in `/eks-dev-general`
- **THEN** the destination stays unresolved

### Requirement: Local jumps from managed rows

Enter on a managed resource in a local destination SHALL open its kind as a root table in its namespace filtered by `metadata.name=<name>`, flashing "Viewing <plural[.group]>/<name> in <context> · esc returns to the Application". `esc` at the root of that table SHALL reopen the Argo CD view on the same row, preserving the mode it was originally opened from. Other jumpable rows SHALL open their target directly.

#### Scenario: Jump and return

- **WHEN** the user presses Enter on "Service/web" and then `esc`
- **THEN** the services table filtered to `web` opens and `esc` reopens the Argo CD view on that row

### Requirement: Remote jumps switch context

When the destination resolves to another kubeconfig context, managed rows SHALL be marked as jumpable and Enter SHALL switch to that context and open the resource there once connected. An unknown kind there SHALL open pods with "<context> has no <Kind>; viewing pods". `esc` at the root SHALL switch back and reopen the view with the original namespace, filter and row; Tab cycling SHALL keep this way back, and opening another root view SHALL drop it.

#### Scenario: Open in the spoke cluster

- **WHEN** the destination resolves to context `west` and the user presses Enter on "Service/web"
- **THEN** sofka switches to `west` and shows services filtered to `web`
- **AND** `esc` switches back and reopens the Argo CD view

### Requirement: Failed remote switch reopens the view

If the context switch for a remote jump fails, sofka SHALL stay on the original cluster, reopen the Argo CD view on the same row and flash "context switch failed: <error>".

#### Scenario: Spoke unreachable

- **WHEN** switching to the destination context times out
- **THEN** the Argo CD view is shown again with the error flash

### Requirement: No-jump explanations

Enter on a row with nowhere to go SHALL flash "these resources live in <context>; ⏎ on a managed resource opens it there" for a remote context destination, "these resources live in <server>, which no kubeconfig context serves" for an unresolved one, and "no resource to jump to on this line" otherwise, without navigating.

#### Scenario: Unresolved remote server

- **WHEN** the destination is `https://other.example:6443` with no matching context and the user presses Enter on a managed row
- **THEN** the view stays open and the flash contains "no kubeconfig context serves"

### Requirement: Refresh and cancellation

`r` SHALL re-read the selection, follow its tracking metadata again and collapse all expansions; only the latest request SHALL update the view. On arrival the cursor SHALL land on the restored row or the first jumpable finding. Leaving with `esc`, `q` or a jump SHALL cancel pending reports and cause searches, and clear their status.

#### Scenario: Esc cancels a pending report

- **WHEN** the user opens `:argocd` and presses `esc` before the report arrives
- **THEN** the late report is ignored and the view stays empty
