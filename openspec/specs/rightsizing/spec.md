# rightsizing Specification

## Purpose

Historical right-sizing: `:rightsize` estimates CPU and memory requests for a workload or pod from past usage in a Prometheus-compatible metrics backend, configured by `[providers.metrics]` or autodiscovered, and previews a patch without mutating the cluster.

## Requirements

### Requirement: Rightsize command

The `:rightsize` command (aliases `:sizing`, `:vpa`) SHALL right-size the selected row when the current resource is pods, deployments, statefulsets, daemonsets or replicasets. It MUST warn "no selection to right-size" when no row is selected and "right-size applies to workloads (deploy/sts/ds/rs) and pods" for any other kind.

#### Scenario: Unsupported kind

- **WHEN** the user runs `:rightsize` on a Service
- **THEN** the warning "right-size applies to workloads (deploy/sts/ds/rs) and pods" is shown
- **AND** no metrics query is sent

#### Scenario: Empty table

- **WHEN** no row is selected and the user runs `:rightsize`
- **THEN** the warning "no selection to right-size" is shown

### Requirement: Containers analysed

Right-sizing SHALL analyse every container listed in `spec.containers` of a Pod or `spec.template.spec.containers` of a workload, reading each container's current CPU and memory requests. Init and ephemeral containers MUST NOT be analysed. When no containers are found it MUST warn "no containers found to right-size".

#### Scenario: Two containers

- **WHEN** the selected Deployment has containers `app` and `sidecar`
- **THEN** the report has a section for each container

### Requirement: Pod name matching

Usage SHALL be matched to pods in the object's namespace by an anchored pod-name regex built from the literal object name plus a controller suffix: none for pods, `-[a-z0-9]{1,10}-[a-z0-9]{5}` for deployments, `-[0-9]+` for statefulsets, and `-[a-z0-9]{5}` for daemonsets and replicasets. Workloads with a longer name prefix MUST NOT match.

#### Scenario: Sibling workload excluded

- **WHEN** the user right-sizes Deployment `api` in a namespace that also has Deployment `api-worker`
- **THEN** the queries select pods matching `^api-[a-z0-9]{1,10}-[a-z0-9]{5}$`
- **AND** `api-worker` pods do not match

#### Scenario: Dots matched literally

- **WHEN** the user right-sizes pod `web.v1`
- **THEN** the pod regex is `^web\.v1$`

### Requirement: Usage queries

For each container sofka SHALL send eight instant queries concurrently, selecting `namespace`, `pod=~` (the name regex) and `container` labels: CPU P50/P95/P99 as `max(quantile_over_time(q, rate(container_cpu_usage_seconds_total{…}[step])[window:step]))*1000`, memory P50/P95/P99 as `max(quantile_over_time(q, container_memory_working_set_bytes{…}[window]))`, and sums of `increase()` of `container_oom_events_total` and `container_cpu_cfs_throttled_periods_total` over the window.

#### Scenario: CPU P95 query

- **WHEN** right-sizing container `app` of pod `api-0` in namespace `prod` with defaults
- **THEN** a query `max(quantile_over_time(0.95, rate(container_cpu_usage_seconds_total{namespace="prod",pod=~"^api-0$",container="app"}[5m])[7d:5m]))*1000` is sent

### Requirement: Query transport

Each query SHALL be sent as an HTTP POST of form field `query` to `<base>/api/v1/query` with the configured `headers`, and MUST time out after 30 seconds. Only a response with status `success` and a first sample value SHALL count as data; an empty or malformed result SHALL count as no data, and a non-2xx response SHALL fail with "HTTP <status>: <first 200 characters of body>".

#### Scenario: Empty result

- **WHEN** the backend returns `{"status":"success","data":{"result":[]}}`
- **THEN** that value is shown as "—" and no query error is counted

#### Scenario: Slow backend

- **WHEN** a query takes longer than 30 seconds
- **THEN** it fails with "query timed out after 30s"

### Requirement: Rightsize report view

