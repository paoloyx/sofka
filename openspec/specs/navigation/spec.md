# navigation Specification

## Purpose

Defines how a user moves around sofka: drilling into and back out of resource
views, the breadcrumb stack, view history, Tab cycling, the `:` command
palette, owner and node jumps, `:find`, help scrolling, picker paging, table
cursor movement, text-input editing keys, quitting, mouse navigation and the
terminal window title.

## Requirements

### Requirement: Table cursor movement

The resource table SHALL move the cursor one row with `j`/`k` or the arrow keys, jump to the first or last row with `g`/`Home` and `G`/`End`, and move one visible page with `PgDn`/`ctrl-f` and `PgUp`/`ctrl-b`, clamping at both ends without wrapping. Moving down with no selected row MUST select the first row.

#### Scenario: Down with no selection lands on the first row

- **WHEN** the table has rows but no selected row
- **AND** the user presses `j`
- **THEN** the first row is selected

#### Scenario: Last row jump

- **WHEN** the user presses `G` in a table with 20 rows
- **THEN** the cursor is on row 20

#### Scenario: Page down clamps at the end

- **WHEN** the cursor is two rows above the last row
- **AND** the user presses `PgDn`
- **THEN** the cursor is on the last row

### Requirement: Welcome flash

On startup sofka SHALL show the status message "Welcome to sofka — ':' resource · enter drill · d describe · l logs · ? help".

#### Scenario: Fresh start

- **WHEN** sofka starts and no other status message has been set
- **THEN** the status line shows the welcome message

### Requirement: Namespace drill

Pressing `enter` on a namespaces row SHALL open the Pods list in that namespace through the normal namespace selection, clearing the filter, label selector and owner scope, so the namespace is remembered for the context and recorded in view history. The Namespaces list MUST be kept as a breadcrumb frame.

#### Scenario: Enter on a namespace opens its pods

- **WHEN** the Namespaces list is shown with `team-a` selected
- **AND** the user presses `enter`
- **THEN** the Pods list for namespace `team-a` is shown
- **AND** pressing `esc` returns to the Namespaces list with its filter and selected row restored

### Requirement: Node drill

Pressing `enter` on a nodes row SHALL open pods across all namespaces filtered server-side by `spec.nodeName=<node>`, with the scope label `node/<name>`.

#### Scenario: Enter on a node

- **WHEN** the Nodes list is shown with `worker-1` selected
- **AND** the user presses `enter`
- **THEN** pods scheduled on `worker-1` from every namespace are listed
- **AND** the scope label reads `node/worker-1`

### Requirement: Workload drill to pods

Pressing `enter` on a deployments, statefulsets, daemonsets, replicasets or jobs row SHALL open the pods matched by the object's selector (`matchLabels`, with `matchExpressions` also applied) in the object's namespace. An object without a pod selector MUST flash "no pod selector on this object" and stay on the list.

#### Scenario: Deployment drill

- **WHEN** a deployment with selector `app=web` is selected
- **AND** the user presses `enter`
- **THEN** pods with label `app=web` in that deployment's namespace are listed

#### Scenario: Missing selector

- **WHEN** a selected job has no pod selector
- **AND** the user presses `enter`
- **THEN** the status line shows "no pod selector on this object"
- **AND** the job list stays open

### Requirement: Service drill to pods

Pressing `enter` on a services row SHALL open the pods matched by `spec.selector` with the scope label `svc/<name>`. A service without a selector MUST flash "service has no selector".

#### Scenario: Service with selector

- **WHEN** service `web` with selector `app=web` is selected
- **AND** the user presses `enter`
- **THEN** matching pods are listed with the scope label `svc/web`

#### Scenario: Service without selector

- **WHEN** a service without `spec.selector` is selected
- **AND** the user presses `enter`
- **THEN** the status line shows "service has no selector"

### Requirement: Pod drill to containers

Pressing `enter` on a pods row SHALL open the containers view of that pod.

#### Scenario: Enter on a pod

