## Context

See `proposal.md` for motivation and `specs/sveltos/spec.md` for the behavior contract.

Existing integration points:

- `src/app/dashboards.rs` opens Pulse, gathers asynchronously, and refreshes every five seconds.
- `src/app/mod.rs` defines modes, state, and `PALETTE_COMMANDS`; `src/app/input.rs` dispatches actions and handles palette completion.
- `src/store.rs` carries dashboard replies; `src/app/lifecycle.rs` applies generation-tagged messages and handles context cleanup.
- `src/app/argocd.rs` demonstrates report requests, source identity checks, document return state, and cancellation.
- `src/k8s.rs` resolves kinds by API group and supports DynamicObject access. Generic CRD discovery already exists; it does not supply cross-object Sveltos correlation.
- `src/keymap.rs` already provides Open, Back, Close, Refresh, Filter, NextView, PreviousView, navigation, and YAML actions.
- `src/ui.rs` renders modes and effective help. `src/app/tests.rs` tests dashboard errors and stale messages without a cluster.

Public Sveltos v1.16.0 sources were inspected; no infrastructure or cluster was queried. The addon-controller release uses libsveltos v1.16.0. Baseline fields include profile matching/updated/updating/failed clusters; ClusterSummary feature reports, dependency text, suspension, next reconcile time, and Helm conflict/failure reports; ClusterConfiguration inventory; and SveltosCluster readiness/connectivity.

## Goals / Non-Goals

**Goals:** One reusable evidence model for both tab orientations; identity-safe joins; partial evidence without false success; bounded asynchronous reads; existing input, theme, and document conventions.

**Non-Goals:** Reimplementing Sveltos selectors, dependency evaluation, hash computation, or scheduling; introducing a generic dashboard framework; reading credential Secrets; interpreting inventory as observed runtime health.

## Decisions

### 1. Native built-in dashboard, not plugin/backend

Add a pure `src/sveltos.rs` evidence/correlation module and a thin `src/app/sveltos.rs` gather/input driver. Keep rendering in `src/ui.rs`, matching current project structure. Register the module in the existing crate and application module trees.

Add one dashboard mode with an internal page enum for overview, cluster-related deployments, profile-related deployments, and deployment detail. Do not add a separate application mode per tab or a new generic framework.

Alternative: user-configured columns/references cover single-object browsing but cannot express missing-summary target rows, rollout correlation, or inventory sections. An external adapter/backend would add deployment and process dependencies without helping this bounded read-only feature.

### 2. Resolve only explicit API groups

| Kind                 | Group                    | Scope      |
| -------------------- | ------------------------ | ---------- |
| SveltosCluster       | lib.projectsveltos.io    | Namespaced |
| Cluster              | cluster.x-k8s.io         | Namespaced |
| ClusterProfile       | config.projectsveltos.io | Cluster    |
| Profile              | config.projectsveltos.io | Namespaced |
| ClusterSummary       | config.projectsveltos.io | Namespaced |
| ClusterConfiguration | config.projectsveltos.io | Namespaced |

Use discovered served versions rather than hardcoded URLs. Baseline Sveltos resources use v1beta1. CAPI must handle the served shape: v1beta2 initialization/conditions and legacy v1beta1 fields where served. Show raw evidence for unsupported shapes rather than inventing a readiness result. CAPI readiness follows the Sveltos baseline's control-plane-initialized evidence, not a fabricated connection check.

No global namespace configuration changes. The dashboard reads all management namespaces and advertises that scope; it restores the originating table unchanged. An absent optional API is different from denied discovery or listing. The dashboard may show partial Sveltos evidence if some kinds are absent/unreadable.

Alternative: bare kind resolution risks collisions such as unrelated Cluster kinds. Adding version-pinned Rust CRD dependencies is unnecessary for these read-only JSON fields.

### 3. Stable identities and evidence joins

Retain source DynamicObjects for inspection. Use small typed keys for cluster identity (group/type, namespace, name) and profile identity (group, kind, namespace, name). Validate source/owner UIDs when present; a replacement is not the same source.

