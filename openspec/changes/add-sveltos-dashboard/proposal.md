## Why

Sveltos deployment diagnosis requires correlating clusters, profiles, per-cluster summaries, and deployed inventory. Generic CRD tables expose individual objects but not which rollout is failing, what blocks it, or what was deployed.

## What Changes

- Add `:sveltos`, a read-only dashboard for the connected management context, compatible with Sveltos v1.16.0.
- Provide switchable Clusters and Profiles tabs; Clusters opens by default.
- Include SveltosCluster and CAPI Cluster when served, plus ClusterProfile and namespaced Profile.
- Drill from either tab into the same profile/cluster deployment detail: feature states, failure reasons, dependencies, suspension, Helm conflicts, and last applied times.
- Show deployed charts, versions, and resource identities from management-cluster inventory.
- Separate cluster readiness/connectivity, controller-reported rollout progress, and feature provisioning. Never infer workload health or current rollout completion from an old `Provisioned` status.
- Support refresh, local filtering, source-CR YAML inspection, and explicit partial-data warnings.

## Capabilities

### New Capabilities

- `sveltos`: Management-cluster-only dashboard, correlation, provisioning diagnosis, and reported deployment inventory.

### Modified Capabilities

None. Existing resource discovery, generic CRD tables, Fleet, Pulse, and GitOps views retain their behavior.

## Impact

Reuse native discovery, DynamicObject reads, generation-tagged background tasks, navigation actions, document overlays, and ratatui rendering. Integration touches application modes, palette registration/completion, message handling, lifecycle cleanup, rendering, keymap scope, and cluster-free tests. Update `docs/keys.md`, `docs/features.md`, and `?` help together.

No new service, binary, dependency, configuration field, or persistent storage is planned. Dashboard reads only the selected management context; no Secret reads, downstream connections, or external Sveltos API backend.

### Non-goals

Write actions, automatic context switching, downstream workload health checks, cluster creation/provisioning, dry-run change previews, full dependency graphs, historical analytics, multi-management-context aggregation, and feature parity with the Sveltos web dashboard.
