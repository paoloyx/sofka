# Karpenter dashboard

`:karpenter` (alias `:kpt`) opens a read-only dashboard for clusters that run
[Karpenter](https://karpenter.sh). It answers three questions: how full are my
pools, why is a node not going away, and why is a pod not scheduled. It needs
the `nodepools` and `nodeclaims` CRDs; without them sofka flashes
`Karpenter not installed`.

Data is read from NodePools, NodeClaims, NodeClasses (any provider kind,
such as `EC2NodeClass`), nodes, pods and events across all
namespaces and refreshed every 5s. The title is a path,
`karpenter › <pool> › <tab>` (the pool only while one is focused), then the
interval, the local time
of the last update (`updated 12:18:05`), and `refreshing…` while a load started
by opening the dashboard or by `r` is running. Nothing is changed in the
cluster.

## Tab line

One line under the title holds the health badge and the tabs, each with its
row count. Counts that need attention are orange, others green.

| Part                             | Meaning                                                                                                                         |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| `○ loading…`                     | first load after opening the dashboard; counts show `…` and the table `loading…`                                                |
| `● healthy`                      | no blocked disruption, no pending pods, no pool at its limit                                                                    |
| `▲ degraded`                     | a pool is above 80% of a limit, a claim is not ready, or disruption is blocked by PDB, `nominated`, `do-not-disrupt` or `other` |
| `▲ attention`                    | a pool is at its limit or unhealthy, a NodeClass is not ready, or pods are pending                                              |
| `NodeClaims (96 · 3 not ready)`  | claims, and how many are not `Ready` (shown only when above 0)                                                                  |
| `Pending pods (2)`               | pods Karpenter could not place                                                                                                  |
| `Disruption (182 · 30 blocked)`  | Disruption rows and how many are blocked; on the active tab split by cause, actionable causes first                             |
| `NodeClasses (52 · 1 not ready)` | NodeClasses, and how many are not `Ready` (shown only when above 0)                                                             |

## Tabs

The active tab is shown in brackets, for example `[NodePools (57)]`. Switch with
`tab` / `backtab` (or `l` / `h`, arrows).

| Tab          | One row per    | Columns                                                   |
| ------------ | -------------- | --------------------------------------------------------- |
| NodePools    | NodePool       | name, claims, CPU, memory, nodes, budget, mode, nodeclass |
| NodeClaims   | NodeClaim      | name, pool, type, instance, CPU, memory, node, age, state |
| Pending pods | pod            | pod, age, why                                             |
| Disruption   | node/claim/pod | object, kind, state, age, message                         |
| NodeClasses  | NodeClass      | name, kind, pools, claims, state, age                     |

- **NodePools** show usage bars against `spec.limits` (green below 80%, warning at
  80%, critical at 100%); CPU or memory without a limit shows the raw usage
  (`4 · no limit`). They also show the disruption budget (`10% (allows 1)`) and the
  capacity mix (spot / on-demand). A pool with unhealthy conditions, or whose
  NodeClass is not ready or missing, is critical.
- **NodeClaims** show the instance type, the node it runs on and its state
  (`Ready`, `Launching`, `Deleting`, `NotReady: <condition>`).
- **Pending pods** lists unschedulable pods and says why, for example
  `likely blocked: pools at limit: <pool>`. The full scheduler message is in
  the detail pane.
- **Disruption** answers "why is this node still here, and what happens to
  it". One row per object with its latest state: `DisruptionLaunching`,
  `DisruptionWaitingReadiness`, `DisruptionTerminating` (`▲`, in progress),
  `Unconsolidatable` (`○`, informational), or `blocked: <cause>` (`▲`) from
  `DisruptionBlocked` events and `karpenter.sh/do-not-disrupt` holders. Causes
  are `PDB`, `nominated` (a pod is about to land on the node),
  `do-not-disrupt` and `other`. `deleting` and `initializing` (node going away
  or not up yet) are transient, so they are `○` and do not degrade health.
  Events outlive the objects they name, so rows for nodes and claims that no
  longer exist are dropped.
- **NodeClasses** show which pools use each class, how many claims it backs,
  and its state (`Ready`, `Deleting`, `NotReady: <first failed condition>`).

The row marker shows severity: `●` healthy, `○` informational, `▲` needs
attention (colour tells warning from critical); the legend sits on the right
of the top border. The pane under the table shows details for the selected row
as `label: value` lines, with problems in orange or red:

| Tab          | Detail                                                                 |
| ------------ | ---------------------------------------------------------------------- |
| NodePools    | policy, budget, limits, capacity mix, failed conditions, broken claims |
| NodeClaims   | zone, NodeClass, node, and failed conditions or `all True`             |
| Pending pods | scheduler message                                                      |
| Disruption   | event message, plus pool, claim, node and instance when known          |
| NodeClasses  | role, instance profile, resolved AMIs/subnets/SGs, failed conditions   |

## Keys

| Key               | Action                                                                    |
| ----------------- | ------------------------------------------------------------------------- |
| `tab` / `backtab` | next / previous tab                                                       |
| `/`               | filter every column of the current tab (`enter` apply, `esc` clear)       |
| `S`               | sort picker, like resource views; the first entry is severity             |
| `I`               | reverse the sort order                                                    |
| `A`               | sort by AGE, again to reverse (NodeClaims, Pending pods, Disruption)      |
| `p`               | problems only (hide healthy and informational rows)                       |
| `x`               | show idle pools (0 claims); hidden by default, a filter always finds them |
| `enter`           | focus the pool; on a claim open its node; other rows open the resource    |
| `y` / `d`         | YAML / describe of the row's object without leaving; `esc` returns        |
| `[` / `]`         | leave the dashboard to the previous / next view in history                |
| `r`               | refresh now                                                               |
| `j`/`k`, `g`/`G`  | move, first / last row                                                    |
| `esc`, `q`        | clear filter, drop the pool focus, then close                             |
| `?`               | help                                                                      |

The sorted column header shows its direction (`CPU ↓`), like resource views.
Severity order (the picker's default, worst first) puts the arrow above the row
markers (`↓ NAME`). When a filter, `p` or hidden idle pools leave out rows,
the active tab shows visible/total (`NodePools (47/57)`). Badges show the
filters in effect: `problems only` in red while `p` is on, and
`10 idle pools hidden` in blue on NodePools until `x` shows them; both are
hidden while a filter is set,
because the filter ignores both. Sort is kept per tab. Numeric columns sort by
value, `AGE` by time, everything else as text. The bottom line lists the keys
that apply to the current tab. Keys can be remapped in the `[keys.karpenter]`
table, see [keybindings](keybindings.md#keyskarpenter).

## Drill-through

Navigation follows the Karpenter hierarchy, NodePool → NodeClaim → node:

- `enter` on a pool focuses it and opens NodeClaims. The focus (exact name,
  shown in the title path) holds across tabs: NodePools, NodeClaims and
  Disruption show only that pool's rows, NodeClasses only the class it uses. Pending pods have no pool, so that tab
  shows all of them (`(all pools)` in the title). `esc` drops the focus and
  returns to NodePools with the pool selected.
- `enter` on a claim opens its node (the claim itself until it has one); the
  node's `J` (owner) leads back to the claim.
- `enter` on a pending pod or disruption row opens the resource in the main
  table, ready for describe, logs or edit.
- `enter` on a NodeClass opens it in the main table.
- `esc` in that view returns to the dashboard with the same tab, pool, sort and
  row, and `[`/`]` back into the dashboard keep them too.
- `y` and `d` show any row's YAML or describe in place, no drill needed; `esc`
  returns to the same tab and row.

## Limits

- Read-only. There are no pool or claim actions. `y` opens the shared YAML
  view, where `e` edits like anywhere else (blocked by read-only mode).
- No cost data; prices are not read.
- Sort, idle and problems toggles are not remembered between sessions.
- `Disruption` (except `do-not-disrupt` holders) relies on events, which Kubernetes expires after
  about an hour. Only Karpenter's disruption event reasons are listed, so the
  load stays fast on busy clusters. If events cannot be listed the tab line shows
  `events unavailable`.
