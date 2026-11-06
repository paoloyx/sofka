# cluster-overviews Specification

## Purpose

Cluster-wide and relationship overviews: the `:pulse` health dashboard, the `:xray` ownership tree, the `:fleet` multi-context summary, the adjacent view (`u` / `:adjacent`) of connected objects, and the RBAC subject and rule views (`:users`, `:groups`, `:policy`).

## Requirements

### Requirement: Pulse dashboard command

The `:pulse` command (aliases `:dashboard`, `:pu`) SHALL open a dashboard that summarizes cluster health across all namespaces, whatever namespace the table is showing. It MUST stop the table's live watches while open and re-gather its data every 5 seconds.

#### Scenario: Open pulse

- **WHEN** the user runs `:pulse` while viewing pods in one namespace
- **THEN** the pulse dashboard opens
- **AND** its counts cover nodes, pods, deployments, statefulsets, daemonsets, jobs and PVCs in all namespaces

### Requirement: Pulse tiles

Pulse SHALL show six tiles: "Nodes Ready" (nodes whose Ready condition is True over total), "Pods" (Running, Pending, Failed, Succeeded and Total counts by phase), "Deployments", "StatefulSets", "DaemonSets", and "Storage / Batch" showing "PVCs Bound b/t" (claims in phase Bound) and "Jobs N" (total jobs).

#### Scenario: Pod phase counts

- **WHEN** the cluster has 3 Running pods, 1 Pending pod and 1 Failed pod
- **THEN** the Pods tile shows Running 3, Pending 1, Failed 1 and Total 5

### Requirement: Pulse workload readiness

Pulse SHALL count a Deployment or StatefulSet as ready only when its ready replicas are at least its desired replicas and the desired count is above zero, and a DaemonSet as ready only when its ready count is at least its desired scheduled count and that count is above zero. A workload scaled to zero MUST NOT count as ready.

#### Scenario: Scaled-to-zero deployment

- **WHEN** a Deployment has `spec.replicas: 0`
- **THEN** pulse counts it in the Deployments total
- **AND** does not count it as ready

### Requirement: Pulse gauge colours

Each readiness gauge SHALL be labelled "ready/total" and coloured grey and drawn full when the total is 0, green when all are ready, yellow when at least half are ready, and red otherwise.

#### Scenario: Half ready

- **WHEN** 2 of 4 deployments are ready
- **THEN** the Deployments gauge reads "2/4" in yellow

### Requirement: Pulse incomplete data

When any list request for pulse fails or is denied, pulse MUST NOT present the missing data as zero silently; it SHALL flash the warning "pulse is incomplete — <first error>", where the error names the list, such as "listing pods: …". A later poll SHALL warn again even after an earlier successful gather.

#### Scenario: Pods list forbidden

- **WHEN** listing pods returns 403 during a pulse gather
- **THEN** the status shows a warning starting with "pulse is incomplete — listing pods:"

### Requirement: Pulse keys

In pulse, `r` SHALL re-gather immediately, and Esc or `q` SHALL return to the resource table and restart its live watch.

#### Scenario: Leave pulse

- **WHEN** the user presses Esc in pulse
- **THEN** the resource table is shown again with its watch running

### Requirement: Xray ownership tree

The `:xray` command (alias `:x`) SHALL show the current resource kind across the watched namespaces as root rows, each followed by its descendants found by ownerReference UID, indented 2 spaces per depth. Pods SHALL expand into their containers as leaf rows. With no resource kind selected it MUST warn "select a resource first".

#### Scenario: Deployment tree

- **WHEN** the user runs `:xray` on deployments
- **THEN** each deployment is followed by its ReplicaSets, their Pods, and each pod's containers

#### Scenario: No kind

- **WHEN** no resource kind is being viewed and the user runs `:xray`
- **THEN** the warning "select a resource first" is shown

### Requirement: Xray descendant kinds

Xray SHALL look for descendants by root kind: none for pods; jobs and pods for cronjobs; pods for jobs, daemonsets, replicasets and statefulsets; and replicasets and pods for deployments and any other kind.

#### Scenario: CronJob tree

- **WHEN** xray roots are CronJobs
- **THEN** each CronJob lists the Jobs it owns and the Pods those Jobs own

### Requirement: Xray row glyphs and status

Xray rows SHALL use ◈ for deployments, statefulsets and daemonsets, ◇ for replicasets, ● for pods, ▪ for containers and ◆ for other kinds. The status text SHALL be the phase for pods, "succeeded/completions" for jobs (completions default 1), "active N" for cronjobs, "ready/replicas" for deployments, replicasets and statefulsets (replicas default 1), and "ready/desired" for daemonsets.

