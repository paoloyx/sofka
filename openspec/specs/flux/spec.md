# flux Specification

## Purpose

Flux CD controls and inspection in sofka: the `t` suspend/resume/reconcile menu for Flux custom resources, implemented as native Kubernetes API patches with no `flux` binary, and the `:gitops` view that explains the Flux ownership and reconciliation chain of the selected resource.

## Requirements

### Requirement: Flux action menu opens with t

In the table, pressing `t` on a Flux-suspendable kind SHALL open a popup action menu for the marked rows or, with none marked, the selected row. The menu SHALL list "Suspend", "Resume", "Reconcile now" and "Cancel", with the cursor on the first item. Nothing SHALL happen when there are no marked rows and no selected row.

#### Scenario: Open the menu on a Kustomization

- **WHEN** the user views `kustomizations` with a row selected
- **AND** presses `t`
- **THEN** a menu titled " Flux: current selection " opens
- **AND** it lists Suspend, Resume, Reconcile now and Cancel with Suspend highlighted

### Requirement: Flux-suspendable kinds are matched by group

The Flux menu SHALL be offered only when both the API group and plural match one of: `kustomize.toolkit.fluxcd.io` kustomizations; `helm.toolkit.fluxcd.io` helmreleases; `source.toolkit.fluxcd.io` gitrepositories, helmrepositories, helmcharts, ocirepositories, buckets; `image.toolkit.fluxcd.io` imagerepositories, imageupdateautomations; `notification.toolkit.fluxcd.io` alerts, receivers. A same-named plural in another group SHALL NOT qualify.

#### Scenario: Same plural in a foreign group is rejected

- **WHEN** the user presses `t` on a `kustomizations` kind whose group is not `kustomize.toolkit.fluxcd.io`
- **THEN** no Flux menu opens
- **AND** the unsupported-kind error is flashed

### Requirement: Unsupported kinds are refused

Pressing `t` on a kind that is not Flux-suspendable, not a CronJob, not an Argo CD Application or ApplicationSet, and not pods SHALL flash the error "suspend/resume only applies to CronJobs, Flux resources (ks/hr/HelmCharts/git-, helm-, oci-repos, buckets, image automation, alerts, receivers), and ArgoCD Applications/ApplicationSets" and SHALL stay in the table.

#### Scenario: t on Deployments

- **WHEN** the user presses `t` on a Deployment row
- **THEN** the unsupported-kind error is shown as an error flash
- **AND** the view remains the table

### Requirement: Read-only mode disables the Flux menu

In read-only mode, `t` on a Flux kind SHALL flash the error "read-only mode — action disabled", SHALL NOT open the menu, and SHALL keep existing marks.

#### Scenario: Read-only session

- **WHEN** sofka runs read-only and the user presses `t` on a HelmRelease
- **THEN** the read-only error is flashed
- **AND** no menu opens and no patch is sent

### Requirement: HelmRelease menu offers Force reconcile

For `helmreleases` in group `helm.toolkit.fluxcd.io` the menu SHALL list "Suspend", "Resume", "Reconcile now", "Force reconcile" and "Cancel". Other Flux kinds SHALL NOT offer Force reconcile.

#### Scenario: HelmRelease menu items

- **WHEN** the user presses `t` on a Flux HelmRelease
- **THEN** the menu shows five items with "Force reconcile" just before "Cancel"

### Requirement: Menu title names the target set

The menu title SHALL read " Flux: current selection " when at most one row is marked and " Flux: N marked <plural> " when N (two or more) rows are marked.

#### Scenario: Marked rows in the title

- **WHEN** three Kustomizations are marked and the user presses `t`
- **THEN** the menu title reads " Flux: 3 marked kustomizations "

### Requirement: Menu requires an explicit choice

The menu SHALL be navigated with `j`/`k`, arrow keys and PgUp/PgDn, and SHALL act only when Enter is pressed on an item. Enter SHALL return to the table before acting. `esc` or `q` SHALL close the menu with no action. Enter on "Cancel" SHALL close the menu without acting and without changing the flash message.

#### Scenario: Cancel item does nothing

- **WHEN** the menu is open and the user moves to "Cancel" and presses Enter
- **THEN** the view returns to the table
- **AND** no request is sent and the flash is unchanged

#### Scenario: Escape closes without action

- **WHEN** the menu is open and the user presses `esc`
- **THEN** the menu closes and nothing is patched

### Requirement: Bulk targets come from marks

