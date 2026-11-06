## Context

See `proposal.md` for motivation and `specs/sveltos/spec.md` for the behaviour contract. This change builds on `add-sveltos-dashboard`, which must be archived first.

Current state:

- `src/sveltos.rs` (about 1,900 lines) holds the pure evidence model: source reads, cluster/profile keys, deployments, provisioning buckets, inventory.
- `src/app/sveltos.rs` drives it. `State` keeps a page stack whose root is one of two tabs, swapped with `inactive_tab`. Each page builds `(Row, Vec<String>)` entries. The filter is a case-insensitive substring over the joined cells.
- `gather_source` lists each allowlisted kind cluster-wide with `Api::all_with`, under one shared budget (4 concurrent requests, 5s per request, 30s per gather, 200 list requests, 20,000 objects).
- `draw_sveltos` in `src/ui.rs` renders a header, an optional evidence panel, and one table. Pulse already has tile helpers (`gauge_tile`, `counts_tile`).

The Freelens Sveltos extension (`freelensapp/freelens-sveltos-extension`) was read for behaviour, not code. Its rules were re-expressed against Sveltos v1.16.0 fields.

## Goals / Non-Goals

**Goals:** add the fleet-level tabs without a second gather loop, a second model, or new dependencies. Keep new derivations pure and testable without a cluster. Keep the identity, partial-evidence, and read-only guarantees of the existing dashboard.

**Non-Goals:** reproducing the Freelens layout (cards, column chooser, drag-resizable columns, chart icons); a graphical dependency graph; any managed-cluster, Secret, or internet read.

## Decisions

### 1. New logic lives in pure submodules of `sveltos`

Add `src/sveltos/query.rs`, `src/sveltos/addons.rs`, `src/sveltos/deps.rs`, and `src/sveltos/extras.rs` as children of `src/sveltos.rs`:

- `query`: parses and matches the filter.
- `addons`: add-on rows, update detection, and deployment errors.
- `deps`: dependency stages, missing profiles, and cycles.
- `extras`: Sveltos version detection, plus ClusterReport, Classifier, EventSource, and EventTrigger rows.

Each module takes JSON values or small plain structs and returns plain rows. It depends only on `serde_json` and `std`, so it can be written and unit-tested in isolation. `src/app/sveltos.rs` maps the existing model into these inputs.

Alternative: growing `src/sveltos.rs` further. Rejected because it is already the largest Sveltos file, and separate files let the derivations be built in parallel without merge conflicts.

### 2. Tabs are an ordered list with per-tab frames

Replace `inactive_tab` with one root `PageFrame` per tab, in this order: Overview, Clusters, Profiles, Add-ons, Errors, Dependencies, Dry run, Events, Classifiers. Overview is selected on each fresh open. Tab moves forward and Shift-Tab moves back, both wrapping, and only at root level. Each tab keeps its own selection and filter while the dashboard stays open. The heading line lists all tabs and marks the active one, wrapping on narrow terminals.

Alternative: one palette command per tab (`:sveltos-addons`, ...). Rejected: more commands, help rows, and docs to keep in sync, and Freelens shows them as tabs of one page.

### 3. Reads: four more Sveltos kinds and controller Deployments in one namespace

Extend the allowlist, resolved by discovery and listed cluster-wide like the existing kinds:

| Kind          | Group                    | Scope      |
| ------------- | ------------------------ | ---------- |
| ClusterReport | config.projectsveltos.io | Namespaced |
| Classifier    | lib.projectsveltos.io    | Cluster    |
| EventSource   | lib.projectsveltos.io    | Cluster    |
| EventTrigger  | lib.projectsveltos.io    | Cluster    |

Deployments (`apps`) are listed only in namespace `projectsveltos`, through the same budget and the same source states (available, absent, incomplete, failed). If no Sveltos controller is found there, the version reads unknown.

Alternative: Freelens falls back to listing Deployments cluster-wide. Rejected: that reads every Deployment in the management cluster for one header line and consumes the object budget. Installations in another namespace read version unknown.

### 4. Add-ons and errors reuse gathered evidence

Add-on rows come from ClusterSummary status (Helm release summaries, feature summaries, deployed kinds) joined with ClusterConfiguration inventory (chart app version, last applied time), keyed by the existing identity-safe deployment join. No new read is needed.

The update column uses only the addon-controller's `latestVersion` and `latestPatchVersion` fields. sofka never queries chart repositories.

Error rows are built from the same deployment evidence. Enter on an add-on or error row opens the existing deployment detail for that profile and cluster, so every path still converges on one detail page.

### 5. One filter grammar for every Sveltos list

`query` parses comma-separated terms combined with AND:

- `key: value` matches the column whose heading equals the key, ignoring case. On Clusters it also matches a label of that key.
- A bare term keeps the existing substring match over displayed cells.

Columns are therefore filter keys, and drill-down pages share the grammar. `/` editing, Enter, and Esc keep their current behaviour.

Alternative: reusing the resource table's structured filter grammar. Rejected, as in the base design: dashboard rows are synthetic and do not carry Kubernetes field paths.

### 6. Dependencies are rows, not a drawing

`deps` assigns each profile a stage: 1 for profiles with no dependencies, otherwise one more than the highest stage among its dependencies. Profiles in cycles are not staged. `dependsOn` names resolve to ClusterProfiles for a ClusterProfile, and to Profiles in the same namespace for a Profile.

The tab shows one row per profile, sorted by scope, stage, and name, plus rows for missing dependencies. Each row has its stage, its dependencies, and its dependants. Enter on a profile row opens the existing profile page.

Alternative: an ASCII graph layout. Rejected: it conflicts with filtering, selection, and narrow terminals, and the stage order carries the deployment-order information.

### 7. Detail for new kinds uses the evidence panel and source inspection

Enter on a Dry run, Event, or Classifier row opens a detail page that reuses the evidence panel, listing the parsed report or status fields. `y` inspects the source object through the existing management-cluster GET with UID validation. This extends the source-row machinery from four source kinds to eight.

### 8. Overview reuses Pulse tile rendering

The Overview tab renders the version line and controller versions, then a grid of counter tiles in the Pulse style, followed by a table of controllers and versions. Tiles wrap into more rows on narrow terminals. A tile whose source is absent reads `not available`; one whose read failed reads `unreadable` and joins the incomplete-data banner.

## Risks / Trade-offs

- [Five more reads per gather reach the budget sooner on large fleets] → The existing incomplete banner names the limit reached, and collected evidence stays usable.
- [Nine tabs are cramped on narrow terminals] → The tab line wraps and the title names the active tab.
- [The addon-controller update check may be disabled or not yet run] → The Update column reads up to date only for Helm releases Sveltos manages, and absent version evidence reads unknown, never newer.
- [Freelens behaviour can drift from Sveltos releases] → Fields are pinned to v1.16.0, and unknown states are shown raw, as in the base design.
- [Deployment reads need RBAC on `apps` in `projectsveltos`] → A denied read marks the version unreadable, and other tabs keep working.

## Migration Plan

Archive `add-sveltos-dashboard` before archiving this change, so the `sveltos` capability exists in the main specs. No data migration. Roll back by reverting the code; there is no persisted state or configuration.
