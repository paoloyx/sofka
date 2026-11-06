# diagnostics Specification

## Purpose

Give users and maintainers redacted, shareable insight into a running or headless sofka: the `:info` and `sofka info` reports, per-resource incident bundles, table snapshots, structured logging, state and log directories, and the handling of internal failures and panics.

## Requirements

### Requirement: Info report command

`:info` (aliases `:diagnostics`, `:about`) SHALL open a document titled `diagnostics (:info)` containing, in order: the version line `sofka v<version>` with `build: <os>/<arch> (debug|release)`, then the Cluster, Watch health, Actions, API request latency, Config sources, Active config, Logging and Directories sections, and a `Warnings [N]` section when warnings exist.

#### Scenario: Report shows version, cluster and watch health

- **WHEN** the server version is `v1.36.2-eks-bca9cf6` with 3 watch errors, 2 reconnects and last error `connection refused`
- **AND** the user runs `:info`
- **THEN** the document shows `sofka v<version>`, `k8s rev:     v1.36.2-eks-bca9cf6`, `errors:     3`, `reconnects: 2` and `connection refused`

### Requirement: Info cluster section

The Cluster section SHALL show whether sofka is connected, the context, cluster, API server URL, Kubernetes server revision, the namespace (`(all)` for all namespaces), `discovery: N resource kinds` with one `•` line per discovery fallback or skipped API group, the metrics API state (`available (data received)` or `no data yet`), and the last metrics poll error when there is one.

#### Scenario: Skipped discovery groups listed

- **WHEN** discovery could not read an API group
- **AND** the user runs `:info`
- **THEN** the Cluster section lists that group on a `•` line

### Requirement: Info watch health and actions

The Watch health section SHALL show the count of watch errors, the count of watch reconnects (a re-list after an initial sync counts as one), and the last error or `none`. The Actions section SHALL show the last failed action.

#### Scenario: Re-list counts as reconnect

- **WHEN** a watch syncs, then resets and syncs again
- **AND** the user runs `:info`
- **THEN** the report shows `reconnects: 1`

### Requirement: Info request latency table

When any API requests have been made, the report SHALL include a latency table with columns CLASS, COUNT, ERRORS, AVG, P50, P90 and MAX, one row per request class used (discovery, read, watch, write, metrics, logs, exec). With no requests the table SHALL be omitted.

#### Scenario: No requests yet

- **WHEN** no API request has been recorded
- **THEN** the report has no latency table

### Requirement: Info config sources

The Config sources section SHALL list the base config path with its state (`loaded`, `invalid — using defaults`, or `absent — using defaults`), or `no config directory — using defaults`, followed by every drop-in and per-cluster or per-context override path with its state.

#### Scenario: Missing base config

- **WHEN** the config directory exists without a base config file
- **THEN** the base path is shown as `absent — using defaults`

### Requirement: Info active config

The Active config section SHALL show the skin (or `auto`), the effective read-only flag, the alias count, plugin and view counts each followed by up to 12 sorted names and `+N more`, and the bookmark, guardrail and config warning counts. Plugin input values MUST NOT appear.

#### Scenario: Plugins named

- **WHEN** one plugin `argocd-sync` is loaded
- **THEN** the report shows `plugins:    1  (argocd-sync)`

### Requirement: Info logging and directories

The Logging section SHALL show the level, the log file, lines written and `dropped: N (writer fell behind)`, or `(disabled — set [logging] level or SOFKA_LOG=debug)` when logging is off. The Directories section SHALL show the state, logs, snapshots and bundles directories, and the last state write error when one occurred.

#### Scenario: Logging off by default

- **WHEN** no logging level is configured
- **THEN** the report shows `level: off` and the Directories section lists `logs:`

### Requirement: Info warnings section

When there are config or runtime warnings (including journal failures), the report SHALL end with `Warnings [N]` and one bullet per warning. With no warnings the section SHALL be omitted.

#### Scenario: Journal failure listed

- **WHEN** a journal entry could not be saved
- **THEN** `:info` lists a `journal entry not saved` warning

### Requirement: Info report is redacted

Every line of the `:info` report, including its header, SHALL pass through credential and address redaction (see the safety capability), so server URLs with userinfo, IP addresses, tokens in errors and credential query parameters appear as `«redacted»`. The resource view MUST still show the unredacted server after leaving the report.

#### Scenario: IP server masked in report only

