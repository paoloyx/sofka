# logs Specification

## Purpose

The logs view streams container logs for a pod, a set of marked pods, or every pod behind a workload or Service, straight from the Kubernetes API, and can instead query a VictoriaLogs backend for history beyond a pod's lifetime. It orders, filters, colors, formats and exports the lines.

## Requirements

### Requirement: Single pod logs

Pressing `l` on a pod row with no marked pods SHALL open a log view titled `<name> — logs` that streams every container of the pod, including init and ephemeral containers, prefixing each line with `[<container>] ` only when the pod has more than one container.

#### Scenario: Pod with two containers

- **WHEN** a pod `web` with containers `app` and `proxy` is selected
- **AND** the user presses `l`
- **THEN** a view titled `web — logs` opens
- **AND** lines are prefixed `[app] ` or `[proxy] `

### Requirement: Marked pod logs

Pressing `l` in the pod table while pods are marked SHALL stream every container of the marked pods that are still present in the current filtered rows, in a view titled `marked pods (N) - logs` with each line prefixed `[<namespace>/<pod>:<container>] `. When no marked pod remains in view it SHALL warn `no marked pods in the current view`.

#### Scenario: Two marked pods

- **WHEN** pods `a/web` and `b/api` are marked
- **AND** the user presses `l`
- **THEN** a view titled `marked pods (2) - logs` streams both pods

#### Scenario: Marks hidden by the filter

- **WHEN** the only marked pod is hidden by the table filter
- **AND** the user presses `l`
- **THEN** the flash shows `no marked pods in the current view` and the table stays open

### Requirement: Workload logs

Pressing `l` on a Deployment, StatefulSet, DaemonSet, ReplicaSet or Job SHALL stream the pods matching its `spec.selector` in a view titled `<kind>/<name> — logs (all pods)`, prefixing lines with `[<pod>:<container>] ` for multi-container pods or `[<pod>] ` otherwise. A workload without a selector SHALL warn `no pod selector for logs`.

#### Scenario: Deployment logs

- **WHEN** a Deployment `api` with two ready pods is selected
- **AND** the user presses `l`
- **THEN** a view titled `deployment/api — logs (all pods)` streams lines from both pods

### Requirement: Service logs

Pressing `l` on a Service SHALL stream the pods matching its selector in a view titled `svc/<name> — logs (all pods)`. A Service without a selector SHALL warn `service has no selector`.

#### Scenario: Service without selector

- **WHEN** a Service with no selector is selected
- **AND** the user presses `l`
- **THEN** the flash warns `service has no selector`

### Requirement: Unsupported log targets

Pressing `l` on any other resource kind SHALL warn `logs available for pods and workloads` and SHALL NOT open a log view.

#### Scenario: Logs on a ConfigMap

- **WHEN** a ConfigMap row is selected
- **AND** the user presses `l`
- **THEN** the flash warns `logs available for pods and workloads`

### Requirement: Selector stream scope

Workload and Service log views SHALL list the matching pods once at open, stream only their regular containers with at most 100 initial lines per container, show `(no matching pods)` when none match, and show `[error] <error>` when listing fails.

#### Scenario: No pods match

- **WHEN** a Deployment is scaled to zero
- **AND** the user presses `l` on it
- **THEN** the log view shows `(no matching pods)`

### Requirement: Previous container logs

Pressing `p` on a pod row SHALL show the full previous-instance logs of the pod's first container, without tail, since or follow, in a view titled `<name> — previous logs`.

#### Scenario: Crash-looping pod

- **WHEN** a restarting pod `web` is selected
- **AND** the user presses `p`
- **THEN** a view titled `web — previous logs` shows the previous container's output

### Requirement: Container picker logs

In the container picker, `enter` or `l` SHALL open `<pod>:<container> — logs`, `p` SHALL open `<pod>:<container> — previous logs`, and `L` SHALL open VictoriaLogs history for that container.

#### Scenario: Pick a container