#### Scenario: Job status

- **WHEN** a Job has `spec.completions: 3` and `status.succeeded: 2`
- **THEN** its xray row shows "2/3"

### Requirement: Xray refresh and title

The xray view SHALL be titled "Xray [N]" with N rows, re-gather every 5 seconds and on `r`, and keep the cursor within the rows. A failed list SHALL flash "xray is incomplete — <error>". `j`/`k` move, `g`/`G` jump to first/last, and Esc or `q` return to the table and restart its watch.

#### Scenario: Refresh keeps cursor in range

- **WHEN** the cursor is on the tenth xray row and a refresh returns 4 rows
- **THEN** the cursor moves to the fourth row

### Requirement: Xray logs

In xray, Enter or `l` SHALL open logs for the selected row: a container row opens that container's logs titled "<pod>:<container> — logs", a pod row opens the pod's logs titled "<pod> — logs", and any other row MUST warn "logs available on pods/containers". Log viewing itself belongs to the logs capability.

#### Scenario: Logs on a deployment row

- **WHEN** a deployment row is selected in xray and the user presses `l`
- **THEN** the warning "logs available on pods/containers" is shown

### Requirement: Fleet context list

The `:fleet` command (aliases `:clusters`, `:multi`) SHALL summarize the contexts in `[fleet] contexts`, minus those marked removed and plus those marked added from the context switcher, in marking order. With no contexts it MUST warn "no fleet contexts — set [fleet] contexts = ["ctx-a", …] or mark them with space in :ctx" and not open.

#### Scenario: Nothing configured

- **WHEN** `[fleet] contexts` is empty and no context is marked
- **AND** the user runs `:fleet`
- **THEN** the "no fleet contexts" warning is shown and the view does not change

### Requirement: Fleet marks from the context switcher

In the `:ctx` switcher, space SHALL toggle the selected context in or out of the fleet, show ✓ on members, flash "fleet + <ctx>" or "fleet − <ctx>", and keep the switcher open. Marks MUST persist in `<state-dir>/fleet.toml` (keys `added` and `removed`), written atomically; the config file MUST NOT be rewritten. A failed save SHALL warn "fleet mark not saved: <error>".

#### Scenario: Unmark a configured context

- **WHEN** `prod` is in `[fleet] contexts` and the user presses space on it in `:ctx`
- **THEN** the flash reads "fleet − prod"
- **AND** `fleet.toml` lists `prod` under `removed`
- **AND** a later `:fleet` omits `prod`

#### Scenario: Corrupt marks file

- **WHEN** `fleet.toml` is missing or unparsable
- **THEN** fleet uses only the configured contexts

### Requirement: Fleet opening

Opening fleet SHALL stop the table's watches, seed one "connecting…" row per context, flash "fleet — N contexts", and gather every context in the background, at most 4 at a time with an 8 second timeout each. Each row's read-only flag SHALL come from the `--readonly`/`--write` override when given, else from that context's resolved config.

#### Scenario: Timeout

- **WHEN** a context does not answer within 8 seconds
- **THEN** its row shows "error: timed out"

### Requirement: Fleet row contents

Each healthy fleet row SHALL show the server version (`?` if unknown), "nodes r/t" (red unless all ready), "pods N✗/T" counting pods that are terminating or neither Succeeded nor Running-and-Ready, a Flux cell, an Argo cell, and "read-only" (yellow) or "write". A connect or list failure SHALL turn the row into an error showing the first line of the error, shortened to 60 characters.

#### Scenario: Unhealthy pods

- **WHEN** a context has 10 pods of which one is Pending and one is Running but not Ready
- **THEN** its row shows "pods 2✗/10"

### Requirement: Fleet Flux and Argo cells

The Flux cell SHALL read "flux ok", "flux N✗" counting Kustomizations and HelmReleases with Ready=False, or "flux —" when the Flux CRDs are absent. The Argo cell SHALL read "argo ok", "argo N✗" counting Applications that are OutOfSync or whose health is Degraded, Missing or Unknown, or "argo —" when Applications cannot be read. Progressing, Suspended or empty status MUST NOT count.

#### Scenario: Suspended application

- **WHEN** an Application is Synced with health Suspended
- **THEN** it does not add to the argo count

### Requirement: Fleet row glyph and title

Fleet rows SHALL show ◐ while connecting, a red ● on error, a green ● when nodes are all ready and no pods, Flux or Argo objects are unhealthy, and a yellow ● otherwise. The view SHALL be titled "Fleet [N]".

#### Scenario: Degraded context