- **WHEN** a pod is selected
- **AND** the user presses `enter`
- **THEN** the containers view for that pod opens

### Requirement: CronJob drill to jobs

Pressing `enter` on a cronjobs row SHALL open the jobs owned by that CronJob with the scope label `cronjob/<name>` and flash "↳ jobs of <name>". When the jobs kind is not available it MUST flash "jobs kind unavailable".

#### Scenario: Enter on a CronJob

- **WHEN** CronJob `nightly` is selected
- **AND** the user presses `enter`
- **THEN** only jobs owned by `nightly` are listed
- **AND** the status line shows "↳ jobs of nightly"

### Requirement: CRD drill to custom resources

Pressing `enter` on a customresourcedefinitions row SHALL open that CRD's custom resources across all namespaces with the scope label `crd/<crdname>` and flash "↳ <plural>". A CRD without a plural name MUST flash "CRD has no plural name"; a CRD whose resource cannot be resolved MUST flash "could not resolve CRD's resource (no served version?)".

#### Scenario: Enter on a CRD

- **WHEN** CRD `widgets.example.com` is selected
- **AND** the user presses `enter`
- **THEN** all `widgets` objects from every namespace are listed
- **AND** the status line shows "↳ widgets"

#### Scenario: Unresolvable CRD

- **WHEN** the selected CRD has no served version known to discovery
- **AND** the user presses `enter`
- **THEN** the status line shows "could not resolve CRD's resource (no served version?)"

### Requirement: Cluster API MachineDeployment drill

Pressing `enter` on a `machinedeployments` row from API group `cluster.x-k8s.io` SHALL open `machines.cluster.x-k8s.io` matched by the object's `matchLabels` selector with the scope label `machinedeployment/<name>`. Without a selector it MUST flash "no machine selector on this object". A `machinedeployments` kind from another group MUST fall through to the configured drill or YAML.

#### Scenario: CAPI MachineDeployment

- **WHEN** a Cluster API MachineDeployment `md-0` with a selector is selected
- **AND** the user presses `enter`
- **THEN** the matching Machines are listed with the scope label `machinedeployment/md-0`

### Requirement: Built-in drills for RBAC and Helm kinds

Pressing `enter` on roles, clusterroles, rolebindings or clusterrolebindings from group `rbac.authorization.k8s.io` SHALL open the rules view, and on `helm`, `helmhistory` or `helmreleases` rows SHALL open the corresponding Helm view; the content of those views is defined by their own capabilities.

#### Scenario: Enter on a role

- **WHEN** an RBAC role is selected
- **AND** the user presses `enter`
- **THEN** the rules view for that role opens

### Requirement: Configured and fallback drills

For kinds without a built-in drill, `enter` SHALL use the kind's `[views."…"].drill` when configured, otherwise jump to the node named by `[views."…"].node` when configured, otherwise open the YAML detail view of the row.

#### Scenario: No drill configured

- **WHEN** a configmaps row is selected and no view drill or node pointer is configured
- **AND** the user presses `enter`
- **THEN** the YAML of the selected configmap opens

#### Scenario: Configured drill

- **WHEN** `[views."widgets.example.com"].drill` targets another kind
- **AND** the user presses `enter` on a widget row
- **THEN** the target kind opens scoped to that row
- **AND** the status line shows "↳ drilled into <plural>"

### Requirement: Configured drill on built-in kinds is ignored

A `[views."…"].drill` configured for a kind with a built-in drill (roles, clusterroles, rolebindings, clusterrolebindings, namespaces, nodes, deployments, statefulsets, daemonsets, replicasets, jobs, cronjobs, services, pods, customresourcedefinitions, helm, helmhistory, helmreleases) SHALL be ignored with the config warning `views."<key>": drill is ignored — `enter` on <plural> has a built-in drill-down that config doesn't replace`.

#### Scenario: Drill on deployments in config

- **WHEN** the config sets a `drill` for `deployments`
- **THEN** a config warning reports that the drill is ignored
- **AND** `enter` on a deployment still opens its pods

### Requirement: Missing drill target kind