While gathering, the status SHALL read "right-sizing <name> over <window>…". The result SHALL open in the document view titled "right-size — <Kind>/<name>" and begin with "source: <backend location>" and "window: <window> headroom: +<headroom>% over P95". Document scrolling, search and copy belong to the document-views and selection-clipboard capabilities.

#### Scenario: Report header

- **WHEN** right-sizing Deployment `api` with a configured URL `https://prom.example.com`
- **THEN** the view is titled "right-size — Deployment/api"
- **AND** starts with "source: https://prom.example.com" and "window: 7d headroom: +15% over P95"

### Requirement: Per-container report lines

For each container the report SHALL show "container: <name>" and a `cpu` and a `mem` line with "request", "P50", "P95", "P99", "→ suggest" and a bracketed verdict, using "—" for any unknown value. When OOM kills or throttled periods are above zero it SHALL add "evidence: N OOM kill(s) · M throttled CPU period(s) over <window>".

#### Scenario: OOM evidence

- **WHEN** the OOM query returns 2 and the throttling query returns 0
- **THEN** the container shows "evidence: 2 OOM kill(s) · 0 throttled CPU period(s) over 7d"

### Requirement: Suggested requests

The suggested request SHALL be P95 × (1 + headroom/100), computed only when P95 is known and above zero. CPU SHALL be rounded up to whole millicores (`Nm`, at least `1m`) and memory up to whole mebibytes (`NMi`, at least `1Mi`).

#### Scenario: Suggest with default headroom

- **WHEN** a container's CPU P95 is 100m and memory P95 is 128Mi with headroom 15
- **THEN** the suggestions are `115m` and `148Mi`

### Requirement: Verdicts

Each resource line SHALL carry a verdict: "no data" without a P95; "no request set" when the request is missing or not above zero; "under-provisioned" when P95 exceeds the request; "over-provisioned" when the request exceeds 1.5 × P95; otherwise "sized about right". Any OOM kill in the window MUST force the memory verdict to "under-provisioned".

#### Scenario: Over-provisioned CPU

- **WHEN** a container requests 500m CPU and its CPU P95 is 100m
- **THEN** the cpu line ends with "[over-provisioned]"

#### Scenario: OOM forces under-provisioned

- **WHEN** a container's memory P95 is well below its request but the OOM count is 1
- **THEN** the mem line ends with "[under-provisioned]"

### Requirement: Failed queries are unknown, not zero

A failed query MUST leave its value unknown ("—") and MUST NOT be treated as zero usage or used for a suggestion. When any query fails the report SHALL show "⚠ N metrics query failed" (or "queries") followed by "— '—' below means unknown, not zero" and "first error: <error>", and the status SHALL warn "N metrics queries failed".

#### Scenario: One failed query

- **WHEN** one of the eight queries returns HTTP 500
- **THEN** the report shows "⚠ 1 metrics query failed — '—' below means unknown, not zero"
- **AND** the affected value shows "—"

### Requirement: Patch preview

When any container has a suggestion, the report SHALL end with "suggested patch (preview only; copy with c, apply with kubectl patch):" and a pretty-printed strategic-merge patch setting `resources.requests` `cpu`/`memory` per container by name, under `spec.containers` for a Pod or `spec.template.spec.containers` for a workload, listing only containers and resources with a suggestion. Otherwise it SHALL say "no usage data found — is the workload scraped by the metrics backend?".

#### Scenario: Workload patch

- **WHEN** container `app` of a Deployment gets suggestions `115m` and `148Mi`
- **THEN** the patch is `{"spec":{"template":{"spec":{"containers":[{"name":"app","resources":{"requests":{"cpu":"115m","memory":"148Mi"}}}]}}}}` pretty-printed

#### Scenario: No data

- **WHEN** every query returns an empty result
- **THEN** the report says "no usage data found — is the workload scraped by the metrics backend?"

### Requirement: Rightsize never mutates

`:rightsize` MUST NOT write to the cluster. It SHALL only list Services for discovery and send read-only metrics queries, and it SHALL be available in read-only mode.

#### Scenario: Read-only session

