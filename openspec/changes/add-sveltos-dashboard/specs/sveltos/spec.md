## Purpose

Provide read-only Sveltos deployment visibility inside sofka, correlating managed clusters, profiles, provisioning evidence, and reported inventory without connecting to downstream clusters.

## ADDED Requirements

### Requirement: Dashboard entry and compatibility

`:sveltos` SHALL open the Sveltos dashboard for the connected management context, without requiring a selected resource. It SHALL appear in command completion and help. The dashboard MUST support the Sveltos v1.16.0 API contract. When discovery successfully establishes that none of ClusterProfile, Profile, or ClusterSummary is served in `config.projectsveltos.io`, the command SHALL leave the current view unchanged and warn that Sveltos APIs are not available. Discovery failures MUST be reported as incomplete evidence rather than proof that Sveltos is absent.

#### Scenario: Open independently of selection

- **WHEN** the user runs `:sveltos` with no selected resource and Sveltos APIs are served
- **THEN** the dashboard opens on the Clusters tab for the connected context

#### Scenario: Sveltos absent

- **WHEN** discovery completes successfully and none of the three Sveltos configuration kinds is served
- **THEN** the current view remains open and a Sveltos API unavailable warning is shown

### Requirement: Management-context-only read boundary

The dashboard SHALL use only management-context discovery and reads of SveltosCluster, CAPI Cluster, ClusterProfile, Profile, ClusterSummary, and ClusterConfiguration. It MUST NOT read Secret contents, connect to downstream clusters, switch context automatically, invoke a Sveltos backend or CLI, or issue mutation requests. Dashboard actions SHALL remain read-only even when sofka runs in write mode.

#### Scenario: Inventory resource lives downstream

- **WHEN** the user selects a downstream resource in reported inventory and presses Enter
- **THEN** no downstream request or context switch occurs
- **AND** the dashboard explains that only management-cluster evidence is available

#### Scenario: Mutating key in write mode

- **WHEN** sofka runs in write mode and the user presses an edit, delete, suspend, or sync binding in the dashboard
- **THEN** no mutation or external editing process starts

### Requirement: Inventory scope and optional cluster APIs

The dashboard SHALL read cluster-scoped profiles and namespaced data across the connected management cluster, independent of the originating table namespace. It SHALL include SveltosCluster in `lib.projectsveltos.io` and Cluster in `cluster.x-k8s.io` when served. Successfully discovered absence of either cluster API SHALL be labelled unavailable, not unhealthy. Read/discovery failures SHALL be distinguished from absent APIs.

#### Scenario: Namespace-scoped origin

- **WHEN** the user opens the dashboard from a pods table scoped to namespace `team-a`
- **THEN** the dashboard can show clusters and profiles in other management namespaces
- **AND** the header identifies its all-namespaces management scope

#### Scenario: CAPI absent

- **WHEN** CAPI Cluster is not served and SveltosCluster is served
- **THEN** SveltosCluster rows remain available and CAPI is labelled unavailable

### Requirement: Two overview tabs

The dashboard SHALL provide Clusters and Profiles tabs, with Clusters selected on each fresh open. Tab and Shift-Tab SHALL switch overview tabs; each tab SHALL preserve its own selection and filter while the dashboard remains open. Profiles SHALL contain ClusterProfile and namespaced Profile, with kind and namespace visible.

#### Scenario: Same profile name in different scopes

- **WHEN** ClusterProfile `base` and Profile `team/base` exist
- **THEN** the Profiles tab shows two distinct rows with their kinds and scopes

### Requirement: Cluster overview

Each cluster row SHALL show type, namespace, name, reported readiness, reported connectivity where available, paused/deleting state, and deployment counts grouped by provisioning state. The dashboard MUST NOT treat CAPI control-plane initialization as proof of downstream connectivity. Missing readiness or connectivity evidence SHALL read unknown or not reported rather than healthy.

#### Scenario: Initialized CAPI cluster

- **WHEN** CAPI reports its control plane initialized but supplies no connectivity evidence
- **THEN** the row shows the initialization/readiness evidence separately and connectivity as not reported

#### Scenario: Paused cluster with successful deployments

- **WHEN** a paused cluster has provisioned feature reports
- **THEN** its row still shows the paused flag rather than presenting it as actively reconciling

### Requirement: Profile overview and observed targeting

Each profile row SHALL show kind, namespace, name, sync mode, controller-reported matching target count, updated/updating rollout counts, provisioning counts, and failure/suspension indicators. Matching targets SHALL come from observed `status.matchingClusters`, not a client-side reimplementation of selectors or Sets. Declared selectors, explicit cluster references, Set references, dependencies, tier, and maxUpdate SHALL be inspectable in profile detail. Missing target status SHALL remain unknown; an explicit empty target set SHALL read no matching clusters, not successful rollout.

