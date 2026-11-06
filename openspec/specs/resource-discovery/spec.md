# resource-discovery Specification

## Purpose

Describe how sofka discovers the resource types a cluster serves, picks a version for each kind, resolves names, short names and aliases to resource types, reports discovery problems, and fetches custom resource printer columns.

## Requirements

### Requirement: Aggregated discovery

On connect, sofka SHALL discover resource types with aggregated discovery, requesting `/api` and `/apis` once each. A healthy connection MUST need only these two requests plus `/version`, including for clusters whose aggregated lists are empty and for reading discovered short names.

#### Scenario: Three requests

- **WHEN** sofka connects to a cluster that supports aggregated discovery
- **THEN** it makes no discovery requests beyond `/api`, `/apis` and `/version`

### Requirement: Legacy discovery

When the server answers `/api` or `/apis` with a legacy discovery document, sofka SHALL read every version of every API group separately.

#### Scenario: Older API server

- **WHEN** `/apis` returns an `APIGroupList`
- **THEN** sofka reads each group version's resource list

### Requirement: Aggregated discovery fallback note

When aggregated discovery fails for any other reason, sofka SHALL use legacy discovery and record the note `Aggregated API discovery failed: <error>. Sofka read each API group separately.` The note MUST be printed as a startup warning and MUST NOT count as an unread API group.

#### Scenario: Unexpected response kind

- **WHEN** `/apis` returns an unexpected document kind
- **THEN** stderr shows the fallback note
- **AND** the unread group count stays 0

### Requirement: Non-preferred versions

Discovery SHALL include resources from every served version of a group, not only the preferred version, in both aggregated and legacy discovery, so a kind served only in a non-preferred version still resolves.

#### Scenario: Kind only in an older version

- **WHEN** group `example.com` prefers `v2` and kind `Legacy` is served only in `v1beta1`
- **THEN** `legacies` resolves to the `v1beta1` version

### Requirement: Version selection

For each API group and kind, sofka SHALL use the most stable served version, preferring GA over beta and beta over alpha, and higher numbers within the same level.

#### Scenario: GA wins

- **WHEN** a kind is served as `v1beta1` and `v1`
- **THEN** sofka watches and shows the `v1` version

### Requirement: Unreadable API groups

During legacy discovery, an API group version that cannot be read SHALL be skipped with the warning `API discovery could not read <group/version>: <error>`, and a group with no versions SHALL be skipped with `API discovery could not read <group>: the group has no versions`. The connection MUST still succeed.

#### Scenario: Extension API server down

- **WHEN** reading `metrics.k8s.io/v1beta1` fails with HTTP 503
- **THEN** sofka connects without that group
- **AND** records a warning naming `metrics.k8s.io/v1beta1`

### Requirement: Core group is required

When the core API group (`/api/v1`) cannot be read, discovery MUST fail with `running API discovery: reading core API group v1` and the cause.

#### Scenario: Core group fails

- **WHEN** `/api/v1` returns an error during legacy discovery
- **THEN** the connection fails with an error naming the core API group

### Requirement: Stale API services

Aggregated discovery SHALL tolerate stale or broken APIService entries, such as a failed metrics-server, and MUST still connect with the remaining resources.

#### Scenario: Broken metrics-server

- **WHEN** the metrics-server APIService is unavailable
- **THEN** sofka connects and Pods resolve

### Requirement: Discovery warnings are reported

Unread API groups SHALL be reported as stderr warnings at startup, as a flash `API discovery could not read N API group(s). Refer to :info for details.` on the first screen, as a count in `--check`, and with group and reason in `:info`.

#### Scenario: Flash points to info

- **WHEN** one API group cannot be read at startup
- **THEN** the first screen flashes `API discovery could not read 1 API group. Refer to :info for details.`

### Requirement: Subresources are not resource types

Subresources such as `pods/log` or `deployments/scale` MUST NOT become resource types or resolvable names.

#### Scenario: Subresource name

- **WHEN** the user enters `:pods/log`
- **THEN** it does not resolve to a resource type

### Requirement: Scale support

sofka SHALL treat a kind as scalable only when discovery reports a `scale` subresource that allows the `patch` verb.

#### Scenario: Read-only scale subresource

- **WHEN** a custom kind has a `scale` subresource with only `get`
- **THEN** sofka does not treat the kind as scalable

### Requirement: Child resource kinds

Discovery SHALL record the listable, namespaced resource types, one version each, as the candidate kinds for finding objects owned by another object.

#### Scenario: Not listable

- **WHEN** a namespaced kind does not allow `list`
- **THEN** it is not a candidate child kind

### Requirement: Qualified resource names

Every resource in a named API group SHALL resolve by `plural.group`, such as `pods.metrics.k8s.io`, regardless of priority or collisions.

#### Scenario: Shadowed resource

- **WHEN** both core `pods` and `pods.metrics.k8s.io` exist
- **THEN** `pods.metrics.k8s.io` opens the metrics resource

### Requirement: Bare plural and kind names

Each resource SHALL also resolve by its bare plural and its lowercase kind. When several groups share a bare name, the group with the higher priority MUST own it, and ties between equal priorities MUST resolve the same way on every run.

