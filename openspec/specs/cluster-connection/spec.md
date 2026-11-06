# cluster-connection Specification

## Purpose

Describe how sofka loads kubeconfig, authenticates and reaches the Kubernetes API server through proxies and TLS, reports connection failures, and keeps resource watches alive through errors, reconnects and system sleep.

## Requirements

### Requirement: Default context connection

Without `--context`, sofka SHALL connect to the kubeconfig `current-context` from `KUBECONFIG` (including a merged list of files) or `~/.kube/config`. It MUST NOT change the kubeconfig `current-context`.

#### Scenario: Current context

- **WHEN** the kubeconfig `current-context` is `dev`
- **AND** the user runs `sofka`
- **THEN** sofka connects to the cluster of context `dev`

### Requirement: Context option

`--context NAME` SHALL connect to the named kubeconfig context instead of the current context, without changing the kubeconfig. A failure to build the client for that context MUST be reported with `building config for context 'NAME'` and the cause.

#### Scenario: Named context

- **WHEN** the user runs `sofka --context west`
- **THEN** sofka connects to the cluster of context `west`
- **AND** the kubeconfig `current-context` is unchanged

### Requirement: Kubeconfig option

`--kubeconfig PATH` SHALL make sofka use that file by setting `KUBECONFIG` for the whole process, so that the connection, context switches, completion and `kubectl` commands run by sofka all read the same file.

#### Scenario: kubectl uses the same file

- **WHEN** the user runs `sofka --kubeconfig ./other.yaml`
- **AND** opens a describe view that runs `kubectl`
- **THEN** `kubectl` reads `./other.yaml`

### Requirement: kubectl context pinning

When sofka is connected through a named kubeconfig context, every `kubectl` command it runs SHALL pass `--context` with that context name, so a different kubeconfig `current-context` does not redirect it.

#### Scenario: Switched context

- **WHEN** sofka is connected to `west` while the kubeconfig `current-context` is `east`
- **THEN** `kubectl` commands that sofka runs target `west`

### Requirement: In-cluster fallback

When no usable kubeconfig is found, sofka SHALL connect with the in-cluster service account configuration. When the kubeconfig has no `current-context` but in-cluster configuration succeeds, sofka MUST still connect.

#### Scenario: Running inside a pod

- **WHEN** sofka runs in a pod with a service account and no kubeconfig
- **THEN** sofka connects to the API server of that cluster

### Requirement: Kubeconfig load errors

When neither kubeconfig nor in-cluster configuration can be loaded for a reason other than a missing current context, sofka MUST report `loading kubeconfig (is KUBECONFIG / ~/.kube/config present?)` followed by the root cause. An unreadable kubeconfig for a named context MUST be reported with `reading kubeconfig` and the cause.

#### Scenario: Malformed kubeconfig

- **WHEN** the kubeconfig file is not valid YAML
- **THEN** the error starts with `loading kubeconfig` or `reading kubeconfig`
- **AND** includes the parse error

### Requirement: Certificate data line breaks

sofka SHALL accept base64 `certificate-authority-data`, `client-certificate-data` and `client-key-data` that contain CR or LF line breaks, including values written with the YAML `!!binary` tag. Other invalid base64 MUST still be rejected.

#### Scenario: Wrapped base64

- **WHEN** `client-certificate-data` is base64 wrapped at 64 characters with CRLF line breaks
- **THEN** sofka connects with that certificate

### Requirement: Credential file and inline data

sofka SHALL accept client certificates and keys from kubeconfig file paths or inline data. When both are present, inline data MUST take precedence.

#### Scenario: Inline wins

- **WHEN** a user entry sets `client-certificate` to a missing file and `client-certificate-data` to a valid certificate
- **THEN** sofka connects with the inline certificate

### Requirement: Empty bearer token

An empty kubeconfig `token` SHALL be ignored, so client certificate authentication still applies and no `Authorization` header is sent.

#### Scenario: Empty token with certificate

- **WHEN** a user entry has a client certificate and `token: ""`
- **THEN** requests authenticate with the client certificate only

### Requirement: Non-interactive exec credentials

sofka SHALL run kubeconfig exec credential plugins non-interactively, whatever `interactiveMode` the kubeconfig sets, so a plugin cannot prompt on the terminal that sofka owns.