- **WHEN** the container picker for pod `web` highlights container `proxy`
- **AND** the user presses `enter`
- **THEN** a view titled `web:proxy — logs` streams only that container

### Requirement: Initial tail and lookback

Each kubelet log stream SHALL request the last `[logs] tail` lines (default 300, at least 1) with timestamps, and, when `[logs] since` holds a valid duration, only lines newer than it; an invalid `since` SHALL be ignored.

#### Scenario: Configured since

- **WHEN** `[logs] since = "30m"` and `[logs] tail = 50` are set
- **AND** the user opens pod logs
- **THEN** each stream requests at most 50 lines from the last 30 minutes

### Requirement: Waiting containers

A container that is still waiting to start SHALL be retried every second until its stream opens, and retries SHALL stop when the user leaves the log view.

#### Scenario: Container starting

- **WHEN** the user opens logs of a pod whose container is still creating
- **THEN** the view keeps retrying and streams lines once the container starts

### Requirement: Stream errors

Any other stream error SHALL add a line `<prefix>[error] <error>` to the view while the other streams keep running.

#### Scenario: One container fails

- **WHEN** one of two container streams returns a forbidden error
- **THEN** the view shows `[<container>] [error] <error>`
- **AND** lines from the other container keep arriving

### Requirement: Line cleanup

Log lines SHALL have carriage returns removed and tabs expanded to spaces before they are stored.

#### Scenario: Tab-separated output

- **WHEN** a container writes `a\tb\r`
- **THEN** the stored line contains spaces instead of the tab and no carriage return

### Requirement: Chronological ordering

Log lines from all streams SHALL be ordered by their timestamps, keeping arrival order for equal times; a line without a timestamp SHALL sort with the newest known time, or its arrival time when none is known.

#### Scenario: Interleaved containers

- **WHEN** two containers write lines at 10:00:01 and 10:00:00 that arrive in that order
- **THEN** the 10:00:00 line is shown first

### Requirement: Buffer cap

The log buffer SHALL keep at most `[logs] buffer` lines (default 5000, at least 1) while following and up to 100000 lines while paused, dropping the oldest lines first.

#### Scenario: Chatty pod

- **WHEN** `[logs] buffer = 100` and a container writes 150 lines while following
- **THEN** the view keeps only the newest 100 lines

### Requirement: Follow and pause

The log view SHALL follow new lines by default; scrolling up, down, by page, or to the top SHALL pause following while the viewport stays fixed as lines arrive, and `G`/end SHALL resume following.

#### Scenario: Pause by scrolling up

- **WHEN** a log view is following
- **AND** the user presses `k` and 500 lines arrive
- **THEN** the viewport stays at the same position
- **AND** pressing `G` resumes following

### Requirement: Autoscroll toggle

`s` or `f` SHALL toggle following with the flash `autoscroll: on` or `autoscroll: off`.

#### Scenario: Toggle autoscroll

- **WHEN** a log view is following
- **AND** the user presses `s`
- **THEN** the flash shows `autoscroll: off`

### Requirement: Log movement keys

In the log view, `j`/down and `k`/up SHALL scroll one row, `pagedown`/`space` and `pageup` SHALL scroll 20 rows, and `g`/home SHALL go to the top; downward scrolling SHALL be clamped at the last page.

#### Scenario: Page down at the bottom

- **WHEN** a paused log view already shows its last page
- **AND** the user presses `j`
- **THEN** the viewport does not move

### Requirement: Stop and resume streams

`x` SHALL stop all streams of the view with the flash `log stream stopped`, and pressing it again SHALL clear the buffer and restart the streams with the flash `log stream resumed`.

#### Scenario: Resume restarts

- **WHEN** a stopped log view holds 200 lines
- **AND** the user presses `x`
- **THEN** the buffer is cleared and the streams restart

### Requirement: Clear buffer

`z` SHALL empty the log buffer, including markers, with the flash `log buffer cleared`, while streams keep running.