- **WHEN** the server is `https://10.40.0.3:6443/` and a watch error names it
- **AND** the user opens `:info`
- **THEN** neither the report nor the screen shows `10.40.0.3`, and `«redacted»` with `:6443/` is shown
- **AND** after Esc the resource view shows the address again

#### Scenario: Credentials in errors masked

- **WHEN** the last error contains `authorization: Bearer <token>` and the metrics error contains `token=s3cr3t`
- **THEN** the report shows neither secret and contains `«redacted»`

### Requirement: Headless info report

`sofka info` SHALL print the same report as `:info` to stdout, every line redacted. Online it SHALL print `Connecting to cluster…` to stderr and honour `--context`; on a connection failure it SHALL print `warning: <redacted error>` to stderr, still print the report and exit 0. `sofka info --offline` (and the deprecated `--info`) SHALL skip connecting.

#### Scenario: Offline report

- **WHEN** the user runs `sofka info --offline`
- **THEN** the Cluster section shows `connected: not attempted (--offline)` with the identity read from kubeconfig
- **AND** the process exits 0

### Requirement: Headless cluster details

In `sofka info`, the Kubernetes revision, discovery and metrics lines SHALL appear only when connected, with the metrics API shown as `discovered (metrics.k8s.io)` or `not installed`.

#### Scenario: Metrics server absent

- **WHEN** `sofka info` connects to a cluster without metrics.k8s.io
- **THEN** the report shows `metrics API: not installed`

### Requirement: Headless watch probe

When connected, `sofka info` SHALL probe a watch on the resource and namespace a normal launch would open, printing `probe: <resource> in <ns|all namespaces>` and `established: yes, in Nms (K objects in the initial list)`, or `no — see the error below`, `no — no initial sync within 5s`, or `no — '<res>' is not in this cluster's discovery`. When not connected it SHALL print `not probed (--offline)` or `not probed — no connection`.

#### Scenario: Probe times out

- **WHEN** the watch does not complete an initial list within 5 seconds
- **THEN** the report shows `established: no — no initial sync within 5s`

### Requirement: Headless warnings footer

`sofka info` SHALL always end with the warnings list, or `No config validation warnings.` when there are none. Logging for `sofka info` SHALL be configured from the base config only.

#### Scenario: Clean config

- **WHEN** the config has no warnings
- **THEN** the last line is `No config validation warnings.`

### Requirement: Bundle assembly

`:bundle` (aliases `:diag`, `:incident`) SHALL gather a diagnostic bundle for the selected object, showing `assembling bundle for <name>…`, then open a preview titled `Diagnostic bundle — <Kind>/<name> (:bundle-save to write)` with status `bundle ready — review, then :bundle-save`. Nothing SHALL be written to disk until `:bundle-save`.

#### Scenario: Preview before save

- **WHEN** the user runs `:bundle` on a selected pod
- **THEN** a preview document opens and no file is written

### Requirement: Bundle refusals

`:bundle` SHALL refuse with an error flash: `select a resource first` with no resource kind, `bundle is not available for Helm releases` in the Helm views, and `no selection for bundle` with no selected row.

#### Scenario: No selection

- **WHEN** no resource is selected
- **AND** the user runs `:bundle`
- **THEN** no preview opens and an error flash is shown

### Requirement: Bundle contents

A bundle SHALL contain a header (sofka version, collection time, context, cluster, namespace or `(cluster-scoped)`, resource), a manifest of included and redacted items, the explain-unhealthy findings (or `(healthy — nothing to explain)`), owner YAML, resource YAML, events, session timeline entries, a metrics snapshot (`cpu`, `memory`), and one `Logs — <pod>` section per pod. Empty sections SHALL show `_none_`.

#### Scenario: Healthy pod

- **WHEN** the selected pod has no findings
- **THEN** the Explanation section shows `(healthy — nothing to explain)`

### Requirement: Bundle logs

For a pod, the bundle SHALL include its own logs; for deployments, statefulsets, daemonsets and replicasets it SHALL include logs of pods matched by the selector, up to `[bundle] max_pods` (default 3). Each pod SHALL contribute the timestamped last `[bundle] log_lines` lines (default 200) of its first container; `log_lines <= 0` SHALL disable logs. A failed fetch SHALL show `(logs unavailable: <e>)`.

#### Scenario: Logs disabled

