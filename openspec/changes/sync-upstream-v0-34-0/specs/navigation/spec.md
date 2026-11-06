## ADDED Requirements

### Requirement: Namespace resources drill

Pressing `R` on a namespaces row SHALL open the `:ns-resources` view for that namespace through the normal namespace selection, keeping the Namespaces list as a breadcrumb frame. On any other kind, when no bookmark, workspace or plugin is bound to the key, `R` MUST keep the current view and flash the warning "R lists resources from a namespace row; elsewhere use :ns-resources". Pressing `enter` on a `:ns-resources` row SHALL open that kind's own list, at the API version that was counted, in the same namespace as a breadcrumb drill and flash "↳ <plural> in <namespace label>".

#### Scenario: R on a namespace

- **WHEN** the Namespaces list is shown with `prod` selected
- **AND** the user presses `R`
- **THEN** the `:ns-resources` view for `prod` opens
- **AND** `esc` returns to the Namespaces list, where `enter` still opens the pods of `prod`

#### Scenario: R on another kind

- **WHEN** the Pods list is shown and no bookmark, workspace or plugin is bound to `R`
- **AND** the user presses `R`
- **THEN** the Pods list stays open
- **AND** the status line shows the warning "R lists resources from a namespace row; elsewhere use :ns-resources"

#### Scenario: Enter on a counted kind

- **WHEN** the `:ns-resources` view for `prod` lists `deployments.apps`
- **AND** the user presses `enter` on that row
- **THEN** the Deployments list in `prod` opens
- **AND** `esc` returns to the `:ns-resources` view for `prod`, which counts again

## MODIFIED Requirements

### Requirement: Configured drill on built-in kinds is ignored

A `[views."…"].drill` configured for a kind with a built-in drill (roles, clusterroles, rolebindings, clusterrolebindings, namespaces, nodes, deployments, statefulsets, daemonsets, replicasets, jobs, cronjobs, services, pods, customresourcedefinitions, helm, helmhistory, helmreleases, rollouthistory, ns-resources) SHALL be ignored with the config warning `views."<key>": drill is ignored — `enter` on <plural> has a built-in drill-down that config doesn't replace`. The `ns-resources` entry MUST apply only to the built-in view, keyed `ns-resources` or `v1/ns-resources`, so a custom resource with that plural configured under its own API group keeps its drill.

#### Scenario: Drill on deployments in config

- **WHEN** the config sets a `drill` for `deployments`
- **THEN** a config warning reports that the drill is ignored
- **AND** `enter` on a deployment still opens its pods

#### Scenario: Custom resource sharing the ns-resources plural

- **WHEN** the config sets a `drill` for `example.com/ns-resources`
- **THEN** no config warning reports that the drill is ignored

### Requirement: Root view switches

Switching to a kind through the palette, a resource query or view history SHALL clear the breadcrumb stack, filter and sort, move the cursor to the first row and flash "Viewing <Title>" or "Viewing <Title> in <namespace label>". `:workloads [namespace]` (also `:workload` and `:wk`) and `:ns-resources [namespace]` SHALL open their views as root views in the same way and record them in view history. `:workloads` MUST flash "Viewing workloads", or "Viewing workloads in <namespace label>" when a namespace is given, and `:ns-resources` MUST flash "Viewing resources in <namespace label>", using the current namespace when none is given. An unknown kind MUST flash the error "No resource matches '<input>'".

#### Scenario: Switching with the palette

- **WHEN** the user is inside a drilled pods view
- **AND** runs `:deploy`
- **THEN** the Deployments list opens with an empty breadcrumb stack
- **AND** the status line starts with "Viewing Deployments"

#### Scenario: Unknown kind

- **WHEN** the user runs `:nosuchthing`
- **THEN** the status line shows the error "No resource matches 'nosuchthing'"

#### Scenario: Workloads in a namespace

- **WHEN** the user is inside a drilled pods view
- **AND** runs `:wk prod`
- **THEN** the workloads view for `prod` opens with an empty breadcrumb stack
- **AND** the status line shows "Viewing workloads in prod"

#### Scenario: Namespace resources in the current namespace