#### Scenario: Always interactive plugin

- **WHEN** an exec plugin sets `interactiveMode: Always`
- **THEN** sofka runs it with interactive input disabled

### Requirement: Exec authentication hints

When an exec credential plugin fails while connecting or while a watch authenticates, sofka MUST report `Authentication failed. Run 'aws sso login' for the active AWS profile, then retry.` if the plugin stderr mentions SSO (case-insensitive), and `Authentication command failed. Log in with your credential provider, then retry.` otherwise.

#### Scenario: Expired AWS SSO session

- **WHEN** the exec plugin exits with an error and stderr `Error loading SSO Token: Token has expired`
- **THEN** sofka shows the `aws sso login` hint

### Requirement: OIDC token refresh

For kubeconfig `auth-provider` entries named `oidc`, sofka SHALL use a valid cached `id-token` and SHALL refresh an expired one through the issuer's `/.well-known/openid-configuration` and a `refresh_token` grant with client credentials. Refresh MUST work with or without a configured CA for the identity provider.

#### Scenario: Expired id-token

- **WHEN** the configured `id-token` has expired and a `refresh-token` is set
- **THEN** sofka obtains a new token from the issuer and connects

### Requirement: Kubeconfig proxy URL

A non-empty `proxy-url` on the selected kubeconfig cluster SHALL take priority over environment proxy settings and `NO_PROXY`. sofka MUST support `http`, `https` and `socks5` proxy URLs, MUST send user information in the URL as Basic proxy authorization, and MUST reject other schemes with `unsupported proxy protocol`.

#### Scenario: Explicit proxy beats exclusions

- **WHEN** the cluster sets `proxy-url: http://proxy:3128`
- **AND** `NO_PROXY` matches the API server
- **THEN** sofka still connects through `proxy:3128`

### Requirement: Environment proxy

Without a kubeconfig `proxy-url`, sofka SHALL use the environment proxy (such as `HTTPS_PROXY`) unless an exclusion matches the API server. Exclusions MUST come from `NO_PROXY`, or from `no_proxy` when `NO_PROXY` is unset or contains only whitespace. These rules MUST apply at startup and on every context switch.

#### Scenario: Lowercase fallback

- **WHEN** `NO_PROXY` is set to spaces and `no_proxy=api.internal`
- **AND** the API server is `https://api.internal:6443`
- **THEN** sofka connects directly

### Requirement: NO_PROXY address entries

`NO_PROXY` entries are comma-separated and SHALL match case-insensitively. `*` MUST match every server; a CIDR or IP entry MUST match only a literal server IP without name resolution; `[ipv6]` with an optional port and `host:port` MUST match when the port equals the server port, which defaults to 443 for `https` and 80 for `http`. An empty or invalid port and URL-form entries MUST never match.

#### Scenario: Default HTTPS port

- **WHEN** `NO_PROXY=api.internal:443`
- **AND** the API server is `https://api.internal`
- **THEN** sofka connects directly

### Requirement: NO_PROXY domain entries

A bare domain entry such as `example.com` SHALL match that host and its subdomains. Entries `.example.com` and `*.example.com` SHALL match subdomains only, and `*example.com` MUST match nothing.

#### Scenario: Leading dot

- **WHEN** `NO_PROXY=.example.com`
- **THEN** `api.example.com` connects directly
- **AND** `example.com` still uses the proxy

### Requirement: Client certificates through proxies

Client certificate authentication SHALL work through `http` and `socks5` proxies, with TLS to the API server established end to end through the tunnel. For an `https` proxy, the cluster client certificate MUST NOT be sent to the proxy.

#### Scenario: SOCKS5 tunnel

- **WHEN** the cluster sets `proxy-url: socks5://127.0.0.1:1080` and the user has a client certificate
- **THEN** sofka connects and authenticates with that certificate

### Requirement: X.509 v1 client certificates

sofka MUST reject a static X.509 v1 client certificate by default with `client certificate is X.509 v1; use a v3 certificate or pass --allow-v1-client-cert for this run`. With `--allow-v1-client-cert`, sofka SHALL use it for this run, including context switches and completion. The flag MUST NOT disable server certificate checks.