- **WHEN** a context reports "flux 1✗" and everything else is healthy
- **THEN** its row glyph is a yellow ●

### Requirement: Fleet keys

In fleet, `j`/`k` SHALL move, `r` SHALL reset all rows to connecting and gather again, Enter SHALL switch to the selected context, and Esc or `q` SHALL return to the table. When the selected row's Argo count is above zero, Enter SHALL instead switch to that context and open `applications` in all namespaces filtered by "sync=OutOfSync||health=Degraded||health=Missing||health=Unknown".

#### Scenario: Jump to degraded applications

- **WHEN** the selected fleet row shows "argo 2✗" and the user presses Enter
- **THEN** sofka switches to that context and shows Applications in all namespaces with the degraded-Applications filter

### Requirement: Adjacent view entry

`u` in the table or `:adjacent` (aliases `:adj`, `:related`) SHALL open the adjacent view for the selected object, titled "<name> — adjacent [N]", with status "gathering connections for <name>..." while it works. It MUST refuse with "select a resource first" without a kind, "adjacent is not available for Helm releases" on Helm views, "adjacent is not available for namespaces — enter re-scopes to one" on namespaces, and "no selection" with no row.

#### Scenario: Namespaces refused

- **WHEN** the user presses `u` on a namespace row
- **THEN** the warning "adjacent is not available for namespaces — enter re-scopes to one" is shown

### Requirement: Adjacent source read

The adjacent view SHALL first read the current source object from the API server. If that read fails, the view SHALL show the error as a warning with an empty list.

#### Scenario: Source deleted

- **WHEN** the selected object was deleted before `u` is pressed
- **THEN** the adjacent view shows an incomplete warning and no rows

### Requirement: Adjacent owners and children

The adjacent view SHALL list each ownerReference target as "↑ owned by", read with a GET in its API group, and each object in the source namespace whose ownerReference UID matches the source as "↓ owns", searching the built-in child kinds plus any `[views."<key>"].children` for the kind. An owner kind the cluster does not serve SHALL warn "owner kind <Kind> (<apiVersion>) is not served here".

#### Scenario: ReplicaSet connections

- **WHEN** the user opens adjacent on a ReplicaSet owned by a Deployment
- **THEN** the view shows "↑ owned by" the Deployment and "↓ owns" each of its Pods

### Requirement: Adjacent built-in child kinds

Built-in children SHALL be: deployments→replicasets; replicasets→pods; statefulsets and daemonsets→pods and controllerrevisions; jobs→pods; cronjobs→jobs; services→endpointslices; Karpenter nodepools→nodeclaims; Karpenter nodeclaims→nodes.

#### Scenario: Service children

- **WHEN** the user opens adjacent on a Service
- **THEN** EndpointSlices owned by the Service appear as "↓ owns"

### Requirement: Adjacent forward references

The adjacent view SHALL GET every object the source names through a built-in or configured reference and list it with "→" and the reference's relation. A referenced object that returns 404 MUST be skipped without a warning; other errors SHALL become warnings.

#### Scenario: Missing ConfigMap

- **WHEN** a pod mounts ConfigMap `gone` that does not exist
- **THEN** no row for `gone` is shown and the view is not marked incomplete

### Requirement: Adjacent reverse references

The adjacent view SHALL list objects whose references name the source with "←" and the relation. Each rule's source kind SHALL be listed in its reverse scope: `namespace` (default) lists the source's namespace, or the table namespace for a cluster-scoped source, expanding namespace patterns to every watched namespace; `cluster` lists across the cluster; `none` skips the lookup. Rules keyed `kind@ns` only search that namespace.

#### Scenario: Pods mounting a PVC

- **WHEN** the user opens adjacent on a PersistentVolumeClaim
- **THEN** pods in its namespace that mount the claim appear as "← mounts"

### Requirement: Adjacent built-in pod references

Pods SHALL reference their node ("runs on"), service account ("runs as"), priority class ("prioritized by", no reverse lookup), PVC volumes, ConfigMap and Secret volumes and projected sources ("mounts"), `envFrom` and `env.valueFrom` ConfigMap/Secret refs ("reads"), and image pull secrets ("pulls with").

#### Scenario: Pod adjacent

- **WHEN** the user opens adjacent on a pod with `serviceAccountName: api` scheduled on `node-1`
- **THEN** the view shows "→ runs on" Node `node-1` and "→ runs as" ServiceAccount `api`

### Requirement: Adjacent built-in storage, ingress and Karpenter references