A Flux action SHALL apply to every marked row still present in the table, or to the selected row when nothing is marked. Marks SHALL be cleared when the action starts. No confirmation prompt SHALL be shown.

#### Scenario: Suspend marked rows

- **WHEN** two Kustomizations are marked and the user chooses Suspend
- **THEN** both objects are patched
- **AND** the marks are cleared

### Requirement: Suspend and resume patch spec.suspend

Suspend SHALL send a JSON merge patch (`application/merge-patch+json`) to each target setting `{"spec":{"suspend":true}}`; Resume SHALL set `{"spec":{"suspend":false}}`. Requests SHALL go to the target's own group, version, namespace and plural.

#### Scenario: Suspend a GitRepository

- **WHEN** the user chooses Suspend on GitRepository `podinfo` in `flux-system`
- **THEN** a PATCH to `/apis/source.toolkit.fluxcd.io/<version>/namespaces/flux-system/gitrepositories/podinfo` carries `{"spec":{"suspend":true}}`

### Requirement: Reconcile now requests a reconcile annotation

Reconcile now SHALL merge-patch only the annotation `reconcile.fluxcd.io/requestedAt` with the current time in RFC 3339 form, and SHALL NOT change `spec`.

#### Scenario: Reconcile a Kustomization

- **WHEN** the user chooses Reconcile now on Kustomization `apps`
- **THEN** the patch body is `{"metadata":{"annotations":{"reconcile.fluxcd.io/requestedAt":"<now>"}}}`
- **AND** it contains no `spec` field

### Requirement: Force reconcile sets both annotations

Force reconcile on a HelmRelease SHALL merge-patch `reconcile.fluxcd.io/requestedAt` and `reconcile.fluxcd.io/forceAt` to the same new RFC 3339 timestamp, requesting a Helm install or upgrade even when the spec is unchanged.

#### Scenario: Force reconcile a HelmRelease

- **WHEN** the user chooses Force reconcile on HelmRelease `podinfo`
- **THEN** the patch sets `requestedAt` and `forceAt` to identical timestamps

### Requirement: Progress and success messages

While a Flux action runs, the status SHALL show "suspending <name>…", "resuming <name>…", "reconciling <name>…" or "force reconciling <name>…", or "<verb> N <plural>…" for several targets. On success it SHALL show "suspended <name>", "resumed <name>", "reconcile requested: <name>" or "force reconcile requested: <name>" (N <plural> for several).

#### Scenario: Single reconcile success

- **WHEN** Reconcile now succeeds on Kustomization `apps`
- **THEN** the flash reads "reconcile requested: apps"

#### Scenario: Bulk suspend success

- **WHEN** Suspend succeeds on three marked HelmReleases
- **THEN** the flash reads "suspended 3 helmreleases"

### Requirement: Success does not wait for the controller

A Flux action SHALL report success once the API accepts the patch and SHALL NOT wait for the Flux controller to reconcile or for a Helm operation to finish.

#### Scenario: Request confirmation only

- **WHEN** a Force reconcile patch is accepted
- **THEN** "force reconcile requested: <name>" is shown immediately

### Requirement: Partial failures are reported together

When patching a target fails, the remaining targets SHALL still be attempted. Each failure SHALL be described as "<verb> <name> failed: <error>", all failures SHALL be joined with "; " into one error flash, and no success message SHALL be shown.

#### Scenario: One of two targets fails

- **WHEN** Force reconcile is chosen for two HelmReleases and the first patch is rejected
- **THEN** the second is still patched
- **AND** an error flash contains "force reconcile <first> failed:"

### Requirement: Flux actions are journaled

Each Flux action SHALL be recorded in the session action journal as suspend, resume, reconcile or force reconcile, labelled "<name> in <namespace>" for one target or "N <plural>" for several.

#### Scenario: Journal entry for a reconcile

- **WHEN** the user reconciles Kustomization `apps` in `flux-system`
- **THEN** the journal records "reconcile" for "apps in flux-system"

### Requirement: GitOps view opens from the palette

`:gitops`, `:flux`, `:reconcile` and `:recon` SHALL open the GitOps view for the selected row, titled "<name> — GitOps", with status "GitOps: <Kind>/<name>…" until results arrive. Without a resource kind it SHALL flash "select a resource first"; on Helm views "GitOps view is not available for Helm releases"; with no row "no selection for GitOps view".

#### Scenario: Open on a Deployment

