## Why

`:sveltos` diagnoses one profile/cluster deployment at a time. Fleet-level questions users answer in the Sveltos dashboard and the Freelens Sveltos extension (which version runs, which add-ons are on a cluster, which charts are outdated, what failed where, in which order profiles deploy, what a dry run would change) still need generic CRD tables or another tool.

## What Changes

- Add Overview (new default), Add-ons, Errors, Dependencies, Dry run, Events, and Classifiers tabs beside Clusters and Profiles.
- Overview: Sveltos version from controller Deployment image tags in namespace `projectsveltos`, per-controller versions, and counters for clusters ready, ClusterProfiles, Profiles, failing add-ons, outdated Helm charts, dry-run reports, event sources, event triggers, and classifiers.
- Clusters: add a derived status, Kubernetes version, labels, and add-on, profile, and deployment-error counts.
- Add-ons: one row per Helm release or deployed resource kind per cluster, with version, app version, available chart update reported by addon-controller, owning profile, status, and last update.
- Errors: profile-, feature-, and Helm-release-level failures and conflicts across clusters.
- Dependencies: `dependsOn` deployment stages for ClusterProfiles and Profiles, flagging missing profiles and cycles.
- Dry run: ClusterReport actions per cluster and profile.
- Events and Classifiers: EventSource, EventTrigger, and Classifier rows with their reported matches.
- Filter: `key: value` terms combined with AND; bare terms keep substring matching.
- Read allowlist adds Deployments in `projectsveltos`, ClusterReport, Classifier, EventSource, and EventTrigger. Absent APIs read not available. The dashboard stays read-only and management-context only.

### Non-goals

Helm release resources read from managed clusters (needs kubeconfig Secrets and downstream connections), pause/resume or label edits, GitHub release checks or any other internet access, chart icons, card layouts, and column choosers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `sveltos`: more tabs and a new default tab, wider read allowlist, enriched cluster rows, add-on inventory, aggregated errors, dependency stages, dry-run reports, event and classifier rows, and structured filtering. The capability is introduced by `add-sveltos-dashboard`, which must be archived before this change.

## Impact

- Code: `src/sveltos.rs` and new pure submodules, `src/app/sveltos.rs`, `src/ui.rs`, `src/app/tests/sveltos.rs`.
- Docs: `docs/features.md`, `docs/keys.md`, and the `?` help for the new tabs and filter syntax.
- Reads stay within the existing per-gather budget (20,000 objects, 200 list requests, 30 seconds).
- No new dependency, configuration field, binary, or persistent storage.