Built-in references SHALL include: PVC storage class ("provisioned by"), volume attributes class ("attributes from") and bound PV ("bound to", no reverse); PV claimRef ("bound to", no reverse) and storage class ("provisioned by", cluster reverse); ServiceAccount token secrets ("tokens in", no reverse); Ingress backend and default backend services ("routes to"), TLS secrets ("tls from") and ingress class ("class", no reverse); Karpenter NodeClaim status node ("became", cluster reverse).

#### Scenario: Ingress adjacent

- **WHEN** the user opens adjacent on an Ingress routing to Service `web` with TLS secret `web-tls`
- **THEN** the view shows "→ routes to" Service `web` and "→ tls from" Secret `web-tls`

### Requirement: Configured references

`[[views."<key>".refs]]` entries SHALL add references for the kind: `path` (JSON pointer, `*` fans out over arrays), `kind`, `relation` (default "references"), `reverse` (`namespace` default, `cluster`, `none`), optional `namespace_path`, `kind_path` with allowed `kinds`, and `group_path` or `group`. Refs and children from every view key matching the kind SHALL apply.

#### Scenario: Custom resource ref

- **WHEN** a view declares a ref with `path = "/spec/secretRef/name"` and `kind = "secrets"`
- **AND** the user opens adjacent on such an object
- **THEN** the named Secret appears as "→ references"

### Requirement: Configured reference warnings

A configured ref MUST warn instead of guessing when a cluster-scoped source names a namespaced kind without `namespace_path` ("ref <src> → <kind>: a namespaced kind named from a cluster-scoped one needs namespace_path"), when a kind read at `kind_path` is not in `kinds` ("kind <K> is not one of kinds"), or when no namespace is found at `namespace_path` ("<name> has no namespace at <namespace_path>").

#### Scenario: Kind outside the allowed list

- **WHEN** an object's `kind_path` value is `Pod` and `kinds` only allows `ConfigMap`
- **THEN** no row is added and the view warns that kind Pod is not one of kinds

### Requirement: Adjacent rows and messages

Rows SHALL read "<arrow> <relation> <Kind> <ns>/<name>" in aligned columns and be unique by direction, relation, kind, namespace and name. An empty view SHALL say "gathering…" while working, "nothing connected to this object was found" when complete, or "no results found; lookup is incomplete" after errors. Errors SHALL show the banner "adjacent: incomplete (<warning>)" and flash "adjacent is incomplete — <warning>".

#### Scenario: Partial failure

- **WHEN** listing pods for a reverse lookup is forbidden
- **THEN** the other rows are still shown
- **AND** the banner "adjacent: incomplete (…)" names the failure

### Requirement: Adjacent keys

In the adjacent view, Enter SHALL open the selected object in its own resource view, filtered to its name and keeping its API group, or warn "no object selected"; `y` and `d` SHALL show its YAML and describe, returning to the adjacent view on Esc; `r` SHALL read the source and gather again; Esc or `q` SHALL return to the originating view with its selection restored and cancel any gather.

#### Scenario: Jump to a connected object

- **WHEN** the user selects "→ runs on Node node-1" and presses Enter
- **THEN** the nodes view opens filtered to `node-1`

### Requirement: Discover children of custom resources

In the adjacent view of a namespaced custom resource with a UID, `c` SHALL search every listable, namespaced kind from API discovery in the source namespace for objects owned by the source, adding them as "↓ owns" rows as pages arrive, and the title SHALL hint "(c discover children)". Other sources MUST warn "child discovery requires a namespaced custom resource with a UID"; during the first gather it MUST warn "wait for the initial adjacent lookup to finish".

#### Scenario: Built-in source

- **WHEN** the user presses `c` in the adjacent view of a Deployment
- **THEN** the warning "child discovery requires a namespaced custom resource with a UID" is shown

### Requirement: Child discovery limits and status

Child discovery SHALL run at most 4 requests at a time, 200 objects per page, 200 requests, 20,000 objects, 5 seconds per request and 30 seconds total. The status SHALL read "children: searching (s/t kinds, N objects checked)", then "children: complete (…)" or "children: incomplete (s/t kinds; F errors; <reason>)", keeping rows found before a failure. Leaving, refreshing, opening an overlay or switching context SHALL cancel it ("children: incomplete (search cancelled)").

#### Scenario: Request limit

- **WHEN** child discovery reaches 200 requests
- **THEN** the status reads "children: incomplete (…; request limit reached)"
- **AND** children found so far stay listed

### Requirement: RBAC subject lists

`:users` and `:groups` SHALL list every User or Group subject in all RoleBindings and ClusterRoleBindings (read 500 per page), titled "RBAC users (all namespaces)" or "RBAC groups (all namespaces)", sorted, as "User <name> N binding(s)", counting each binding once per subject. While reading it SHALL show "Reading RBAC data...". With no subjects it SHALL show "No subjects found in the data read."