- ClusterSummary supplies target identity through `spec.clusterType`, `clusterNamespace`, and `clusterName`.
- Its Sveltos profile ownerReference supplies profile kind, name, and UID. A namespaced Profile owner resolves in the summary namespace; ClusterProfile is cluster-scoped.
- Profile `status.matchingClusters` supplies controller-observed targeting. Include matching targets even when no summary exists, and join `failedClusters` to expose summary creation/update failures.
- Preserve existing summaries outside current matching targets as no-longer-matching/removal evidence.
- ClusterConfiguration identifies its cluster using its namespace and labels `projectsveltos.io/cluster-name` and `projectsveltos.io/cluster-type`. Read `clusterProfileResources` or `profileResources` for the selected owner. Their feature array is serialized as `Features`, with an uppercase F in v1.16.0.
- Validate inventory profile ownership where owner UID evidence exists. Missing labels, owners, duplicates, mismatched identities, and disappeared references become diagnostic rows, not guessed joins.

Unresolved summaries remain selectable in an explicitly labelled unresolved section of overview/related lists. Never drop useful failure evidence because an owner or cluster cannot be read.

Alternative: parsing generated CR names is shorter initially but breaks identity, namespace/type collisions, and replaced-object handling. Re-evaluating selectors/Sets would duplicate controller logic and create inconsistent target sets.

### 4. Keep status dimensions separate

Evidence model separates:

1. Cluster readiness, connection status, pause/deletion flags, and pull-mode heartbeat if present.
2. Controller-reported matching/updated/updating rollout lists.
3. Summary-level and feature-level deployment states.
4. Helm ownership/conflict reports and deployed inventory.
5. Data availability, freshness, and completeness.

Use profile rollout lists as explicitly labelled controller reports. Do not reproduce Sveltos' internal hash algorithm or infer latest rollout completion from feature status. A copied summary configuration differing from its current owner is useful configuration-difference evidence, not proof of scheduling or failure. Contradictory rollout lists, mismatched references, or unavailable owner evidence remain flagged.

Configured features come from the summary's copied profile spec: nonempty `helmCharts`, `policyRefs`, and `kustomizationRefs`. Show expected but missing feature reports as pending/unknown. A summary with no configured features reads no features configured. Preserve unknown feature IDs and states; do not normalize them to success.

For count purposes, one profile/cluster deployment has one primary provisioning bucket: explicit failure/conflict first; removal states (including removal Blocked) next; provisioning next; unknown/missing evidence next; provisioned only if every configured feature is Provisioned and every reported feature is Provisioned. Missing-summary and no-features cases have their own buckets. Show raw states in detail; pause, suspension, DryRun, and OneTime remain separate flags. DryRun is excluded from deployed-success counts.

Feature `Blocked` in v1.16.0 describes deferred removal; installation dependency waits come from dependency evidence. LastAppliedTime is a reported reconciliation time, not a guarantee that evidence remains current. Helm Managing means ownership, not runtime health.

Alternative: one red/green health badge hides stale rollouts, paused controllers, and mixed states. Show separate columns and short labels instead.

### 5. Gather bounded snapshots, not six watch streams

Reuse the dashboard task/message pattern, adding a dashboard request token alongside context generation. Gather each discovered allowlisted kind once per refresh, paginate at 500 objects, and correlate in memory. Do not issue requests per row. Apply the spec's fixed limits: four concurrent requests, five seconds per request, thirty seconds total, two hundred list requests, twenty thousand collected objects.

Polling runs five seconds after the previous gather finishes. Manual refresh cancels and supersedes it; requests never overlap. Abort work on close, context change, or navigation away; reject replies with stale generation/request tokens. Integrate cleanup with existing mode reset and context-switch paths.

Represent each source read as available, absent, incomplete, or failed, with capture time and error. Keep current partial evidence when bounded reads stop. If a source read fails entirely after an earlier success, retain its last evidence as stale; mark all dependent conclusions stale/incomplete. Never interpret failed reads as empty lists. Clear warnings only after the affected source succeeds again.

Snapshots across kinds are not transactional. Treat join inconsistencies as incomplete evidence and let the next poll resolve them. Do not claim one atomic Kubernetes snapshot.

Alternative: dedicated watchers could reduce repeat reads but add six independent lifecycles and caches. Bounded polling matches existing dashboards and keeps the first version smaller.

### 6. Navigation and source inspection