A drill whose target kind is not served by the cluster SHALL flash "<kind> kind unavailable" and keep the current view.

#### Scenario: Target kind missing

- **WHEN** a configured drill targets a kind the cluster does not serve
- **AND** the user presses `enter`
- **THEN** the status line shows "<kind> kind unavailable"
- **AND** the current list stays open

### Requirement: Drill view state

A drill SHALL push the current view onto the breadcrumb stack, keep only the `-l` and `-f` selector terms of the current filter, reset the sort, ignore the namespace when the target kind is cluster-scoped, and place the cursor on the first row.

#### Scenario: Text filter dropped on drill

- **WHEN** the deployments filter is `web -l tier=front`
- **AND** the user drills into a deployment
- **THEN** the pods view keeps `-l tier=front` and drops `web`
- **AND** the cursor is on the first row

### Requirement: Esc order in the table

`esc` in the resource table SHALL do the first applicable step: clear row marks; else clear the filter and widen any server-side selector it set; else pop the breadcrumb stack; else return to Argo CD when the view was opened from there.

#### Scenario: Marks cleared before filter

- **WHEN** rows are marked and a filter is active
- **AND** the user presses `esc`
- **THEN** the marks are cleared
- **AND** the filter is still active

#### Scenario: Filter cleared before popping

- **WHEN** no rows are marked, a filter is active and a breadcrumb frame exists
- **AND** the user presses `esc`
- **THEN** the filter is cleared and the view stays the same

### Requirement: Breadcrumb restore

Popping a breadcrumb frame SHALL restore the previous kind, namespace, server-side selectors, filter, scope label and selected row, and reset the sort.

#### Scenario: Back from a drill

- **WHEN** the user drilled from row 4 of a filtered deployments list
- **AND** presses `esc` in the pods view with no marks and no filter
- **THEN** the deployments list returns with the same filter
- **AND** row 4 is selected

### Requirement: Root view switches

Switching to a kind through the palette, a resource query or view history SHALL clear the breadcrumb stack, filter and sort, move the cursor to the first row and flash "Viewing <Title>" or "Viewing <Title> in <namespace label>". An unknown kind MUST flash the error "No resource matches '<input>'".

#### Scenario: Switching with the palette

- **WHEN** the user is inside a drilled pods view
- **AND** runs `:deploy`
- **THEN** the Deployments list opens with an empty breadcrumb stack
- **AND** the status line starts with "Viewing Deployments"

#### Scenario: Unknown kind

- **WHEN** the user runs `:nosuchthing`
- **THEN** the status line shows the error "No resource matches 'nosuchthing'"

### Requirement: View history recording

sofka SHALL record each root view (kind, namespace and filter) in a history of at most 50 entries, skipping an entry equal to the current one and truncating forward entries when a new one is added. Drills, owner jumps, node jumps and `:helm` MUST NOT add history entries.

#### Scenario: Drill not recorded

- **WHEN** the user opens Pods, then Deployments, then drills into one deployment's pods
- **AND** presses `[`
- **THEN** the Pods root view returns, not Deployments

### Requirement: View history navigation

`[` SHALL go back and `]` forward through view history, flashing "history i/n: <Title> in <namespace>". At the ends it MUST flash "already at oldest view" or "already at newest view"; an entry whose kind no longer resolves MUST flash "cannot resolve '<kind>' anymore".

#### Scenario: Back and forward

- **WHEN** the user viewed Pods then Services
- **AND** presses `[`
- **THEN** the Pods view returns with its namespace and filter
- **AND** pressing `]` returns to Services

#### Scenario: Oldest end

- **WHEN** the current view is the oldest history entry
- **AND** the user presses `[`
- **THEN** the status line shows "already at oldest view"

### Requirement: Tab view cycling

With no active workspace, `Tab` SHALL switch forward and `Shift-Tab` backward through pods, services, deployments, statefulsets, daemonsets, secrets, configmaps, ingresses and persistentvolumeclaims in the current namespace, wrapping and skipping kinds the cluster does not serve. With an active workspace it MUST cycle the workspace's views instead.

