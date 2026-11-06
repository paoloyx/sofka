# port-forwarding Specification

## Purpose

Port-forwarding lets the user reach a pod or Service from the local machine without leaving sofka. `f` on a row opens a picker of the object's declared ports, forwards run as background `kubectl port-forward` processes that survive view changes, `:pf` lists and stops them, and saved `[[forwards]]` entries can be started from the list or automatically when a matching context connects.

## Requirements

### Requirement: Port-forward key scope

`f` and `F` SHALL open the port-forward picker on pods and services only. On any other kind the key SHALL first be offered to user bookmarks, workspaces and plugins bound to it, and only when none takes it SHALL sofka flash `port-forward applies to pods/services`.

#### Scenario: Port-forward on a deployment

- **WHEN** the deployment table is shown and no user binding uses `f`
- **AND** the user presses `f`
- **THEN** the flash reads `port-forward applies to pods/services`
- **AND** no picker opens

### Requirement: Service port list

The picker for a Service SHALL list one entry per TCP port in `spec.ports`, labelled `<port>:<port>` followed by `  (<port name>)` when the port is named. A port with no `protocol` counts as TCP; UDP and SCTP ports MUST NOT be listed.

#### Scenario: Named and UDP ports

- **WHEN** a Service declares TCP port 80 named `http` and UDP port 53
- **AND** the user presses `f` on it
- **THEN** the picker lists `80:80  (http)`
- **AND** port 53 is not listed

### Requirement: Pod port list

The picker for a pod SHALL list each TCP `containerPort` of its containers, init containers and ephemeral containers, labelled `<port>:<port>  (<container>/<port name>)`, skipping init and ephemeral containers whose status is terminated. Identical entries MUST appear only once.

#### Scenario: Terminated init container

- **WHEN** a pod's init container declares port 9000 and has terminated
- **AND** the user presses `f` on the pod
- **THEN** port 9000 is not listed

### Requirement: Custom entry

The picker SHALL always end with `Custom…`, and SHALL contain only `Custom…` when no port is declared. `enter` on `Custom…` SHALL open the prompt `Port-forward <name> (LOCAL:REMOTE, e.g. 8080:80):` with empty input. Submitting it empty MUST flash `no ports given` and start nothing.

#### Scenario: Object with no declared ports

- **WHEN** a pod declares no container ports
- **AND** the user presses `f`
- **THEN** the picker contains only `Custom…`

### Requirement: Picker navigation

In the picker, `j`/`k` and the arrow keys SHALL move one entry, `PgDn`/`PgUp` one page, and `esc` or `q` SHALL close it without starting anything.

#### Scenario: Escape the picker

- **WHEN** the picker is open
- **AND** the user presses `esc`
- **THEN** the table is shown again and no forward is running

### Requirement: Start a declared mapping

`enter` on a declared mapping SHALL start `kubectl port-forward -n <ns> <target> <LOCAL:REMOTE>` in the background, pinned to the active context, where the target is `pod/<name>` for pods and `svc/<name>` for services. On success it SHALL close the picker and flash `port-forwarding <target> <ports> -n <ns> (:pf to view/stop)`. A process that cannot be spawned MUST flash `port-forward failed to start: <error>`.

#### Scenario: Forward a Service port

- **WHEN** the picker for Service `web` in `default` selects `8080:8080`
- **AND** the user presses `enter`
- **THEN** sofka starts `kubectl port-forward -n default svc/web 8080:8080`
- **AND** the flash ends with `(:pf to view/stop)`

### Requirement: Local port availability check

Before starting a forward from the picker or a prompt, sofka SHALL try to bind the local port on `127.0.0.1` and `::1` and treat it as available if either bind succeeds. When neither succeeds it MUST flash `port <port> is already in use` (or `cannot bind local port <port>: <error>`) and keep the picker or prompt open. A port of 0 or a non-numeric local part skips the check.

#### Scenario: Port taken

- **WHEN** local port 8080 is held by another process on both loopback addresses
- **AND** the user presses `enter` on `8080:8080`
- **THEN** the flash reads `port 8080 is already in use`
- **AND** the picker stays open on the same row

#### Scenario: One loopback family free

- **WHEN** port 8080 is taken on `127.0.0.1` but free on `::1`
- **AND** the user presses `enter` on `8080:8080`
- **THEN** the forward starts

### Requirement: Edit the local port

`e` on a declared mapping SHALL open the prompt `Local port for <name>, remote <remote>:` prefilled with the current local port. The input MUST be a number from 1 to 65535, otherwise sofka SHALL flash `local port must be a number from 1 to 65535` and keep the prompt open. `esc` SHALL return to the same picker row. `e` on `Custom…` does nothing.

#### Scenario: Fix a conflict by editing only the local port

- **WHEN** port 8080 is in use and the picker is on `8080:8080`
- **AND** the user presses `e`, replaces the value with `18080` and presses `enter`
- **THEN** sofka forwards `18080:8080`

### Requirement: Prompts keep input on failure

When the local port of a custom or edited forward is unavailable, or the edited forward's process cannot be spawned, the prompt SHALL stay open with the typed value so the user can correct it.

#### Scenario: Spawn failure keeps the edit