#### Scenario: Consent flag

- **WHEN** the kubeconfig user has a v1 client certificate
- **AND** the user runs `sofka --allow-v1-client-cert`
- **THEN** sofka connects with that certificate

### Requirement: v1 certificate limits

With `--allow-v1-client-cert`, sofka MUST still reject a certificate whose private key does not match (`client certificate does not match its private key`), malformed or v2 certificates, v1 certificates with extensions or unique IDs, and v1 certificates returned by exec credential plugins. v3 certificates MUST use the normal path with or without the flag.

#### Scenario: Exec-issued v1 certificate

- **WHEN** an exec plugin returns a v1 client certificate
- **THEN** sofka fails with an error stating that `--allow-v1-client-cert` supports static kubeconfig client certificates only

### Requirement: TLS session resumption

By default sofka SHALL resume TLS sessions. `--no-tls-resumption` MUST disable resumption for the whole run, including context switches and completion queries, so every new connection performs a full handshake with client certificate authentication.

#### Scenario: Server rejects resumed sessions

- **WHEN** the API server answers resumed TLS sessions with HTTP 401
- **AND** the user runs `sofka --no-tls-resumption`
- **THEN** repeated requests succeed

### Requirement: Exact server certificate pin

When a server certificate fails standard verification only because its issuer is unknown or because a CA certificate is used as the server certificate, sofka SHALL accept it if it exactly matches a certificate in the kubeconfig `certificate-authority` or `certificate-authority-data`. Hostname (including `tls-server-name`), validity dates, key usage and TLS signature checks MUST still apply.

#### Scenario: Teleport local proxy

- **WHEN** `tsh proxy kube` serves a CA certificate as its server certificate
- **AND** the kubeconfig CA data contains exactly that certificate
- **THEN** sofka connects without extra flags

### Requirement: Server certificate pin limits

A certificate that shares only the key or subject with a configured CA MUST NOT qualify for the exact pin. The pin SHALL apply with static, exec and `oidc` auth-provider credentials; other auth-providers, `insecure-skip-tls-verify` and system trust MUST keep standard verification.

#### Scenario: Same key, different certificate

- **WHEN** the server certificate has the same key as the configured CA but different contents
- **THEN** the connection fails verification

### Requirement: Server verification failures

sofka MUST fail the connection when the server certificate has an unknown issuer, does not match the host name or has expired, whether or not `--allow-v1-client-cert` or `--no-tls-resumption` is given.

#### Scenario: Wrong host name

- **WHEN** `tls-server-name` does not match the server certificate
- **THEN** the connection fails with a host name error

### Requirement: Dead connection detection

sofka SHALL enable TCP keepalive on API server connections so a connection to a peer that stopped answering fails within about a minute, without ending quiet watch, log or port-forward streams.

#### Scenario: NAT drops the flow

- **WHEN** a NAT silently drops an idle watch connection
- **THEN** the connection fails within about a minute and the watch retries

### Requirement: Uncompressed watches

Watch requests SHALL ask for uncompressed responses (`Accept-Encoding: identity`) to avoid gzip stream errors. List requests MUST keep gzip compression unless the kubeconfig disables it.

#### Scenario: Watch request headers

- **WHEN** sofka starts a watch
- **THEN** the request sends `Accept-Encoding: identity`

### Requirement: Server version

sofka SHALL read the server version in parallel with API discovery with a 2-second limit. A failed or slow `/version` request MUST NOT fail the connection and SHALL leave the version unknown. The version text MUST have control characters removed and be cut to 128 characters with `…`.

#### Scenario: Slow version endpoint

- **WHEN** `/version` does not answer within 2 seconds
- **THEN** sofka connects with an unknown server version

### Requirement: Request latency measurement

sofka SHALL measure API request latency up to the response headers for each operation class. Cancelled requests MUST count as errors and 5xx responses as failures. At debug log level sofka SHALL log `request.start`, `request.done`, `request.failed` and `request.cancelled` events.

#### Scenario: Server error

- **WHEN** a list request returns HTTP 503
- **THEN** that operation class records a failure in its latency summary

### Requirement: Disconnected identity

When sofka starts without a connection, the header SHALL show the context and cluster names from the kubeconfig for the requested context, or for the current context when none was requested.