#### Scenario: Core wins

- **WHEN** both core `pods` and `pods.metrics.k8s.io` exist
- **THEN** `pods` and `pod` open core Pods

### Requirement: Group priority

Group priority SHALL be, from highest to lowest: the core group; `apps`; `batch`; `networking.k8s.io`; `rbac.authorization.k8s.io`, `storage.k8s.io` and `policy`; every other group; and `metrics.k8s.io` last.

#### Scenario: Metrics last

- **WHEN** a custom group `example.com` and `metrics.k8s.io` both serve `widgets`
- **THEN** `widgets` resolves to the `example.com` resource

### Requirement: Built-in aliases

sofka SHALL provide k9s-style aliases including `po`, `dp`, `deploy`, `svc`, `ns`, `no`, `cm`, `sec`, `sts`, `ds`, `rs`, `rc`, `ing`, `pv`, `pvc`, `sa`, `jo`, `cj`, `ep`, `ev`, `hpa`, `pc`, `crd`, `cr`, `crb`, `ro`, `rb`, `np`, `pdb`, `sc`, `ks` and `hr`. An alias MUST resolve only when its target was discovered and MUST NOT replace a resource name or kind.

#### Scenario: Flux alias without Flux

- **WHEN** the cluster does not serve Flux kustomizations
- **THEN** `ks` does not resolve

### Requirement: Discovered short names

Short names reported by discovery, such as `md` for MachineDeployments, SHALL resolve to their resource. They MUST NOT replace resource names, kinds or built-in aliases. When several resources claim one short name, group priority MUST decide, then alphabetical group and resource order.

#### Scenario: Custom resource short name

- **WHEN** the cluster serves MachineDeployments with short name `md`
- **THEN** `:md` opens MachineDeployments

### Requirement: Config aliases

Entries in the config `[aliases]` table SHALL map an alias to a target resource name, which can be group-qualified as `plural.group` when names overlap. A config alias SHALL override any other name, including resource names, built-in aliases and discovered short names. An alias MUST be ignored when its target does not resolve.

#### Scenario: Override a short name

- **WHEN** the config sets `[aliases] md = "machinedeployments.cluster.x-k8s.io"`
- **THEN** `md` opens that resource even if another resource claims `md`

### Requirement: Config aliases follow the context

Config aliases SHALL be applied at startup and again after every context switch, using the per-cluster and per-context config overrides of the connected context.

#### Scenario: Context-specific alias

- **WHEN** only the `prod` context override defines `[aliases] app = "applications"`
- **AND** the user switches from `prod` to `dev`
- **THEN** `app` no longer uses the `prod` alias

### Requirement: Rediscovery on context switch

A context switch SHALL run API discovery for the target cluster and replace the resource catalog, names and aliases of the previous cluster.

#### Scenario: CRD only on one cluster

- **WHEN** `widgets` exists only on cluster `east`
- **AND** the user switches from `east` to `west`
- **THEN** `widgets` no longer resolves

### Requirement: Name lookup normalization

Name lookup SHALL be case-insensitive and MUST ignore surrounding whitespace and a leading `:`.

#### Scenario: Mixed case

- **WHEN** the user enters `:Deployments`
- **THEN** it resolves to Deployments

### Requirement: Owner kind lookup

sofka SHALL resolve a resource type from an API group and kind as owner references name it, independent of name priority.

#### Scenario: Owner in a lower-priority group

- **WHEN** an object's owner reference names kind `Widget` in group `example.com`
- **AND** another group owns the bare name `widget`
- **THEN** the owner resolves to `example.com` Widgets

### Requirement: Resource catalog

sofka SHALL keep a sorted, deduplicated catalog of discovered plurals and `plural.group` names for completion, suggestions and the `--check` kinds count.

#### Scenario: Grouped resource entries

- **WHEN** the cluster serves `deployments` in group `apps`
- **THEN** the catalog contains both `deployments` and `deployments.apps`

### Requirement: CRD printer column fetch

When the user opens a custom resource by its plural, and the resource has no curated columns and no configured view columns, sofka SHALL read its CustomResourceDefinition within 10 seconds and use the `additionalPrinterColumns` of the served version as table columns.

#### Scenario: CRD columns appear

- **WHEN** the `widgets.example.com` CRD defines a printer column `Ready` for the served version
- **AND** the user opens `widgets`
- **THEN** the table shows a `READY` column

### Requirement: CRD printer column fallback

When CustomResourceDefinitions cannot be resolved, the CRD read fails or times out, or the served version has no printer columns, sofka MUST fall back to server-provided table columns.

#### Scenario: CRD read denied

- **WHEN** reading the CRD for `caliconodestatuses.projectcalico.org` is forbidden
- **THEN** sofka requests the server table columns for that resource instead

### Requirement: CRD printer column cache

Printer columns SHALL be fetched once per resource and cached for the session, including a missing result. A context switch MUST clear the cache. Opening custom resources from a CustomResourceDefinition row SHALL reuse the columns of that CRD without another read.

#### Scenario: Reopen the same resource

- **WHEN** the user opens `widgets`, leaves, and opens `widgets` again
- **THEN** sofka does not read the CRD a second time