#### Scenario: Clear and continue

- **WHEN** a following log view holds lines
- **AND** the user presses `z`
- **THEN** the view is empty and new lines keep appearing

### Requirement: Log line wrap

`w` SHALL toggle line wrap in the log view with the flash `wrap: on` or `wrap: off`; the choice SHALL persist for later log views in the session.

#### Scenario: Wrap long lines

- **WHEN** a log view shows a 300-column line
- **AND** the user presses `w`
- **THEN** the line wraps onto several rows

### Requirement: Timestamps toggle

`t` SHALL show or hide each line's timestamp with the flash `timestamps: on` or `timestamps: off`, without restarting streams and keeping a paused viewport on the same line. Timestamps SHALL be hidden by default and the choice SHALL persist for the session.

#### Scenario: Show timestamps

- **WHEN** a log view hides timestamps
- **AND** the user presses `t`
- **THEN** each line starts with its RFC 3339 timestamp and the title shows `ts`

### Requirement: Log filter syntax

`/` SHALL open a log filter where plain text matches as a case-insensitive substring, `/regex/` matches as a case-insensitive regular expression, and a leading `!` inverts the match; an invalid regex SHALL hide every line and mark the title `[invalid regex]`.

#### Scenario: Inverted regex

- **WHEN** the user types `!/health(z|check)/` into the log filter
- **THEN** lines matching the regex are hidden and all others stay visible

### Requirement: Log filter display

An active log filter SHALL show `/<filter> [<matched count>]` in the title, highlight matches only for a plain substring, keep its value on `enter`, and clear it on `esc` while typing. The filter SHALL match the stored line, which has no timestamp while timestamps are hidden.

#### Scenario: Esc clears the filter

- **WHEN** the user types `error` into the log filter
- **AND** presses `esc`
- **THEN** the filter is removed and all lines are shown

### Requirement: Warning and error filter

`ctrl-z` SHALL toggle a filter that shows only warning and error lines, combined with any text filter, marking the title `[warn/error]` and scrolling to the top.

#### Scenario: Show only problems

- **WHEN** a log view holds info, warn and error lines
- **AND** the user presses `ctrl-z`
- **THEN** only the warn and error lines are shown

### Requirement: Severity detection

A line's severity SHALL come from a JSON `level` or `severity` field when present, then from a klog `E`, `F` or `W` prefix, then from the leftmost marker such as `error`, `level=error`, `fatal`, `panic`, `warn`, `debug` or `trace`. Error lines SHALL be red, warnings peach and debug lines dim.

#### Scenario: JSON level wins

- **WHEN** a line is `{"level":"info","msg":"error in text"}`
- **THEN** the line is not treated as an error

#### Scenario: klog warning

- **WHEN** a line is `W0627 12:00:00 retry`
- **THEN** the line is treated as a warning

### Requirement: ANSI colors and prefixes

The log view SHALL map ANSI SGR colors onto the skin palette and drop other escape sequences, draw each source prefix bold in a color that stays stable for that source, and dim leading timestamps.

#### Scenario: Colored output

- **WHEN** a container writes a line with an ANSI red sequence
- **THEN** the text is drawn in the skin's red without raw escape characters

### Requirement: JSON formatting

`J` SHALL toggle pretty-printing of lines whose payload after any prefix and timestamp starts with `{` or `[`, with the flash `JSON formatting: on` or `JSON formatting: off` and ` JSON` in the title. Records over 4 KiB SHALL stay raw. Filtering, copying and saving SHALL use the raw line.

#### Scenario: Pretty-print a record

- **WHEN** a log line is `{"level":"info","msg":"ready"}`
- **AND** the user presses `J`
- **THEN** the record is shown across several indented rows

### Requirement: Log markers

`m` SHALL add a dim full-width dashed marker after the newest line with the flash `log marker added`. Markers SHALL stay visible through filters, SHALL be excluded from copy and save, SHALL disappear when the buffer is cleared or trimmed past them, and adding one while paused MUST NOT move the viewport.