- **WHEN** sofka runs with `--readonly` and the user runs `:rightsize`
- **THEN** the report is produced and no object is modified

### Requirement: Metrics provider configuration

`[providers.metrics]` SHALL accept `type` (`"prometheus"` or `"victoriametrics"`, required when the section exists), `url` (an `http://` or `https://` base URL, trailing `/` trimmed; omit to autodiscover), `window` (default `7d`), `step` (default `5m`), `headroom` percent (default `15`) and a `headers` table sent with every query. It SHALL be resolvable per cluster and per context like other config sections.

#### Scenario: Custom window

- **WHEN** the config sets `window = "24h"` and `headroom = 30`
- **THEN** the report header reads "window: 24h headroom: +30% over P95"

### Requirement: Durations

`window` and `step` SHALL be a positive integer followed by `s`, `m`, `h` or `d`, with a bare number meaning seconds. Any other value MUST be rejected with a warning.

#### Scenario: Week unit rejected

- **WHEN** the config sets `window = "1w"`
- **THEN** a warning starting "providers.metrics: window:" reports the unknown unit

### Requirement: Invalid metrics configuration

An invalid `[providers.metrics]` section SHALL produce exactly one warning and be ignored, so `:rightsize` falls back to autodiscovery with default settings. Warnings SHALL be "providers.metrics: missing `type` (expected "prometheus" or "victoriametrics")", "providers.metrics: unsupported type …", "providers.metrics: url … must start with http:// or https:// (or be omitted for autodiscovery)", "providers.metrics: window: …" or "providers.metrics: step: …".

#### Scenario: Missing type

- **WHEN** `[providers.metrics]` sets `url` but no `type`
- **THEN** sofka warns "providers.metrics: missing `type` (expected \"prometheus\" or \"victoriametrics\")"
- **AND** `:rightsize` uses autodiscovery

### Requirement: Configuration warnings surfaced

Metrics provider warnings SHALL be printed to stderr at startup, and `:reload` SHALL include them in the in-app config validation warnings. A context switch SHALL apply the new context's resolved section without reporting its warnings.

#### Scenario: Reload with a bad URL

- **WHEN** the user changes `url` to `prom.example.com` and runs `:reload`
- **THEN** the reload reports the "must start with http:// or https://" warning

### Requirement: Metrics backend autodiscovery

Without a configured `url`, each `:rightsize` SHALL list Services in all namespaces by label selector, trying `app.kubernetes.io/name=vmsingle`, `app.kubernetes.io/name=victoria-metrics-single`, `app.kubernetes.io/name=prometheus`, then `operated-prometheus=true`. From the first selector with a match it SHALL pick the first Service by namespace then name, using the port named `http`, else port 9090, 8428 or 8429, else the first port.

#### Scenario: Prometheus operator service

- **WHEN** no VictoriaMetrics service exists and Service `monitoring/prometheus-operated` has label `operated-prometheus=true` and port 9090
- **THEN** right-size queries that service on port 9090

### Requirement: API-server proxy transport

An autodiscovered backend SHALL be queried through the API-server service proxy at `/api/v1/namespaces/<ns>/services/<svc>:<port>/proxy/api/v1/query`, and the report source SHALL read "<ns>/<svc>:<port> (API-server proxy)".

#### Scenario: Proxy source line

- **WHEN** autodiscovery picks `monitoring/vmsingle:8428`
- **THEN** the report starts with "source: monitoring/vmsingle:8428 (API-server proxy)"

### Requirement: Autodiscovery failure

When autodiscovery fails, the document SHALL show "no metrics backend: <error>" and the status SHALL warn "right-size needs Prometheus/VictoriaMetrics". The error SHALL be "discovering metrics services: <error>" when listing Services fails, or "no Prometheus/VictoriaMetrics service found — set [providers.metrics] url in config.toml" when nothing matches.

#### Scenario: No backend in cluster

- **WHEN** no Service matches any discovery selector
- **THEN** the view shows "no metrics backend: no Prometheus/VictoriaMetrics service found — set [providers.metrics] url in config.toml"
