# safety Specification

## Purpose

Protect clusters from accidental or unauthorised changes made through sofka: read-only mode, configurable guardrails, confirmation policies, `:can-i` permission checks, the action journal, and the redaction guarantees that keep credentials and addresses out of what sofka shows or writes.

## Requirements

### Requirement: Read-only mode from configuration

When no `--readonly` or `--write` flag is given, sofka SHALL take read-only mode from the resolved `readonly` config value for the active context, including per-cluster and per-context overrides. It SHALL resolve the value again on every context switch and on `:reload`.

#### Scenario: Per-cluster override enables read-only on switch

- **WHEN** no CLI read-only flag was given
- **AND** the per-cluster config for the target cluster sets `readonly = true`
- **AND** the user switches to that context
- **THEN** the session is in read-only mode

### Requirement: CLI read-only pin wins over configuration

`--readonly` SHALL force read-only mode and `--write` SHALL force write mode for the whole session, overriding the config value and all per-cluster and per-context overrides, on every context switch and on `:reload`. If both flags are given, `--readonly` MUST win.

#### Scenario: --write survives switching into a read-only cluster

- **WHEN** sofka was started with `--write`
- **AND** the user switches to a context whose config sets `readonly = true`
- **THEN** the session stays in write mode

#### Scenario: Both flags given

- **WHEN** sofka is started with both `--readonly` and `--write`
- **THEN** the session is in read-only mode

### Requirement: Read-only indicator

While read-only mode is active, the header SHALL show ` [read-only]` after the context name in an error colour, the compact header SHALL show ` [ro]`, and the `:info` report SHALL show `readonly: true`.

#### Scenario: Header marks read-only sessions

- **WHEN** the session is in read-only mode
- **THEN** the header shows `[read-only]` next to the context

### Requirement: Read-only blocks mutating actions

In read-only mode, every cluster-mutating or container-entering action SHALL be refused before any confirmation, prompt, editor or shell opens, with the error flash `read-only mode — action disabled`. This covers delete, force-delete, cordon/uncordon, drain, attach, restart, set image, edit, exec/shell, debug, debug cleanup, node debug, scale, the `t` Flux/CronJob/ArgoCD action menu, Helm rollback and uninstall, and Elasticsearch refresh.

#### Scenario: Delete, edit and shell are refused

- **WHEN** the session is in read-only mode with a pod selected
- **AND** the user requests delete, edit or shell
- **THEN** no confirmation dialog opens and nothing shells out
- **AND** the flash contains `read-only`

### Requirement: Read-only keeps read paths open

Read-only mode MUST NOT block viewing actions such as describe, YAML, logs and events.

#### Scenario: Describe still works

- **WHEN** the session is in read-only mode
- **AND** the user presses `d` on a selected pod
- **THEN** no read-only refusal is flashed

### Requirement: Read-only blocks PVC and file-upload mutations

In read-only mode, PVC shell, PVC helper pod creation, PVC upload, `:pvc-clean`, and "Upload to pod" in the file transfer menu SHALL be refused with a read-only flash. The transfer menu itself SHALL still open and downloads SHALL still work. Browsing a PVC that needs a helper pod SHALL be refused with a message that read-only mode blocks the helper pod.

#### Scenario: Upload refused, menu still opens

- **WHEN** the session is in read-only mode with a pod selected
- **AND** the user presses `t`
- **THEN** the transfer menu opens
- **AND** choosing "Upload to pod" flashes a read-only error and no prompt opens

### Requirement: Read-only blocks context rename

In read-only mode, opening the rename-context prompt from the context switcher SHALL be refused with `read-only mode — action disabled`.

#### Scenario: Rename refused

- **WHEN** the session is in read-only mode
- **AND** the user presses `r` in the context switcher
- **THEN** no rename prompt opens and the read-only flash is shown

### Requirement: Read-only gates plugins by declared intent

In read-only mode, a plugin whose `mutating` is unset or `true` SHALL be refused with `read-only mode — plugin '<name>' may mutate (set mutating = false to allow)`. A plugin with `network_load = true` SHALL be refused with `read-only mode — plugin '<name>' generates network load`. A plugin with `mutating = false` and no network load SHALL stay available. Refusal MUST happen before any confirmation.