- **WHEN** `[bundle] log_lines = 0`
- **THEN** the bundle contains no log sections

### Requirement: Bundle redaction

Bundle YAML SHALL drop managedFields, replace the `last-applied-configuration` annotation and credential-named annotations with `«redacted»`, replace every Secret `data`/`stringData` value with `«redacted»` noting `Secret data: N value(s)`, and note `N env var(s) sourced from Secrets (values not included)`. The owner SHALL be redacted the same way with notes prefixed `owner: `. With nothing to redact the manifest SHALL say `nothing sensitive found to redact`.

#### Scenario: Secret value removed

- **WHEN** a bundle is built for a Secret with `password: aHVudGVyMg==`
- **THEN** the YAML does not contain `aHVudGVyMg==` and contains `«redacted»`
- **AND** the redaction notes mention `Secret data`

### Requirement: Bundle anonymization

When `[bundle] anonymize = true`, the bundle header SHALL show `«anonymized»` for context and cluster and the manifest SHALL note `context/cluster anonymized`.

#### Scenario: Anonymized header

- **WHEN** anonymize is enabled and the user runs `:bundle`
- **THEN** the header shows `«anonymized»` instead of the context and cluster names

### Requirement: Bundle save

`:bundle-save` (alias `:bundle-write`) SHALL write the previewed bundle to the OS temp directory as `sofka-bundle-<Kind>-<name>-<epoch>.md` (non-alphanumeric characters replaced with `-`) and flash `saved bundle → <path>`, or `bundle save failed: <e>`. Without a preview it SHALL flash the error `no bundle to save — run :bundle first`.

#### Scenario: Save without bundle

- **WHEN** no bundle has been previewed
- **AND** the user runs `:bundle-save`
- **THEN** the error flash `no bundle to save — run :bundle first` is shown

### Requirement: Table snapshot capture

`:snapshot [text|txt|json|yaml|yml]` (aliases `:snap`, `:dump`; default text) SHALL capture the displayed columns, including NAMESPACE when shown, and every visible filtered row, showing `saving snapshot (N rows)…` then `saved snapshot → <path>` or `snapshot save failed: <e>`. An unknown format SHALL flash the error `unknown snapshot format '<arg>' (text/json/yaml)`; with no resource, `select a resource first`.

#### Scenario: Unknown format

- **WHEN** the user runs `:snapshot xml`
- **THEN** the error flash contains `unknown snapshot format`

#### Scenario: Rows captured

- **WHEN** the table shows pods `api-1` and `api-2`
- **THEN** the snapshot has a `NAME` column, two rows, and one cell per column in each row

### Requirement: Snapshot files

Snapshots SHALL be written to `<state-dir>/snapshots/` (created when missing) as `sofka-snapshot-<resource>-<ns|all>-<epoch>.<txt|json|yaml>`, with non-alphanumeric characters replaced by `-`. Text snapshots SHALL start with `# sofka snapshot — <resource>`, `# captured:`, `# context:`, `# cluster:`, `# namespace:`, `# filter:` (when set) and `# rows: N` lines, then an aligned table. JSON and YAML SHALL hold captured_at, context, cluster, namespace, resource, filter, columns and rows.

#### Scenario: JSON snapshot name

- **WHEN** pods in namespace `web` are captured as JSON at epoch 1700000000
- **THEN** the file is named `sofka-snapshot-pods-web-1700000000.json`

### Requirement: Snapshot browser

`:snapshots` (alias `:dumps`) SHALL list saved snapshots newest first as `<filename>   (<age> ago)`, or flash `no snapshots yet — capture one with :snapshot` when there are none. `j`/`k`/arrows SHALL move, page keys SHALL page, and Esc or `q` SHALL return to the table.

#### Scenario: Empty browser

- **WHEN** no snapshots exist
- **AND** the user runs `:snapshots`
- **THEN** the flash shows `no snapshots yet — capture one with :snapshot`

### Requirement: Snapshot viewing and deletion

In the snapshot browser, Enter SHALL open the file in a viewer titled with its file name whose first line is `⚠ snapshot captured <time> (<age>) — a point-in-time capture, likely stale`; Esc SHALL return to the browser. `d` SHALL delete the selected file without confirmation and flash `snapshot deleted`, returning to the table when none remain. Errors SHALL flash `read snapshot failed: <e>` or `delete failed: <e>`.

#### Scenario: Delete last snapshot