#### Scenario: Tab from pods

- **WHEN** the Pods list is shown and no workspace is active
- **AND** the user presses `Tab`
- **THEN** the Services list opens in the same namespace

#### Scenario: Shift-Tab wraps

- **WHEN** the Pods list is shown and no workspace is active
- **AND** the user presses `Shift-Tab`
- **THEN** the persistentvolumeclaims list opens

### Requirement: Owner jump

`J` (shift-j) SHALL open the first owner reference of the selected object as a name-filtered list of the owner's kind, labelled `owner of <child>`, clearing the filter and keeping the current view as a breadcrumb frame. Without an owner it MUST flash "no owner reference"; with an unknown owner kind it MUST flash "owner kind <kind> unresolved".

#### Scenario: Pod owned by a ReplicaSet

- **WHEN** a pod owned by ReplicaSet `web-abc` is selected
- **AND** the user presses `J`
- **THEN** the ReplicaSets list shows `web-abc`
- **AND** `esc` returns to the pods list

#### Scenario: Unowned object

- **WHEN** a pod without owner references is selected
- **AND** the user presses `J`
- **THEN** the status line shows "no owner reference"

### Requirement: Node jump

`o` SHALL open the Nodes list filtered to the node named by the selected row (pods' `spec.nodeName`, or the kind's `[views."…"].node` pointer) with the scope label `node of <target>`. A kind with no node pointer MUST flash `this kind names no node (see views."…".node)`; an empty value MUST flash "<kind>/<name> has no node assigned"; a non-string value MUST flash "<pointer> on <target> is not a node name".

#### Scenario: Pod on a node

- **WHEN** a pod scheduled on `worker-1` is selected
- **AND** the user presses `o`
- **THEN** the Nodes list shows only `worker-1`

#### Scenario: Pending pod

- **WHEN** a pending pod without a node is selected
- **AND** the user presses `o`
- **THEN** the status line shows "pods/<name> has no node assigned"

### Requirement: Opening and closing the palette

`:` SHALL open the command palette with an empty command from any view that binds it, and `esc` in the palette MUST return to the mode it was opened from without running anything.

#### Scenario: Esc returns to the origin

- **WHEN** the user presses `:` in the YAML view
- **AND** presses `esc`
- **THEN** the YAML view is shown again

### Requirement: Palette suggestions without a query

With an empty palette query sofka SHALL list configured bookmarks and workspaces first, then the resource kinds the user is allowed to list, alphabetically; built-in commands MUST NOT be listed until the user types.

#### Scenario: Empty palette

- **WHEN** the user opens the palette with one bookmark configured
- **THEN** the bookmark is the first suggestion
- **AND** resource kinds follow in alphabetical order

### Requirement: Palette suggestion ranking

With a query, the palette SHALL fuzzy-match commands, resource kinds and aliases, plugins for the current kind, bookmarks and workspaces, ranking an exact alias or kind match highest, boosting bookmarks and workspaces, then ordering by score, shorter label and alphabetically, and showing at most 100 suggestions.

#### Scenario: Exact alias first

- **WHEN** the user types `po`
- **THEN** the pods kind is the first suggestion

### Requirement: Palette navigation and completion

In the palette `Down`/`Tab` and `Up`/`Shift-Tab` SHALL move through suggestions with wrap-around, `Right` MUST complete the word being typed from the highlighted suggestion, and `enter` MUST run the highlighted suggestion.

#### Scenario: Wrap at the end

- **WHEN** the last suggestion is highlighted
- **AND** the user presses `Down`
- **THEN** the first suggestion is highlighted

### Requirement: Typed command wins over suggestion

When the user has not moved through the suggestions, `enter` SHALL run an exactly typed built-in command, except that a CRD-owned name matching the typed text MUST take precedence over the built-in command.

#### Scenario: Exact command typed

- **WHEN** the user types `ctx` without navigating the suggestions
- **AND** presses `enter`
- **THEN** the context picker opens