#### Scenario: Failed launch

- **WHEN** connecting to `--context west` fails at launch
- **THEN** the header shows context `west` and its cluster name

### Requirement: Streaming list watches

Watches SHALL first use streaming lists (`sendInitialEvents=true`). When the server answers 400, 404, 405 or 422 while the watch starts, sofka MUST fall back to list and watch and remember that the cluster does not support streaming lists. 401, 403, 429 and 5xx answers MUST NOT trigger the fallback.

#### Scenario: Old API server

- **WHEN** the API server rejects `sendInitialEvents` with HTTP 400
- **THEN** the watch lists and then watches
- **AND** later watches on that cluster skip the streaming attempt

### Requirement: Watch retry backoff

A failing watch SHALL retry with growing delays of about 0.8, 1.6 and 3.2 seconds, up to 30 seconds, and the delay MUST reset after the watch makes progress.

#### Scenario: Repeated 401

- **WHEN** a watch fails three times with HTTP 401 and then succeeds
- **THEN** each wait between failures is at least twice the previous one, starting at 0.8 seconds

### Requirement: Expired watch resume

An expired watch (`410 Gone`, too old resource version) SHALL re-list without an error flash. After a streaming re-list, objects deleted in the meantime MUST disappear from the view.

#### Scenario: Deleted during desync

- **WHEN** a watch expires and an object is deleted before the re-list completes
- **THEN** the object disappears from the view and no error flash appears

### Requirement: Watch error flash

A watch error SHALL flash `watch failed; retrying: <error>`. The flash MUST stay past the normal flash timeout and through re-lists, and MUST clear only when the watch recovers (a successful watch response or a new event). A different later flash MUST NOT be cleared by that recovery.

#### Scenario: Recovery clears the error

- **WHEN** a watch fails and then delivers a new event
- **THEN** the `watch failed; retrying:` flash clears

### Requirement: Namespace pattern watch errors

With a namespace pattern, a watch error SHALL flash `<namespace>: <error>; results incomplete`. When that namespace recovers and another namespace still has an error, sofka MUST show the remaining namespace error.

#### Scenario: Two failing namespaces

- **WHEN** watches fail in `a-crons` and `b-crons`
- **AND** `a-crons` recovers
- **THEN** the flash shows the `b-crons` error with `results incomplete`

### Requirement: Watch errors follow the view

Changing the resource view or refreshing SHALL clear the previous watch error, and results or errors from a previous view's watch MUST be ignored.

#### Scenario: Leave a failing view

- **WHEN** the Pods watch is failing
- **AND** the user opens Services
- **THEN** the Pods watch error is no longer shown

### Requirement: Watch error counts

sofka SHALL count watch errors and keep the last watch error for diagnostics.

#### Scenario: Error recorded

- **WHEN** a watch fails
- **THEN** the diagnostics report shows an increased watch error count and the last error

### Requirement: Reconnect after sleep

On Linux, macOS and Android, sofka SHALL detect a system sleep of at least 30 seconds (wall-clock changes MUST NOT count) and restart the current watch once, flashing `reconnected after sleep` unless an error flash is showing.

#### Scenario: Laptop wakes

- **WHEN** the machine sleeps for five minutes and wakes with sofka on Pods
- **THEN** the Pods watch restarts
- **AND** the flash shows `reconnected after sleep`

### Requirement: Deferred sleep reconnect

The sleep reconnect MUST wait while an overlay is open, while a non-table view is active, while no resource is selected, or while a context switch is running. A completed context switch MUST cancel the pending reconnect; a failed switch MUST keep it and keep the failure flash.

#### Scenario: Wake inside an overlay

- **WHEN** the machine wakes while the help overlay is open
- **AND** the user closes the overlay
- **THEN** the watch restarts once

### Requirement: Context switch connection failure

A failed context switch SHALL flash `context switch failed: <error>` and sofka MUST stay on the previous context. When sofka has never connected, the context picker MUST open again. TLS flags from the command line MUST apply to every switch.

#### Scenario: Unreachable target

- **WHEN** the user switches to a context whose API server is unreachable
- **THEN** the flash shows `context switch failed:` with the cause
- **AND** sofka stays on the previous context