- **WHEN** the current namespace is `staging`
- **AND** the user runs `:ns-resources`
- **THEN** the `:ns-resources` view for `staging` opens
- **AND** the status line shows "Viewing resources in staging"

#### Scenario: Workloads back through history

- **WHEN** the user opened `:wk` and then `:pods`
- **AND** presses `[`
- **THEN** the workloads view returns in the same namespace

### Requirement: Palette argument completion

After `:<kind> ` the palette SHALL complete namespace names with `all` first, and after `:workloads `, `:workload `, `:wk ` or `:ns-resources ` it MUST complete namespace names the same way for the second word; after `:ctx ` it MUST complete context names; after `@` in a resource command it MUST complete context names, and moving through them MUST write `<kind> @<context>` into the command. Every time the palette opens it SHALL fetch the namespace list again, and when the list arrives the namespace suggestions MUST refresh unless the user has already moved through them. A list from an older fetch MUST NOT replace a newer list that has already arrived.

#### Scenario: Namespace completion

- **WHEN** the user types `:pods ` and namespaces `dev` and `prod` exist
- **THEN** the suggestions start with `all`, followed by matching namespaces

#### Scenario: Namespace created after launch

- **WHEN** the palette was opened once and namespace `fresh` was created afterwards
- **AND** the user opens the palette again and types `:pods fre`
- **THEN** once the new namespace list arrives, the first suggestion is the namespace `fresh`

#### Scenario: Workloads namespace completion

- **WHEN** namespace `social` exists
- **AND** the user types `:wk soc`
- **THEN** the first suggestion is the namespace `social`
- **AND** `enter` opens the workloads view in `social`

### Requirement: Help view scrolling

`?` SHALL open the help view with no filter, scrolled to the `Keys:` section of the view it was opened from, or at the top when that view has no section. `j`/`k`/arrows MUST scroll one line, `PgDn`/`space`/`ctrl-f` and `PgUp`/`ctrl-b` MUST scroll one page, `g`/`Home` and `G`/`End` MUST jump to the top or bottom, and `]` and `[` MUST jump to the next or previous section heading.

#### Scenario: Page down in help

- **WHEN** the help view is open at the top
- **AND** the user presses `space`
- **THEN** the help scrolls down one page

#### Scenario: Help opens on the current view's section

- **WHEN** the YAML view is open
- **AND** the user presses `?`
- **THEN** the help view opens with the `Keys: detail` heading as its first visible line

#### Scenario: Jump between sections

- **WHEN** the help view shows the `Keys: detail` heading as its first visible line
- **AND** the user presses `]`
- **THEN** the next section heading becomes the first visible line
- **AND** pressing `[` brings the `Keys: detail` heading back to the first visible line

### Requirement: Text input editing keys

In every text input (palette, filters, prompts, picker filters) `Backspace`/`ctrl-h` SHALL delete one character, `ctrl-w`/`alt-backspace`/`ctrl-backspace` MUST delete trailing whitespace and then the previous word, and `ctrl-u` MUST clear the line. A character chord that holds only `ctrl` or only `alt` and is not bound to an action MUST be ignored instead of typed, and MUST NOT start a picker filter or move a picker selection; a chord holding both `ctrl` and `alt` SHALL still type its character, since some terminals report AltGr characters that way.

#### Scenario: Delete a word

- **WHEN** the palette contains `pods -n prod`
- **AND** the user presses `ctrl-w`
- **THEN** the palette contains `pods -n `

#### Scenario: Clear the line

- **WHEN** the filter input contains `web -l app=x`
- **AND** the user presses `ctrl-u`
- **THEN** the filter input is empty

#### Scenario: Unbound Ctrl and Alt chords

- **WHEN** the palette is open and `ctrl-x` and `alt-x` are not bound
- **AND** the user presses `ctrl-x`, `alt-x` and then `a`
- **THEN** the palette contains `a`

#### Scenario: Unbound chord in the context picker

- **WHEN** the context picker is open without a filter
- **AND** the user presses the unbound `ctrl-x`
- **THEN** no filter starts
- **AND** the selected context stays the same