#### Scenario: Subject counted once per binding

- **WHEN** one RoleBinding lists user `alice` twice and a ClusterRoleBinding lists her once
- **THEN** `:users` shows "User alice" with "2 binding(s)"

### Requirement: RBAC incomplete reads

When RoleBindings or ClusterRoleBindings cannot be read, the RBAC views SHALL still show what was read with a yellow banner "INCOMPLETE: RoleBindings could not be read: <error>" or "INCOMPLETE: ClusterRoleBindings could not be read: <error>".

#### Scenario: Cluster bindings forbidden

- **WHEN** listing ClusterRoleBindings is forbidden
- **THEN** `:users` lists subjects from RoleBindings
- **AND** shows "INCOMPLETE: ClusterRoleBindings could not be read: …"

### Requirement: Directly bound rules for a subject

Enter on a subject SHALL open "Directly bound RBAC rules: <Kind> <name>", starting with "Direct subject matches only. Group membership is not resolved." and "These are stored RBAC rules, not an API access check.", then each binding naming the subject: "Binding: …", "Role: …" and "Scope: namespace <ns> (namespaced resources only)" or "Scope: cluster and all namespaces", followed by its rules. With none it SHALL say "No direct bindings found in the data read."

#### Scenario: Namespaced grant

- **WHEN** user `alice` is bound by RoleBinding `team/readers` to ClusterRole `view`
- **THEN** her rules show "Binding: RoleBinding team/readers", "Role: ClusterRole view" and "Scope: namespace team (namespaced resources only)"

### Requirement: RBAC rule rendering

Each rule SHALL be shown as "Rule N" with "Verbs:", "API groups:" (the empty group shown as `core`), "Resources:" including subresources such as `pods/log`, "Resource names (restricted):" and "Non-resource URLs:" lines; non-resource URLs from a namespaced binding SHALL add "Not granted by this namespaced binding.". A role without rules SHALL show "No rules in this role.".

#### Scenario: Restricted resource names

- **WHEN** a rule allows `get` on `secrets` with `resourceNames: [db]`
- **THEN** it shows "Resource names (restricted): db"

### Requirement: RBAC role reads

Each referenced Role or ClusterRole SHALL be read once, up to 8 at a time. A role that cannot be read SHALL show "Rules unavailable." under its grant and add a warning such as "Role a/reader could not be read: <error>", listed after "INCOMPLETE: " at the top of the document.

#### Scenario: Role forbidden

- **WHEN** reading Role `a/reader` is forbidden
- **THEN** its grant shows "Rules unavailable."
- **AND** the document starts with an INCOMPLETE warning naming `a/reader`

### Requirement: Policy command

`:policy u:<name>`, `:policy g:<name>` or `:policy s:<namespace>/<name>` SHALL open the directly bound rules of that user, group or service account; a bare `:policy` SHALL use the selected core ServiceAccount. Other input MUST warn "usage: :policy u:<name> | g:<name> | s:<namespace>/<name>; bare :policy needs a service account". Service account subjects without a namespace SHALL match the RoleBinding's namespace.

#### Scenario: Service account policy

- **WHEN** the user runs `:policy s:apps/builder`
- **THEN** the view shows rules bound to ServiceAccount `apps/builder`

### Requirement: RBAC object rules

Enter on a Role, ClusterRole, RoleBinding or ClusterRoleBinding of the `rbac.authorization.k8s.io` group SHALL open "RBAC rules: <Kind> <ns>/<name>". Roles SHALL begin "Role definition. Access depends on its bindings." followed by their rules; bindings SHALL show the grant of their roleRef. Same-named kinds in other API groups SHALL keep their normal Enter behaviour.

#### Scenario: Open a binding

- **WHEN** the user presses Enter on ClusterRoleBinding `admins`
- **THEN** "RBAC rules: ClusterRoleBinding admins" shows the bound role's rules with "Scope: cluster and all namespaces"

### Requirement: RBAC view keys

In RBAC views `j`/`k` SHALL move or scroll, PgUp/PgDn step 10, `g`/`G` jump to first/last, and `r` SHALL reload, keeping the selected subject if it still exists and clearing the selection otherwise. Esc SHALL go back from rules to the subject list and then to the previous view. Opening and cancelling the `:` palette SHALL keep the RBAC view. Leaving SHALL cancel pending reads, and late replies MUST be ignored.

#### Scenario: Back from rules

- **WHEN** the user opened `:users`, pressed Enter on a subject, then presses Esc
- **THEN** the subject list is shown again with that subject selected