#### Scenario: Mutating plugin blocked, read-only plugin runs

- **WHEN** the session is in read-only mode
- **AND** the user triggers a plugin without `mutating = false`
- **THEN** the plugin does not run and the flash contains `read-only`
- **AND** triggering a plugin with `mutating = false` runs it

#### Scenario: Bundled sanitize refused in read-only

- **WHEN** the session is in read-only mode
- **AND** the user runs `:sanitize`
- **THEN** no confirmation opens and the flash contains `read-only`

### Requirement: Drain rechecks read-only at every step

Drain SHALL check read-only mode when the drain dialog is requested, when its options are confirmed, and again just before running. If read-only mode is active at any of those points, no eviction or deletion request SHALL be sent.

#### Scenario: Read-only enabled during typed confirmation

- **WHEN** a drain typed-confirmation prompt is open
- **AND** read-only mode becomes active before the user submits the correct name
- **THEN** no request is sent to the cluster and the drain does not start

### Requirement: Guardrail rule matching

Each `[[guardrails]]` rule SHALL apply to an action when its `contexts`, `actions` and `resources` glob lists each match the current context, action name and resource plural, and its `namespaces` list matches the namespace of at least one target. An empty list SHALL match anything.

#### Scenario: Namespace list matches any target

- **WHEN** a rule lists `namespaces = ["prod"]`
- **AND** a bulk action targets one object in `prod` and one in `dev`
- **THEN** the rule applies to the action

### Requirement: Guardrail glob syntax

In guardrail patterns, `*` SHALL match any run of characters and every other character SHALL match literally. A pattern without `*` MUST match only the exact value.

#### Scenario: Exact and wildcard patterns

- **WHEN** a rule lists contexts `prod-*` and `exact`
- **THEN** it matches `prod-eu` and `exact`
- **AND** it does not match `staging-eu` or `exactly`

### Requirement: Namespace guardrails cover all-namespace actions

When a context-scoped action runs across all namespaces (a context-target plugin in all-namespaces mode, or the cluster-wide `:pvc-clean`), the `namespaces` filter of a rule SHALL be treated as matching, so namespace rules cannot be bypassed by acting cluster-wide.

#### Scenario: Context plugin in all-namespaces mode

- **WHEN** a rule denies `plugin:*` in namespace `prod`
- **AND** a context-target plugin is run with all namespaces selected
- **THEN** the plugin is blocked by the guardrail

### Requirement: Guardrail deny

When any matching rule has `deny = true`, the action SHALL be blocked before any confirmation with the error flash `blocked by guardrail: <action> <plural>`, followed by ` — <reason>` when the last matching deny rule has a non-empty `reason`. Deny SHALL take precedence over `max_bulk` and `confirmation`.

#### Scenario: Delete denied with reason

- **WHEN** a rule with `actions = ["delete"]`, `deny = true`, `reason = "prod is locked"` exists
- **AND** the user requests delete on a pod
- **THEN** no confirmation opens
- **AND** the flash contains `blocked by guardrail` and `prod is locked`

### Requirement: Guardrail bulk limit

When matching rules set `max_bulk`, the smallest value SHALL apply, and an action targeting more objects than that value SHALL be blocked with `guardrail: <N> exceeds the max of <max> for <action> — mark fewer`. The bulk limit SHALL take precedence over `confirmation`.

#### Scenario: Bulk delete capped

- **WHEN** a rule sets `actions = ["delete"]` and `max_bulk = 1`
- **AND** the user requests delete with two pods marked
- **THEN** no confirmation opens and the flash contains `exceeds the max`

### Requirement: Guardrail confirmation strength

Matching rules SHALL raise the confirmation to the strongest requested level, ordered none, plain y/N, type-resource-name, type-context-name. `confirmation = "type-resource-name"` and `"type-context-name"` select typed prompts; any other value SHALL mean a plain y/N. A rule MUST NOT lower an action's built-in confirmation.

#### Scenario: Unknown value means plain confirm