### Requirement: Palette argument completion

After `:<kind> ` the palette SHALL complete namespace names with `all` first; after `:ctx ` it MUST complete context names; after `@` in a resource command it MUST complete context names, and moving through them MUST write `<kind> @<context>` into the command.

#### Scenario: Namespace completion

- **WHEN** the user types `:pods ` and namespaces `dev` and `prod` exist
- **THEN** the suggestions start with `all`, followed by matching namespaces

### Requirement: Resource queries

A palette command containing ` /`, `@`, `-n`, `--namespace` or `--context` SHALL be parsed as a resource query that sets kind, namespace, context and filter together. A parse error MUST flash "query: <error>".

#### Scenario: Kind with namespace and filter

- **WHEN** the user runs `:pods -n prod /web`
- **THEN** the Pods list in `prod` opens filtered by `web`

### Requirement: Context in a resource query

A resource query whose `@context` differs from the current context, or names the current context while it is disconnected, SHALL switch to that context first and apply the rest of the query once it is connected.

#### Scenario: Query on another context

- **WHEN** the current context is `dev`
- **AND** the user runs `:deploy @prod`
- **THEN** sofka switches to `prod`
- **AND** shows Deployments there after connecting

### Requirement: Palette built-in commands

The palette SHALL run the built-in commands and aliases listed in `docs/keys.md`, including `:ctx`, `:ns`, `:find`/`:fd`, `:mouse`, `:help` and `:q`/`:quit`/`:q!`, and MUST treat any other word as a resource kind, alias, bookmark, workspace or plugin name.

#### Scenario: Quit from the palette

- **WHEN** the user runs `:q`
- **THEN** sofka exits

### Requirement: Quit

`ctrl-c` SHALL quit sofka from every view, and `q` MUST quit from the resource table.

#### Scenario: ctrl-c in a picker

- **WHEN** the namespace picker is open
- **AND** the user presses `ctrl-c`
- **THEN** sofka exits

### Requirement: Find across kinds

`:find <text>` (alias `:fd`) SHALL fuzzy-search object names in all namespaces across pods, deployments, statefulsets, daemonsets, services, configmaps, secrets, ingresses, jobs, cronjobs, persistentvolumeclaims, nodes, namespaces, kustomizations and helmreleases, showing at most 200 hits best-first in a picker. Failed kinds MUST be reported as "<n> kind(s) could not be listed".

#### Scenario: Find a name

- **WHEN** the user runs `:find web`
- **THEN** a picker lists objects whose names match `web`, best match first

#### Scenario: Missing text

- **WHEN** the user runs `:find` with no text
- **THEN** the status line shows "usage: :find <text>"

### Requirement: Find without a connection

`:find` SHALL refuse to run while disconnected and flash "not connected to a cluster".

#### Scenario: Disconnected

- **WHEN** no cluster is connected
- **AND** the user runs `:find web`
- **THEN** the status line shows "not connected to a cluster"

### Requirement: Find picker navigation

In the find picker `j`/`k` SHALL move, `g`/`G` MUST jump to the first or last hit, `enter` MUST open the hit's kind filtered to that object in its namespace as a breadcrumb drill, and `esc` or `q` MUST return to the table.

#### Scenario: Open a hit

- **WHEN** the find picker highlights deployment `web` in `prod`
- **AND** the user presses `enter`
- **THEN** the Deployments list in `prod` shows only `web`
- **AND** `esc` returns to the previous table view

### Requirement: Help view scrolling

`?` SHALL open the help view with no filter at the top. `j`/`k`/arrows MUST scroll one line, `PgDn`/`space`/`ctrl-f` and `PgUp`/`ctrl-b` MUST scroll one page, and `g`/`Home` and `G`/`End` MUST jump to the top or bottom.

#### Scenario: Page down in help

- **WHEN** the help view is open at the top
- **AND** the user presses `space`
- **THEN** the help scrolls down one page

### Requirement: Help filter and close