- **WHEN** the browser lists one snapshot
- **AND** the user presses `d`
- **THEN** the file is removed, `snapshot deleted` is flashed and the table is shown

### Requirement: Logging levels

`[logging] level` SHALL accept off (default), error, warn, info, debug and trace, with `none`/`silent` meaning off and `warning` meaning warn. An invalid value SHALL disable logging with the warning `logging: level '<x>' is not off/error/warn/info/debug/trace; logging disabled`. A non-empty `SOFKA_LOG` SHALL override the config level; an invalid `SOFKA_LOG` SHALL be ignored with a warning, keeping the config level.

#### Scenario: Environment override

- **WHEN** the config sets `level = "off"` and `SOFKA_LOG=debug`
- **THEN** debug logging is active

### Requirement: Log file

Logs SHALL be written to `[logging] file`, default `<state-dir>/logs/sofka.log`; an empty `file` SHALL use the default with a warning. When logging is off no file SHALL be created. If the file cannot be opened, sofka SHALL warn `opening log file <path>: <e>` and keep running. Only the base config SHALL control logging; per-cluster overrides MUST NOT.

#### Scenario: Logging off creates nothing

- **WHEN** logging is off
- **THEN** no log file or logs directory is created

### Requirement: Log rotation and sharing

Before a line would grow the log file past `max_size_mb` (default 8, minimum 64 KiB), the file SHALL rotate to `<file>.1`, keeping one backup. Concurrent sessions SHALL share the file through `<file>.lock`. Lines that cannot be queued (more than 4096 pending) SHALL be dropped and counted in `:info`.

#### Scenario: Rotation

- **WHEN** a line would exceed the size limit
- **THEN** the log moves to `<file>.1` and writing continues in a new file

### Requirement: Log line format

Each log event SHALL be one logfmt line `ts=<UTC time with milliseconds> level=<level> event=<name>` followed by `key=value` fields, with every value redacted and values containing spaces, quotes or `=` quoted and escaped. sofka SHALL log `startup` (version, build, logging settings), `shutdown`, `action`, `task.panic` and `state.write.failed` events.

#### Scenario: Action logged

- **WHEN** info logging is on and the user edits a pod
- **THEN** the log has a line with `level=info event=action` and the context, action and target

### Requirement: State directory

sofka SHALL keep state under `$XDG_STATE_HOME/sofka` when that variable is non-empty, else `~/.local/state/sofka`, else `<temp>/sofka`, and SHALL replace state files atomically so a crash never leaves a partial file.

#### Scenario: XDG location

- **WHEN** `XDG_STATE_HOME=/var/state` is set
- **THEN** state, logs and snapshots live under `/var/state/sofka`

### Requirement: State write failures

A failed state write SHALL show the error flash `state not saved: <error>`, log `state.write.failed`, and appear as `last state write error` in `:info`, but MUST NOT count as a watch error. Failures still pending at exit SHALL be printed to stderr as `warning: state not saved: <error>`, and if the writer is still busy after 500 ms sofka SHALL print a warning that the last change may not be saved and exit.

#### Scenario: Failure kept out of watch health

- **WHEN** a state file cannot be written
- **THEN** the flash shows `state not saved:` and the watch error count is unchanged

### Requirement: Background task panics

A panic in a background task SHALL show the error status `internal error: <panic>`, set the `:info` watch-health last error, and log a `task.panic` event. The message MUST be shown even if it arrives after a context or view change.

#### Scenario: Panic after context switch

- **WHEN** a background task from an earlier view panics
- **THEN** `internal error: <panic>` is flashed in the current view

### Requirement: Main thread panic restores the terminal

If sofka itself panics, it SHALL restore the terminal (leave the alternate screen, disable mouse capture, clear the title, show the cursor) before printing the panic report.

#### Scenario: Panic leaves a usable shell

- **WHEN** the UI thread panics
- **THEN** the shell is back in normal mode with a visible cursor and the panic message is printed

### Requirement: Startup diagnostics flashes

At startup sofka SHALL flash `config loaded with N warning(s) — :config for details` when config warnings exist and no error is already shown, and SHALL flash `API discovery could not read N API group(s). Refer to :info for details.` (singular `group` for one) when discovery skipped groups.

#### Scenario: One skipped group

- **WHEN** discovery could not read one API group
- **THEN** the flash says `API discovery could not read 1 API group. Refer to :info for details.`