- **WHEN** a rule sets `confirmation = "bogus"` for `shell`
- **AND** the user opens a shell in a pod
- **THEN** a y/N confirmation is shown before the shell starts

### Requirement: Typed guardrail confirmation

A typed confirmation SHALL open a prompt labelled `⚠ guardrail — type '<expected>' to confirm:` (drain uses `Type '<expected>' to confirm:`), where expected is the resource name for one target, the target count for several, or the context name for `type-context-name`. Trimmed input equal to expected SHALL run the action; other input SHALL cancel with `guardrail: input did not match — cancelled`; Esc SHALL cancel.

#### Scenario: Wrong then correct name

- **WHEN** a rule requires `type-resource-name` for delete
- **AND** the user requests delete on pod `a` and submits `wrong`
- **THEN** the flash contains `did not match` and nothing is deleted
- **AND** submitting `a` on a new request starts the delete

### Requirement: Guarded action names

Guardrails SHALL recognise these action names: `delete`, `force-delete` and `restart` on the current resource; `drain` and `node-debug` on `nodes`; `shell`, `debug` and `transfer` on `pods`; `pvc-explore` and `pvc-upload` on `persistentvolumeclaims`; and `plugin:<palette-or-name>` for plugins. PVC upload SHALL also be checked as `transfer` on `pods`, and PVC shell as `shell` on `pods`.

#### Scenario: Plugin denied by name

- **WHEN** a rule denies `plugin:sanitize`
- **AND** the user runs `:sanitize`
- **THEN** no confirmation opens and the flash contains `blocked by guardrail`

#### Scenario: Drain denied

- **WHEN** a rule denies `drain`
- **AND** the user presses `D` on a node
- **THEN** the drain dialog does not open

### Requirement: Built-in confirmation defaults

Delete, force-delete, restart, drain and node debug SHALL always ask for at least a y/N confirmation. Shell, debug and file transfer SHALL run without confirmation unless a guardrail raises it. A plugin SHALL confirm when it sets `confirm`, `dangerous` or `network_load`; for dangerous or network-load plugins the label SHALL start with `⚠` and mark the run `(dangerous)`.

#### Scenario: Network-load plugin confirms

- **WHEN** a plugin with `network_load = true` is run outside read-only mode
- **THEN** a confirmation opens and nothing runs until the user accepts

### Requirement: Confirmation dialog keys

In a y/N confirmation, `y`, `Y` and Enter SHALL accept; Esc, `n`, `N` and `q` SHALL cancel. Any other unbound key or a mouse wheel event SHALL cancel without passing through to the view beneath, unless the user sets `[keys.confirm] back = []`, in which case unbound keys SHALL be ignored.

#### Scenario: Wheel cancels without scrolling

- **WHEN** a delete confirmation is open over a table
- **AND** the user scrolls the mouse wheel
- **THEN** the confirmation is cancelled
- **AND** the table selection does not move

#### Scenario: Cancel-any disabled

- **WHEN** the config sets `[keys.confirm] back = []`
- **AND** the user presses an arrow key in a confirmation
- **THEN** the confirmation stays open

### Requirement: Delete confirmation options

In a delete confirmation, `f`/`F` SHALL toggle force-delete and `c`/`C` SHALL cycle the cascade policy, updating the label each time without closing the dialog.

#### Scenario: Toggle force

- **WHEN** a delete confirmation for pod `web` is open
- **AND** the user presses `f`
- **THEN** the label changes from `Delete pod web…` to `Force delete pod web…`

### Requirement: Ctrl-c cancels drain confirmations

In a drain y/N or typed confirmation, ctrl-c SHALL cancel the confirmation and return to the table without quitting sofka and without sending any request.

#### Scenario: Ctrl-c in drain prompt

- **WHEN** a drain typed confirmation is open with input typed
- **AND** the user presses ctrl-c
- **THEN** sofka returns to the table, does not quit, and sends no request

### Requirement: Guardrails follow configuration reloads

Guardrails SHALL be taken from the resolved configuration for the active context and SHALL be re-resolved on `:reload` and on every context switch.