In the help view `/` SHALL start a filter and reset the scroll; `esc` MUST clear an active filter first and close the help otherwise; `q` and `?` MUST close the help.

#### Scenario: Esc with a filter

- **WHEN** the help view has the filter `logs`
- **AND** the user presses `esc`
- **THEN** the filter is cleared and the help stays open

### Requirement: Picker movement

List pickers SHALL move with `Up`/`Down` clamped at the first and last item without wrapping, and `PgUp`/`PgDn` MUST move by the number of visible rows.

#### Scenario: Up at the top

- **WHEN** the first item of a picker is highlighted
- **AND** the user presses `Up`
- **THEN** the first item stays highlighted

### Requirement: Text input editing keys

In every text input (palette, filters, prompts, picker filters) `Backspace`/`ctrl-h` SHALL delete one character, `ctrl-w`/`alt-backspace`/`ctrl-backspace` MUST delete trailing whitespace and then the previous word, and `ctrl-u` MUST clear the line.

#### Scenario: Delete a word

- **WHEN** the palette contains `pods -n prod`
- **AND** the user presses `ctrl-w`
- **THEN** the palette contains `pods -n `

#### Scenario: Clear the line

- **WHEN** the filter input contains `web -l app=x`
- **AND** the user presses `ctrl-u`
- **THEN** the filter input is empty

### Requirement: Mouse capture setting

Mouse capture SHALL be off unless `mouse = true` is configured, and `:mouse` MUST toggle it for the session, flashing "mouse capture on" or "mouse capture off: drag to select text".

#### Scenario: Toggle mouse

- **WHEN** mouse capture is off
- **AND** the user runs `:mouse`
- **THEN** the status line shows "mouse capture on"

### Requirement: Mouse release in document views

While mouse capture is enabled, sofka SHALL release it in the YAML/describe, diff, events, logs and help views and their filter inputs, and while the plugin activity overlay is shown, so the terminal's native text selection works there.

#### Scenario: Logs release the mouse

- **WHEN** mouse capture is enabled
- **AND** the user opens logs
- **THEN** mouse capture is released until the user leaves the logs view

### Requirement: Mouse clicks in the table

With mouse capture on, a left click on a table row SHALL select that row and end any Shift-arrow range, and a click on a column header MUST sort by that column ascending, or flip the direction if it is already the sort column, flashing "sort by <column> ↑ asc" or "sort by <column> ↓ desc".

#### Scenario: Click a header twice

- **WHEN** the user clicks the AGE header
- **AND** clicks it again
- **THEN** the table is sorted by AGE descending
- **AND** the status line shows "sort by AGE ↓ desc"

### Requirement: Mouse wheel scrolling

With mouse capture on, each wheel event SHALL perform the current view's own up or down action `mouse_scroll_lines` times (default 3; `0` becomes 1 and values above 100 are capped, each with a config warning), stopping early if the view changes.

#### Scenario: Default wheel step

- **WHEN** the table cursor is on row 1 and `mouse_scroll_lines` is unset
- **AND** the user scrolls the wheel down once
- **THEN** the cursor is on row 4

### Requirement: Split wheel sequence repair

While mouse capture is released, sofka SHALL reassemble a cursor-key escape sequence split after its `ESC` byte back into the arrow key, holding a bare `Esc` for about 15 ms, so fast scrolling never acts as an `esc` press.

#### Scenario: Split sequence in logs

- **WHEN** the logs view receives `ESC` followed by `[B` in a separate read
- **THEN** the logs scroll down one line
- **AND** the logs view stays open

### Requirement: Terminal title

sofka SHALL set the terminal title to "sofka: <context>/<namespace or all>" with control characters removed, write it only when it changes, and clear it on exit. `terminal_title = false` MUST disable it, resolved per context.

#### Scenario: Namespace change updates the title

- **WHEN** the context is `dev` and the namespace changes to `prod`
- **THEN** the terminal title becomes "sofka: dev/prod"

#### Scenario: Title disabled

- **WHEN** `terminal_title = false` is configured
- **THEN** sofka does not set the terminal title