- **WHEN** the edited local port is `18080` and spawning `kubectl` fails
- **THEN** the prompt is still open containing `18080`

### Requirement: Forward ownership and safety scope

A port-forward SHALL run as a background process without suspending the TUI. It is not a cluster mutation: it MUST NOT be refused by read-only mode, matched by guardrails or recorded in `:journal`.

#### Scenario: Read-only mode

- **WHEN** read-only mode is on
- **AND** the user starts a forward from the picker
- **THEN** the forward starts

### Requirement: Active forward marker

A table row SHALL show a teal `●` in the name column while a forward started in sofka targets that object, matching on the context name, the cluster server URL, the namespace and the target spelling (`pod/<name>` or `svc/<name>`). A pod and a Service with the same name MUST NOT share a marker.

#### Scenario: Service forward does not mark the pod

- **WHEN** `svc/db` in `default` is forwarded
- **THEN** the Service row `db` shows `●`
- **AND** a pod named `db` in `default` does not

### Requirement: Forward list view

`:pf` (also `:portforwards`, `:forwards`) SHALL open a list titled `Port-forwards [<running count>]` showing running forwards as `● [<saved name>: ]<target> <ports> -n <ns>`, followed by saved `[[forwards]]` entries that are not running as `○ <name>: <target> <ports> -n <ns> (stopped)`. `j`/`k` move; `esc` or `q` return to the table.

#### Scenario: Saved but stopped

- **WHEN** `[[forwards]]` defines `argocd` and it is not running
- **AND** the user runs `:pf`
- **THEN** the list shows `○ argocd: ... (stopped)`

### Requirement: Stop a running forward

`x` or `s` on a running forward in `:pf` SHALL terminate its process, remove it from the list and flash `stopped port-forward <label>`, leaving other forwards running. `enter` on a running forward MUST do nothing.

#### Scenario: Enter does not stop

- **WHEN** the cursor is on a running forward
- **AND** the user presses `enter`
- **THEN** the forward keeps running

### Requirement: Start a saved forward

`enter`, `x` or `s` on a stopped saved forward in `:pf` SHALL start `kubectl port-forward` with its `namespace` (omitting `-n` when empty), `target` and `ports`, without the local bind check. A saved forward whose name is already running MUST flash `forward '<name>' is already running`.

#### Scenario: Start from the stopped tail

- **WHEN** the cursor is on `○ argocd: svc/argocd-server 8080:443 -n argocd (stopped)`
- **AND** the user presses `enter`
- **THEN** sofka starts `kubectl port-forward -n argocd svc/argocd-server 8080:443`
- **AND** the entry is listed as running

### Requirement: Saved forward configuration

Each `[[forwards]]` entry SHALL accept `name`, `target` in kubectl syntax (for example `pod/web`, `svc/api`, `deploy/api`), `namespace`, `ports` as `LOCAL:REMOTE`, `autostart` (default false) and `contexts` (exact context names; empty means every context). Entries without a name MUST NOT be listed or started.

#### Scenario: Unnamed entry

- **WHEN** a `[[forwards]]` entry has no `name`
- **THEN** it does not appear in `:pf`

### Requirement: Saved forward validation warnings

Loading config SHALL warn `forwards: entry with empty name`, `forwards.<name>: duplicate name`, `forwards.<name>: empty target` and `forwards.<name>: ports '<ports>' is not LOCAL:REMOTE (e.g. 8080:80)` for the matching problems, without refusing to start. A single numeric port such as `8080` is accepted.

#### Scenario: Bad port spec

- **WHEN** a forward named `web` has `ports = "http:80"`
- **THEN** config warnings include `forwards.web: ports 'http:80' is not LOCAL:REMOTE (e.g. 8080:80)`

### Requirement: Autostart

When sofka connects, and after each context switch, it SHALL start every saved forward with `autostart = true` whose `contexts` match the active context and whose name is not already running. While disconnected it MUST NOT autostart anything.

#### Scenario: Context-limited autostart

- **WHEN** a forward has `autostart = true` and `contexts = ["home"]`
- **AND** sofka connects to context `work`
- **THEN** the forward is not started

### Requirement: Forwards across context switch and reload

Switching context SHALL load the new context's saved forwards and keep forwards already running from the previous context alive. `:reload` SHALL refresh the saved list without stopping running forwards and without autostarting.

#### Scenario: Forward survives a context switch

- **WHEN** a forward is running in context `a`
- **AND** the user switches to context `b`
- **THEN** the forward is still listed in `:pf`

### Requirement: Exited forwards are reaped

On every tick sofka SHALL remove forwards whose `kubectl` process has exited (pod restarted, connection dropped, port taken) and flash `port-forward <label> exited`.

#### Scenario: Pod deleted under a forward

- **WHEN** the `kubectl port-forward` process for `pod/web` exits
- **THEN** it disappears from `:pf`
- **AND** the flash reads `port-forward pod/web <ports> -n <ns> exited`

### Requirement: Forwards stop on quit

Quitting sofka SHALL terminate every port-forward process it started.

#### Scenario: Quit with forwards running

- **WHEN** two forwards are running
- **AND** the user quits
- **THEN** both `kubectl port-forward` processes are killed