#### Scenario: Mark before a deploy

- **WHEN** the user presses `m` in a following log view
- **AND** new lines arrive
- **THEN** a dashed row separates the older lines from the new ones

### Requirement: Log copy

`c` SHALL copy the lines that pass the current filters, without markers, and flash `copied N log lines`, or warn `no log lines to copy` when none pass.

#### Scenario: Copy filtered lines

- **WHEN** a log filter shows 3 of 50 lines
- **AND** the user presses `c`
- **THEN** the 3 visible lines are copied

### Requirement: Log save

`ctrl-s` SHALL write the lines that pass the current filters to `sofka-<title>-<unix seconds>.log` in the system temp directory, with non-alphanumeric title characters replaced by `-`, flashing `saving logs…` and then `saved logs → <path>` or `save failed: <error>`; with no lines it SHALL warn `no log lines to save`.

#### Scenario: Save after leaving

- **WHEN** the user presses `ctrl-s`
- **AND** leaves the log view before the write finishes
- **THEN** the flash still reports `saved logs → <path>`

### Requirement: Time anchors

In a kubelet log view, `0` SHALL restart the streams from the configured tail with the flash `showing tail (N lines)`, and `1` to `5` SHALL restart them from the last 1m, 5m, 15m, 30m or 1h with the flash `showing last <period>`, keeping the tail limit and marking the title `⏱<period>`. A stopped view SHALL keep its streams stopped.

#### Scenario: Last five minutes

- **WHEN** a pod log view is open
- **AND** the user presses `2`
- **THEN** the buffer is reloaded with lines from the last 5 minutes and the flash shows `showing last 5m`

### Requirement: Lookback prompt

`T` in a kubelet log view SHALL open the prompt `lookback: s/m/h/d or tail (current: <value>)`; a valid duration SHALL restart the streams from that lookback, `tail` SHALL restore the plain tail, and invalid input SHALL warn `lookback: <error>; use s/m/h/d or tail` and keep the stream. The choice SHALL apply to the current view only.

#### Scenario: Invalid lookback

- **WHEN** the user presses `T` and enters `soon`
- **THEN** the flash warns with `use s/m/h/d or tail`
- **AND** the existing stream keeps running

### Requirement: Log view title flags

The log view title SHALL show its state as flags: ` JSON`, ` [warn/error]`, one of ` ⏹stopped`, ` ▶follow` or ` ⏸paused`, ` wrap`, ` ts`, and ` ⏱<period>` for a kubelet time anchor.

#### Scenario: Paused and wrapped

- **WHEN** the user scrolls up and presses `w`
- **THEN** the title contains `⏸paused` and `wrap`

### Requirement: Log fullscreen

`F` SHALL toggle log fullscreen with the flash `fullscreen: on` or `fullscreen: off`, hiding borders, header, status and scrollbars while keeping the title, filter prompt and lookback prompt. Log views SHALL start fullscreen when `[logs] fullscreen = true`, and the choice SHALL persist for the session.

#### Scenario: Toggle fullscreen

- **WHEN** a log view is open
- **AND** the user presses `F`
- **THEN** the log pane fills the terminal

### Requirement: Per-view log state

Opening a new log view SHALL reset the text filter, warning filter, follow and stopped state, while JSON formatting, wrap, timestamps and fullscreen SHALL keep their session values.

#### Scenario: New view keeps JSON

- **WHEN** JSON formatting is on and a filter is active
- **AND** the user opens logs for another pod
- **THEN** JSON formatting is still on and no filter is active

### Requirement: Leave log view

`esc` or `q` SHALL stop all streams and return to the view that opened the logs, reselecting the same table row.

#### Scenario: Back to the pod

- **WHEN** the user opens logs from the second pod row
- **AND** presses `q`
- **THEN** the table shows the second row selected and streaming stops

### Requirement: Log text selection

The log view SHALL release mouse capture so the terminal's own text selection works, with the wheel scrolling the log within its bounds.