#### Scenario: Explicitly empty targets

- **WHEN** a profile reports `matchingClusters: []`
- **THEN** its target count reads zero with no matching clusters
- **AND** it is not labelled successfully deployed

#### Scenario: Target status not reported

- **WHEN** a profile has selectors but no matching target status
- **THEN** the target count reads unknown and the declared selector remains inspectable

### Requirement: Identity-safe correlation

Cluster correlation SHALL distinguish cluster API group/type, namespace, and name. Profile correlation SHALL distinguish API group, kind, namespace, and name, and validate an owner UID when present. The dashboard SHALL correlate ClusterSummary using its cluster reference fields and profile ownerReference, not by guessing generated names. Missing, replaced, ambiguous, or unreadable referenced objects SHALL remain explicitly unresolved.

#### Scenario: Same cluster name in different types

- **WHEN** SveltosCluster `team/west` and CAPI Cluster `team/west` both exist
- **THEN** their deployments and inventory remain separate

#### Scenario: Profile recreated

- **WHEN** a ClusterSummary owner UID differs from the UID of a same-named current profile
- **THEN** the summary is shown as unresolved/replaced-owner evidence and is not attributed to the new profile

### Requirement: Convergent drill-down

Enter on a cluster SHALL open its related profile deployments. Enter on a profile SHALL open its observed target clusters and related deployments. Enter on a deployment SHALL open the same profile/cluster detail regardless of entry path. Related lists MUST retain observed targets without ClusterSummary, profile-reported target failures, and summaries being removed after a cluster stops matching. An unresolved summary SHALL remain inspectable without inventing its missing owner or cluster.

#### Scenario: Summary creation failed

- **WHEN** a matching target has no ClusterSummary and the profile reports a failure for it
- **THEN** the related list includes the target with its reported failure and missing-summary state

#### Scenario: No longer matching

- **WHEN** a ClusterSummary is removing after its cluster stops matching the profile
- **THEN** the related list retains it as no longer matching/removing evidence

### Requirement: Provisioning evidence

Deployment detail SHALL show raw feature states for Helm, Resources, and Kustomize, feature failure reason/message, consecutive failures, lastAppliedTime, summary-level failureMessage, dependency status, suspension reason, and nextReconcileTime when reported. It MUST preserve unknown feature IDs/statuses visibly. Configured features with no report SHALL read pending or unknown, not provisioned; unconfigured features SHALL read not configured.

#### Scenario: Configured feature lacks report

- **WHEN** a summary configures Helm charts but reports no Helm feature status
- **THEN** Helm reads pending/unknown and the deployment is not counted provisioned

#### Scenario: Unknown status

- **WHEN** a feature reports an unrecognized state
- **THEN** that state remains visible and does not count as success

### Requirement: Provisioning state aggregation

Provisioning counts SHALL count profile/cluster deployments, not individual charts or feature entries. A deployment SHALL count provisioned only when all configured features are reported Provisioned and no reported feature is in a non-Provisioned state. Explicit failures or Helm conflicts SHALL prevent success classification. Empty feature configuration SHALL read no features configured, not healthy. Failed, progressing, removing, blocked, missing-summary, and unknown evidence SHALL remain distinguishable; pause/suspension and DryRun SHALL be independent visible flags.

#### Scenario: Mixed feature states

- **WHEN** Helm is Provisioned and Resources is Failed for one deployment
- **THEN** the deployment contributes one failed deployment, not one successful and one failed deployment

#### Scenario: Removal blocked

- **WHEN** a feature reports Blocked during removal
- **THEN** the raw Blocked state and removal context remain visible rather than being described as an installation dependency failure

### Requirement: Rollout progress is separate from provisioning

The dashboard SHALL label updated/updating counts as controller-reported rollout evidence, separate from feature provisioning. Feature Provisioned alone MUST NOT imply that the latest profile configuration was applied. Missing or conflicting rollout evidence SHALL be marked unknown/inconsistent. OneTime SHALL be identified as apply-once; DryRun SHALL be identified as non-applying and MUST NOT count as deployed rollout success.

#### Scenario: Old provisioned summary

- **WHEN** a summary reports Provisioned but its target is not reported updated by the profile
- **THEN** provisioning remains visible separately from rollout status
- **AND** the dashboard does not claim that the latest profile configuration is applied

#### Scenario: DryRun

- **WHEN** a profile uses DryRun
- **THEN** its rows state that configuration is not being applied and do not claim deployment success

