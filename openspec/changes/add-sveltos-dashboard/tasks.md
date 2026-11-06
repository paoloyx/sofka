## 1. Implementation prerequisites

- [x] 1.1 Allow local implementation and testing before a discussion; require maintainer agreement in a linked GitHub Discussion before opening a feature issue or PR.
- [x] 1.2 Fetch all Git refs before code changes, inspect the worktree, and preserve unrelated edits; verify fetch succeeds and record any blocker before continuing.

## 2. Evidence model and correlation

- [x] 2.1 Add `src/sveltos.rs` and register it in the crate; define source availability/freshness and typed cluster/profile keys with synthetic v1.16.0 JSON examples. Verify fixtures distinguish absent fields, explicit empty arrays, both cluster types, both profile scopes, and uppercase inventory `Features`.
- [x] 2.2 Correlate summaries using cluster spec fields and profile ownerReference/UID; join inventory through namespace/type/name labels and profile sections. Verify pure tests cover same-name collisions, replaced owners, missing labels, duplicate evidence, and unknown cluster types.
- [x] 2.3 Build profile/cluster deployment rows from observed matching targets, summaries, and profile failure reports; preserve missing-summary targets, no-longer-matching summaries, and unresolved references. Verify tests retain failure and removal evidence from both tab orientations.
- [x] 2.4 Extract SveltosCluster readiness/connectivity/pause and CAPI control-plane evidence for served v1beta1/v1beta2 shapes; verify missing data stays unknown and CAPI initialization never becomes a connectivity claim.
- [x] 2.5 Implement configured-feature detection, raw-state retention, and one-bucket-per-deployment counts with independent pause/suspension/sync-mode flags. Verify mixed states, missing configured reports, unknown states, FailedNonRetriable, removal Blocked, empty configuration, and conflicts cannot produce false success.
- [x] 2.6 Keep reported rollout lists separate from provisioning, exposing contradictions and configuration-difference evidence without computing Sveltos hashes; verify old Provisioned reports cannot imply latest rollout completion, OneTime is apply-once, and DryRun is excluded from deployment success.
- [x] 2.7 Extract feature/summary errors, dependencies, reconcile times, Helm conflicts, charts, and Local/Remote resource inventory; verify last-applied timestamps, identity fields, and inventory-unavailable versus empty states using pure tests.

## 3. Management-cluster gathering and lifecycle

- [x] 3.1 Add `src/app/sveltos.rs` with explicit API-group resolution and an allowlisted read-only gather; verify mock requests use only the six management source kinds, tolerate absent optional APIs, distinguish discovery errors, and never access Secrets or downstream clients.
- [x] 3.2 Implement 500-object pagination and fixed request/concurrency/time/object budgets; verify mocked multi-page reads and exact limits of four concurrent requests, five seconds per request, thirty seconds total, two hundred list requests, and twenty thousand objects retain partial evidence with warnings.
- [x] 3.3 Add dashboard result messages and generation/request guards in `src/store.rs` and `src/app/lifecycle.rs`; verify stale results after manual refresh, close, root navigation, and context switch are ignored.
- [x] 3.4 Implement serialized polling five seconds after gather completion plus manual cancellation/restart, source timestamps, stale retention, and persistent incomplete warnings; verify paused-time/mock tests cover no overlap, success followed by failure, partial-page failure, and recovery.

## 4. Application navigation and inspection

- [x] 4.1 Register dashboard mode/state, `:sveltos`, and palette completion/dispatch in application modules; verify `handle_key` tests open without selection, default to Clusters, reject confirmed API absence without losing the current view, and surface incomplete discovery.
- [x] 4.2 Wire the `sveltos` keymap scope using existing actions and defaults; verify binding validation, effective help, Tab/Shift-Tab, movement, paging, Enter, Esc, and q through input tests.
- [x] 4.3 Implement independent overview tab filters/selections and cluster/profile-related lists sharing deployment detail; include profile targeting/settings in its related page and an unresolved-evidence section. Verify key-driven tests cover both drill paths, missing summaries, scope collisions, preserved selections, and vanished sources.
- [x] 4.4 Add local case-insensitive substring filtering without resource-table selector grammar; verify `/`, apply, cancel, clear, empty results, and tab-specific filters through `handle_key` with no filter-triggered API calls.
- [x] 4.5 Wire explicit management source rows to existing YAML documents with API-group/UID checks and return state; verify key-driven tests cover y, document close, missing/replaced sources, and refusal to fetch inventory-object YAML.
- [x] 4.6 Preserve dashboard state through command/help overlays, restore originating table scope/filter/selection/watch on close, and disable mutating paths inside dashboard; verify key-driven tests in read and write modes, including no external process or mutation requests.

## 5. Rendering and documentation

- [x] 5.1 Render overview tables, source availability, separate rollout/provisioning columns, freshness, and persistent incomplete warnings in `src/ui.rs`; verify ratatui buffer tests for both tabs, empty/unknown/paused/DryRun states, and narrow terminals.
- [x] 5.2 Render related/deployment pages with full scrollable failure text, dependencies, Helm conflicts, reported inventory, and runtime-health-not-verified notice; verify buffer and key-driven tests expose all evidence without remote jump targets.
- [x] 5.3 Update `docs/keys.md`, `docs/features.md`, `docs/keybindings.md`, and effective `?` help together; verify documentation names the command, scope, supported baseline, bindings, substring filter, read-only boundary, budgets, and runtime-health limitation. Format touched Markdown with `oxfmt`.

## 6. Integration acceptance

- [x] 6.1 Run targeted cluster-free Sveltos model, gather, input, and render tests; verify every scenario in `specs/sveltos/spec.md` has coverage, including read allowlist, UID collisions, stale replies, incomplete inventory, and exact read budgets.
- [x] 6.2 Run `just check` and review the diff; verify formatting, locked clippy, and locked tests pass, existing dashboards/navigation remain unchanged, no dependency/version/configuration changes slipped in, and no live infrastructure command was used.
- [ ] 6.3 Before any later feature PR, link the agreed discussion and include the required AI-use disclosure; verify PR preparation remains one logical change and no PR is opened without scope agreement.