#### Scenario: Fast wheel burst

- **WHEN** the user scrolls the wheel down quickly past the end
- **THEN** the view stays in the log view at its last page

### Requirement: VictoriaLogs entry points

`L` and the `:vlogs`, `:plogs` and `:providerlogs` commands SHALL open VictoriaLogs history for the selected pod, workload, Service or Namespace, titled `<target> — victorialogs (<lookback>)`; other kinds SHALL warn `provider logs cover pods, workloads, services, and namespaces`.

#### Scenario: Pod history

- **WHEN** a pod `api-1` is selected
- **AND** the user presses `L`
- **THEN** a log view titled `api-1 — victorialogs (1h)` opens

### Requirement: VictoriaLogs configuration

`[providers.logs]` SHALL accept `type = "victorialogs"`, an optional `url`, `lookback` (default `1h`), `limit` (default 300), `headers` and `fields`. A missing or unsupported type, a URL without `http://` or `https://`, or an invalid lookback SHALL disable the provider with a warning; invalid headers or field names SHALL be skipped with a warning.

#### Scenario: Bad URL

- **WHEN** `[providers.logs] url = "vlogs:9428"` is configured
- **THEN** a warning says the url must start with http:// or https://

### Requirement: VictoriaLogs autodiscovery

Without a configured `url`, the first VictoriaLogs view SHALL find a VictoriaLogs Service in the cluster by its well-known labels, reach it through the API server's service proxy, show `[provider] using <location>`, and reuse it for the rest of the session; failure SHALL show `[error] <error>`.

#### Scenario: Nothing to discover

- **WHEN** no VictoriaLogs Service exists and no url is configured
- **AND** the user presses `L`
- **THEN** the view shows an error suggesting `[providers.logs] url`

### Requirement: VictoriaLogs field detection

Without configured `fields`, the first VictoriaLogs query SHALL detect the namespace, pod and container field names from the backend's stream fields, showing `[provider] detected log fields: <names>` when they differ from the defaults, or a `[provider]` warning line naming the fallback fields when none match or detection fails.

#### Scenario: fluent-bit fields

- **WHEN** the backend's stream fields use `kubernetes_pod_name` naming
- **AND** the user opens VictoriaLogs history
- **THEN** the view shows `[provider] detected log fields: ...` and queries with those names

### Requirement: VictoriaLogs backfill and tail

A VictoriaLogs view SHALL first show up to `limit` of the newest entries within the lookback, oldest first, or `(no logs in the last <lookback>)` when there are none, then follow the live tail, skipping entries not newer than the backfill and showing `[provider] log stream ended` when the backend closes the tail.

#### Scenario: Empty history

- **WHEN** a pod has no logs in the backend for the last hour
- **AND** the user presses `L`
- **THEN** the view shows `(no logs in the last 1h)`

### Requirement: VictoriaLogs errors

A failing VictoriaLogs query SHALL show `[error] <error>`, a failing live tail SHALL show `[error] live tail unavailable: <error>`, and failing to list a workload's pods SHALL show `[error] listing pods: <error>`.

#### Scenario: Tail unavailable

- **WHEN** the backfill succeeds but the tail request fails
- **THEN** the backfilled lines stay and the view shows `[error] live tail unavailable: <error>`

### Requirement: VictoriaLogs lookback

In a VictoriaLogs view, `T` SHALL prompt `lookback period: e.g. 30m, 4h, 2d (current: <value>)`; a valid duration SHALL re-query, retitle the view, flash `lookback: <period>`, and apply to later `L` presses in the session. Keys `1` to `5` SHALL set the lookback to 1m, 5m, 15m, 30m or 1h, and `0` SHALL restore `1h`.

#### Scenario: Look back four hours

- **WHEN** a VictoriaLogs view is open
- **AND** the user presses `T` and enters `4h`
- **THEN** the view re-queries and its title ends with `victorialogs (4h)`