### Requirement: Helm conflicts and deployed inventory

Deployment detail SHALL show Helm release identity, management/conflict status, conflict and failure messages, and reported chart/version information. Deployed charts and resources SHALL be read from the corresponding ClusterConfiguration profile section and labelled reported inventory. Resource identity SHALL include group, kind, namespace, and name, plus version, deployment destination, and last applied time when reported. Inventory availability MUST be independent of provisioning availability.

#### Scenario: Inventory denied

- **WHEN** ClusterSummary can be read but ClusterConfiguration cannot
- **THEN** provisioning evidence remains visible
- **AND** inventory is marked unavailable with the read error, not empty

#### Scenario: Resource deployed locally

- **WHEN** a reported resource has deploymentType Local
- **THEN** inventory distinguishes it from resources reported as Remote

### Requirement: Workload health is not inferred

The dashboard SHALL state that downstream workload runtime health is not verified. Provisioned, Helm Managing, and inventory presence MUST NOT be labelled proof of healthy Pods, successful runtime checks, or downstream connectivity.

#### Scenario: Provisioned chart

- **WHEN** Sveltos reports a chart feature Provisioned
- **THEN** the dashboard shows provisioning evidence alongside the runtime-health-not-verified limitation

### Requirement: Filtering and keyboard navigation

Overview and related lists SHALL support local case-insensitive substring filtering across displayed name, namespace, kind/type, and state fields. `/` SHALL edit the filter, Enter SHALL apply it, and Esc SHALL cancel editing without changing the prior filter. Navigation SHALL use existing movement, first/last, and paging actions. Esc SHALL go back one drill level, then leave the dashboard at its root; `q` SHALL close it. `:` and `?` SHALL work as overlays and return to the same dashboard state when cancelled.

#### Scenario: Filter failed deployments

- **WHEN** the user applies filter `failed` to a related deployment list
- **THEN** only rows whose displayed fields contain `failed` remain
- **AND** clearing the filter restores the other rows without new API requests

### Requirement: Source YAML inspection

`y` SHALL inspect the management-cluster source CR represented by the selected row and return to the same dashboard selection when closed. Deployment detail SHALL offer explicit source rows for the profile, cluster, summary, and inventory CR when available. Source reads MUST preserve API group and object identity; missing/replaced sources SHALL warn instead of displaying a different object. Non-source inventory rows SHALL explain that downstream object YAML is unavailable in this view.

#### Scenario: Inspect summary

- **WHEN** the user selects the summary source row in deployment detail and presses `y`
- **THEN** its management-cluster YAML opens
- **AND** closing the document restores the same detail selection

### Requirement: Refresh and return lifecycle

The dashboard SHALL gather asynchronously on open, every five seconds after each gather completes, and on `r`. Only one gather SHALL be active at a time. Refresh SHALL preserve selections by object identity where possible and clamp removed selections safely. The view SHALL display last successful read time and whether data is refreshing, incomplete, or stale. Leaving or switching context SHALL cancel pending dashboard work; late replies MUST NOT replace a newer view or another context. Closing SHALL restore the originating resource table's namespace, filter, selection, and watch.

#### Scenario: Manual refresh supersedes old gather

- **WHEN** the user presses `r` during an active gather
- **THEN** the old gather is cancelled and only results from the new gather update the dashboard

#### Scenario: Source disappears

- **WHEN** a refresh removes the selected profile
- **THEN** no other profile silently inherits its detail view and the missing source is explained

### Requirement: Partial evidence and bounded reads

Failed, denied, timed-out, truncated, or malformed reads SHALL produce a persistent incomplete-data banner naming affected sources; readable evidence SHALL remain usable. Dependent counts MUST NOT silently become zero or successful. Reads SHALL be paginated and bounded to four concurrent requests, five seconds per request, thirty seconds per gather, two hundred list requests, and twenty thousand objects per gather. Hitting a limit SHALL retain collected evidence and report which limit was reached.

#### Scenario: Partial RBAC

- **WHEN** listing CAPI Cluster returns 403 while SveltosCluster reads succeed
- **THEN** available SveltosCluster rows remain visible
- **AND** a persistent warning identifies CAPI Cluster as unreadable

#### Scenario: Object limit reached

- **WHEN** a gather reaches twenty thousand objects while additional pages remain
- **THEN** it stops reading, keeps collected evidence, and reports incomplete data due to the object limit

#### Scenario: Failed refresh after success

- **WHEN** a previously readable source fails on refresh
- **THEN** retained evidence from that source is labelled stale with its last successful read time
- **AND** stale evidence is not promoted to a fresh success claim