#### Scenario: Per-context guardrail after switch

- **WHEN** a per-context config adds a deny rule
- **AND** the user switches to that context
- **THEN** the rule applies to actions in that context

### Requirement: Rules review with bare can-i

`:can-i` (aliases `:cani`, `:can`) with no arguments SHALL send a SelfSubjectRulesReview for the current namespace, or `default` when all namespaces are selected, showing `can-i: reviewing rules in <ns>…`. If the namespace selection is a pattern, it SHALL refuse with `select one namespace to review its permissions`.

#### Scenario: Review is sent

- **WHEN** the user runs `:can-i` in namespace `default`
- **THEN** a SelfSubjectRulesReview POST is sent for `default`
- **AND** a document view opens with the result

### Requirement: Rules review document

The rules document SHALL be titled `can-i · <context> · namespace <ns>`. It SHALL show `⚠ incomplete: the rule list below may be partial.` when the review is incomplete and `⚠ evaluation error: <e>` for a non-blank error, then sorted resource rules (verbs, `*` as `all`, group-qualified resources, names) under `Resource permissions in namespace <ns>`, non-resource URLs, and the hint `:can-i <verb> <resource> [namespace]`.

#### Scenario: Incomplete review with evaluation error

- **WHEN** the review returns incomplete with an evaluation error
- **THEN** the document shows `⚠ incomplete:` and `⚠ evaluation error: <e>`
- **AND** it lists `get,list` for `pods`

#### Scenario: Blank evaluation error hidden

- **WHEN** the review returns a blank evaluation error
- **THEN** no `evaluation error:` line is shown

### Requirement: Rules review failure

If the rules review request fails, the document SHALL be titled `can-i · namespace <ns>` and contain `access review failed: <e>`, and a `could not review permissions` warning SHALL be flashed.

#### Scenario: Review rejected

- **WHEN** the rules review request returns an error
- **THEN** the document shows `access review failed:`

### Requirement: Access check with verb and resource

`:can-i <verb> <resource> [namespace]` SHALL send a SelfSubjectAccessReview with the resource resolved through aliases to its plural and group (unknown resources sent lowercased as typed). The namespace argument `all`, `*` or `<all>`, or an empty current namespace, SHALL mean cluster-wide. Missing arguments SHALL flash `usage: :can-i <verb> <resource> [namespace]`; a pattern namespace with no argument SHALL be refused.

#### Scenario: Usage error

- **WHEN** the user runs `:can-i delete`
- **THEN** the flash shows `usage: :can-i <verb> <resource> [namespace]`

### Requirement: Access check verdict

An allowed review SHALL flash `✓ yes — can <verb> <resource> in <ns>` or `… cluster-wide`. A denied or not-allowed review SHALL show an error flash starting `✗ no — cannot`, with the reason when given; a denial SHALL override an allowed flag. A failed review SHALL flash `can-i <subject>: review failed: <e>`. A verdict MUST NOT be recorded as a watch error.

#### Scenario: Denied verdict is a sticky error

- **WHEN** the review answers that `delete pods` is not allowed
- **THEN** the flash shows `✗ no — cannot delete pods` as an error
- **AND** it is still shown after the normal flash expiry and the watch last error is unchanged

### Requirement: In-memory action journal

sofka SHALL record each cluster-affecting action (for example delete, edit, shell, scale, drain, plugin runs) when it starts, with UTC time, context, action name and target (`name`, `name in ns`, or `N <plural>`), keeping the latest 500 entries of the session. Each record SHALL also be logged at info level as event `action`. Plugin input values MUST NOT appear in the journal.

#### Scenario: Edit is recorded

- **WHEN** the user presses `e` on a pod
- **THEN** the journal holds one `edit` entry for that pod

#### Scenario: Secret plugin input stays out

- **WHEN** a plugin runs with `token=hunter2-s3cr3t`
- **THEN** the journal contains `plugin: <name>` and `1 target`
- **AND** it does not contain `hunter2-s3cr3t`

### Requirement: Journal view