- **WHEN** the user selects Deployment `web` and runs `:gitops`
- **THEN** the view titled "web — GitOps" opens and shows "GitOps: Deployment/web…" while loading

#### Scenario: Refused on Helm releases

- **WHEN** the user runs `:flux` in the `:helm` view
- **THEN** the warning "GitOps view is not available for Helm releases" is flashed

### Requirement: GitOps view re-reads the selected resource

Opening or refreshing the view SHALL GET the selected object again rather than use the cached row. If the object's UID changed it SHALL report "<plural>/<name> was replaced; return to the table and select the new resource"; any other read failure SHALL report "reading <plural>/<name>: <error>". Either SHALL appear as an "evidence incomplete — <error>" finding.

#### Scenario: Selected resource was replaced

- **WHEN** the re-read returns an object with a different UID
- **THEN** the findings start with "evidence incomplete —" and name the replacement

### Requirement: Owner is resolved from Flux labels

The owning Flux object SHALL be read from `kustomize.toolkit.fluxcd.io/name` (with `/namespace`), otherwise `helm.toolkit.fluxcd.io/name` (with `/namespace`). A Kustomization or HelmRelease with no such labels SHALL be treated as its own owner, without fetching it again.

#### Scenario: Pod owned by a Kustomization

- **WHEN** the selected pod carries `kustomize.toolkit.fluxcd.io/name: apps` and `/namespace: flux-system`
- **THEN** the headline reads "Pod/<name> is managed by Kustomization/apps"

#### Scenario: Kustomization opened directly

- **WHEN** the view is opened on Kustomization `apps` with no toolkit labels
- **THEN** the headline reads "Kustomization/apps — Flux Kustomization"

### Requirement: Unmanaged resources are reported

A resource with no Flux toolkit labels that is not itself a Kustomization or HelmRelease SHALL be reported as "<Kind>/<name> is not managed by Flux" with the detail "no kustomize/helm toolkit labels found".

#### Scenario: Plain ConfigMap

- **WHEN** the view is opened on a ConfigMap without toolkit labels
- **THEN** the findings read "ConfigMap/<name> is not managed by Flux"

### Requirement: Owner section shows readiness and revisions

Under an "Owner" heading the view SHALL show "<Kind>/<name>" (a jump target unless it is the selection), "owner not found in cluster" if missing, "suspended" if suspended, "Ready: <status> (<reason>) — <message>" or "no Ready condition yet", "applied revision <rev>" from `status.lastAppliedRevision` or the latest Helm history chart version, and "last attempted <rev> (not yet applied)" when the attempted revision differs.

#### Scenario: Attempted revision differs

- **WHEN** the owner's `lastAttemptedRevision` differs from `lastAppliedRevision`
- **THEN** the Owner section includes "last attempted <rev> (not yet applied)"

### Requirement: Source section follows the source reference

Under a "Source" heading the view SHALL show the source named by `spec.sourceRef`, else `spec.chartRef`, else `spec.chart.spec.sourceRef`, defaulting to the owner's namespace, as "<Kind>/<name>" or "<Kind>/<name> (not found)" with a jump target, followed by "revision <artifact revision>". With no reference it SHALL show "no source resolved".

#### Scenario: GitRepository source

- **WHEN** the owner Kustomization references GitRepository `flux-system`
- **THEN** the Source section shows "GitRepository/flux-system" and its artifact revision

### Requirement: Dependencies are listed with their state