```text
:sveltos
   |
   +--> Clusters [default] --> cluster deployments ---+
   |                                                |
   +--> Profiles ----------> profile targets --------+
                                                    |
                                                    v
                                           deployment detail
                                           states / blockers
                                           reported inventory
                                           source CR rows
```

Tab/Shift-Tab switch tabs at overview level. Enter drills, Esc pops the dashboard page stack, q closes. Use existing key actions through a `sveltos` keymap scope; add no new action enum if existing actions suffice. Register default bindings, scope validation, help, and overlay return routing together.

Each overview tab and related list keeps a filter and identity-based selection. Implement the specified substring filter using existing text-input handling; do not graft synthetic dashboard rows into the resource table's structured/server-selector grammar. Explain the simpler grammar in documentation.

Deployment detail contains scrollable feature/blocker, Helm, inventory, and explicit source-CR sections. Source rows support y through existing YAML/document machinery with management-client GET and UID validation. Document source reads belong to the same allowlist and lifecycle. Preserve dashboard state on document close.

Inventory entries are not remote jump targets. Do not hand inventory identities to generic cross-context resource navigation. The same applies to locally deployed inventory objects in this first version: only the six management source kinds are read. Normal generic resource browsing outside this dashboard keeps existing behavior.

Closing restores the original resource table state and restarts its watch. Command/help overlays return without losing dashboard state. A deliberate new root command or context switch discards dashboard state and work.

### 7. Compatibility and cluster-free acceptance

Use synthetic JSON fixtures matching v1.16.0 serialization, including uppercase `Features`, absent optional fields, and the distinct CAPI shapes. Public release source links below establish contracts; do not collect fixture data from live clusters.

Pure tests cover joins, collisions, UID replacements, missing summaries, feature buckets, rollout separation, and inventory. User-visible behavior tests drive `handle_key` with `Cluster::fake()` and injected results. Mock API responses cover request allowlist, pagination, cancellation, partial failures, refresh timing, and limits. Ratatui buffer tests cover narrow layouts and warnings without requiring a terminal or cluster.

No dependency or configuration change is expected. If the implementation needs one, reassess rather than silently extending this design.

## Risks / Trade-offs

- Non-atomic multi-kind reads -> preserve contradictions as incomplete evidence; refresh rather than invent consistency.
- Large management inventories -> bounded paging/concurrency; visible truncation, never silent totals.
- Broad namespace list permissions -> show partial RBAC evidence; do not escalate or read credentials.
- Unknown/future fields -> preserve raw states; baseline remains v1.16.0, not an untested promise for every release.
- Retained inventory may describe resources no longer present -> label it reported inventory and runtime health not verified.
- Polling read load -> gather once per kind, no row fan-out, no overlap; watches remain a later measured optimization.
- Narrow terminals -> prioritize identities/states; put verbose errors and inventory in scrollable detail, with full text accessible.

## Migration Plan

No data migration, deployment, or cluster change. Local implementation and testing may precede the discussion. Obtain maintainer agreement in a GitHub Discussion before opening a feature issue or PR, and link it there. Ship the additive command with docs and tests; existing commands keep their behavior. Rollback removes the new command/mode without changing Kubernetes resources or local persistent state.

## Evidence Sources

- [v1.16.0 profile status](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/api/v1beta1/status.go)
- [v1.16.0 ClusterSummary](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/api/v1beta1/clustersummary_types.go)
- [v1.16.0 ClusterConfiguration](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/api/v1beta1/clusterconfiguration_types.go)
- [v1.16.0 profile spec and labels](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/api/v1beta1/spec.go)
- [v1.16.0 controller joins and rollout tracking](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/controllers/profile_utils.go)
- [v1.16.0 provisioning completion](https://github.com/projectsveltos/addon-controller/blob/v1.16.0/controllers/utils.go)
- [v1.16.0 feature states](https://github.com/projectsveltos/libsveltos/blob/v1.16.0/api/v1beta1/common_types.go)
- [v1.16.0 SveltosCluster](https://github.com/projectsveltos/libsveltos/blob/v1.16.0/api/v1beta1/sveltoscluster_type.go)
- [v1.16.0 CAPI readiness checks](https://github.com/projectsveltos/libsveltos/blob/v1.16.0/lib/clusterproxy/clusterproxy.go)