`:journal` (aliases `:audit`, `:actions`) SHALL open a document titled `action journal (N entries)` with columns `WHEN  ACTION  CONTEXT  TARGET`, newest first, WHEN formatted `MM-DD HH:MM:SS` in UTC. With no entries it SHALL be titled `action journal (empty)` and show `(no actions recorded this session)`.

#### Scenario: Empty journal

- **WHEN** no action was taken this session
- **AND** the user runs `:journal`
- **THEN** the document shows `(no actions recorded this session)`

### Requirement: Persistent journal file

When `[journal] enabled = true`, each entry SHALL be appended as one JSON line `{at, context, action, target}` (UTC RFC 3339 `at`, all fields redacted) to `file`, default `<state-dir>/journal.jsonl`, creating it on the first entry and appending across sessions. When disabled (the default) no file SHALL be created. `[journal]` changes SHALL take effect only after restart.

#### Scenario: Appends across sessions

- **WHEN** journaling is enabled and two sessions each edit a pod
- **THEN** the file contains two valid JSON lines with action `edit` and an `at` ending in `Z`

#### Scenario: Disabled creates nothing

- **WHEN** journaling is disabled and the user edits a pod
- **THEN** no journal file or directory is created

### Requirement: Journal file rotation

Before an entry would grow the journal file past `max_size_mb` (default 8, minimum 64 KiB), the file SHALL be rotated to `<file>.1`, keeping only one backup. Every line SHALL remain complete JSON. Concurrent sessions SHALL coordinate through `<file>.lock`.

#### Scenario: Rotation keeps one backup

- **WHEN** an entry would exceed the size limit
- **THEN** the current file becomes `<file>.1` and the entry starts a new file

### Requirement: Journal write failures are visible

A journal entry that cannot be saved SHALL produce an error flash starting `journal entry not saved:` (oversized entry, full or closed writer queue, or a redacted I/O error) and SHALL be added to the `:info` warnings; the entry SHALL stay in the in-memory journal and sofka MUST keep running. If the writer cannot start, `journal writer could not start: …` SHALL be reported.

#### Scenario: Journal path is a directory

- **WHEN** the journal `file` points at a directory
- **AND** the user edits a pod
- **THEN** the flash contains `journal entry not saved` as an error
- **AND** the in-memory journal still has the entry

### Requirement: Journal shutdown

On exit sofka SHALL wait up to 300 ms for pending journal entries and, on timeout, print `warning: journal shutdown timed out; some entries may not be saved` to stderr.

#### Scenario: Slow disk at exit

- **WHEN** journal writes are still pending 300 ms after quitting
- **THEN** the timeout warning is printed to stderr

### Requirement: Credential redaction

Redaction SHALL replace with `«redacted»` the values of keys containing token, password, passwd, passphrase, secret, apikey, api-key, api_key, credential, private-key, privatekey, authorization, client-key-data, client-certificate-data or certificate-authority-data (case-insensitive; `key=value`, `key: value`, JSON and URL query forms), Bearer/Basic/Negotiate/Digest credentials, bare JWTs, and URL userinfo.

#### Scenario: Query string and JSON values

- **WHEN** text contains `?token_value=private123&limit=5` and `{"password":"abc\"private456","status":"denied"}`
- **THEN** both secrets are replaced with `«redacted»`
- **AND** `limit=5` and `"status":"denied"` are kept

### Requirement: Address redaction

Redaction SHALL replace literal IPv4 and IPv6 addresses with `«redacted»` while keeping ports and paths, for example `https://10.40.0.3:6443/` becomes `https://«redacted»:6443/`.

#### Scenario: IPv6 server URL

- **WHEN** text contains `https://[fd00::3]:6443/`
- **THEN** the address is replaced and `:6443/` is kept

### Requirement: Redaction keeps harmless text

Redaction MUST NOT alter version strings, timestamps, hostnames, invalid addresses such as `999.1.2.3`, hostnames containing IP-like parts, `kind=secrets`, prose mentioning secrets, or empty values such as `token=`.

#### Scenario: Prose and versions kept

- **WHEN** text is `listing secrets is forbidden` or `v1.2.3.4`
- **THEN** it is unchanged