When the owner declares `spec.dependsOn`, a "Depends on" heading SHALL list each dependency as a Kustomization (namespace defaulting to the owner's) with "<name> — ready", "not ready", "not found" or "unknown". The heading SHALL be omitted when there are no dependencies.

#### Scenario: Dependency not ready

- **WHEN** the owner depends on Kustomization `infra` whose Ready condition is False
- **THEN** the view shows "infra — not ready" under "Depends on"

### Requirement: Reconciliation verdict

Under a "Reconciliation" heading the view SHALL state, first match wins: "owner is suspended — not reconciling"; "blocked: source <name> is not ready"; "waiting on dependency <name>"; otherwise "reconciled at <rev>", "not ready (<reason>) — <message>" or "not yet reconciled".

#### Scenario: Blocked on source

- **WHEN** the owner is not suspended and its source is not ready
- **THEN** the Reconciliation line reads "blocked: source <name> is not ready"

### Requirement: Missing evidence is flagged

A read of the owner, source or a dependency that fails with anything other than 404 SHALL prefix the findings with "evidence incomplete — reading <plural>/<name>: <error>"; a 404 SHALL be treated as not found. Only Kustomizations, HelmReleases, GitRepositories, OCIRepositories, Buckets, HelmRepositories and HelmCharts SHALL be fetched.

#### Scenario: Forbidden source read

- **WHEN** reading the source GitRepository returns 403
- **THEN** the first finding starts with "evidence incomplete — reading gitrepositories/<name>:"

### Requirement: Managed resources from the inventory

A "Managed resources" section SHALL list up to 500 entries of the selected object's `status.inventory.entries` as "<Kind>[.<group>]/<name> (<namespace>|cluster, <version>)", with jump targets, without reading those resources. Beyond 500 it SHALL add "N more inventory entries omitted".

#### Scenario: Cluster-scoped inventory entry

- **WHEN** the inventory holds id `_ns1__Namespace` with version `v1`
- **THEN** the view lists "Namespace/ns1 (cluster, v1)" with a jump target

### Requirement: Inventory problems are reported inline

The inventory section SHALL show "no inventory reported" when absent, "invalid inventory entries" when not a list, "no managed resources reported" when empty, "invalid inventory entry" for malformed IDs, ": resource kind unavailable" when discovery does not know the kind, and ": invalid namespace scope" when the namespace contradicts the kind's scope.

#### Scenario: Unknown kind in inventory

- **WHEN** an inventory entry names a kind the cluster does not serve
- **THEN** its line ends with ": resource kind unavailable" and has no jump target

### Requirement: Remote inventories are not navigable

When the selected Flux object sets `spec.kubeConfig`, the inventory SHALL be listed without jump targets and SHALL state "remote cluster configured: inventory navigation is unavailable".

#### Scenario: Kustomization targeting another cluster

- **WHEN** the Kustomization has a non-null `spec.kubeConfig`
- **THEN** no inventory line is a jump target

### Requirement: Enter jumps to a finding's target

Enter on a finding with a target SHALL open that kind's table, resolved group-aware via discovery, in the target's namespace (all scope for cluster-scoped) filtered by field selector `metadata.name=<name>`. Enter on a line without a target SHALL flash "no resource to jump to on this line".

#### Scenario: Jump to the source

- **WHEN** the user presses Enter on the "GitRepository/flux-system" line
- **THEN** the gitrepositories table opens with `metadata.name=flux-system`

### Requirement: Initial cursor lands on the first target

When findings arrive the cursor SHALL be placed on the first finding that has a jump target, or on the first line when none has one.

#### Scenario: Cursor on owner line

- **WHEN** the report for a managed pod arrives
- **THEN** the selected line is the first jumpable finding

### Requirement: Refresh re-follows current references

`r` in the GitOps view SHALL re-read the selection and follow its current owner labels, source and dependencies. Previous findings SHALL stay visible until the new report arrives, and only the latest request SHALL update the view; an older reply SHALL be discarded even if it succeeded.

#### Scenario: Stale reply dropped

- **WHEN** the user presses `r` twice and the first reply arrives last
- **THEN** the view shows the second report

### Requirement: Leaving the GitOps view cancels pending work

`esc` or `q` SHALL return to the previous mode, restoring the table selection, cancel any pending report and clear its progress status. A jump or palette navigation SHALL also cancel it, and a late reply SHALL NOT change the mode.

#### Scenario: Esc before results

- **WHEN** the user opens `:gitops` and presses `esc` before the report arrives
- **THEN** the table is shown, the progress status is cleared and the late report is ignored

### Requirement: Long finding text is truncated

Messages and revisions in GitOps findings SHALL be truncated to 80 characters with an ellipsis.

#### Scenario: Long Ready message

- **WHEN** the owner's Ready message exceeds 80 characters
- **THEN** the shown text ends with an ellipsis

### Requirement: Enter on a HelmRelease opens its Helm history

Enter on a Flux HelmRelease row SHALL open the Helm revision history of the release it manages, using `spec.releaseName`, else "<targetNamespace>-<name>", else the name, stored in `spec.storageNamespace` or else the object's namespace. It SHALL flash "↳ <release> history" and `esc` SHALL return to the HelmRelease list. If no release name resolves it SHALL flash "HelmRelease has no resolvable release name".

#### Scenario: Storage namespace override

- **WHEN** the user presses Enter on HelmRelease `podinfo` in `flux-system` with `storageNamespace: apps`
- **THEN** the Helm history for release `podinfo` in namespace `apps` opens
- **AND** `esc` returns to the helmreleases table
