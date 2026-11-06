# plugins Specification

## Purpose

User-defined commands. This covers inline `[[plugins]]` shell-outs, plugin packages (`plugin.toml`) that receive a JSON request and return text or JSON reports, input forms, the plugin activity panel, and the bundled `:sanitize` plugin.

## Requirements

### Requirement: Inline plugin definition

A `[[plugins]]` entry SHALL accept:

- `name` and `command`, which are required;
- optional `key` (a chord), `palette` (a `:` command name), `args`, `scopes` (resource plurals) and `shell`;
- optional `mutating` (default true), `confirm`, `dangerous` and `network_load`;
- optional `output` (default `terminal`), `timeout` (default 30s, minimum 1s) and `prompt`;
- optional `requires`, `install`, `inputs`, `target` and `port_forward`.

Unknown fields in inline entries SHALL be ignored.

#### Scenario: Minimal inline plugin

- **WHEN** the config defines `[[plugins]]` with `key = "g"`, `name = "p"` and `command = "echo"`
- **THEN** pressing `g` on a selected row runs `echo` in the terminal

#### Scenario: Unknown inline field

- **WHEN** an inline plugin contains an unrecognised field
- **THEN** sofka loads the plugin without an error

### Requirement: Inline plugin configuration warnings

sofka SHALL report these non-fatal warnings and keep the plugin loaded:

- an invalid key: `plugin "N": invalid key — <error>`. This check is skipped when `key` is empty and `palette` is set.
- an unknown `output`, which falls back to terminal.
- an unknown `prompt`, which falls back to `missing`.
- an invalid `timeout`, which falls back to 30s.
- a `shell = true` command containing a `$` placeholder, which sofka does not expand there.

#### Scenario: Unknown output

- **WHEN** a plugin sets `output = "window"`
- **THEN** a warning says the output is unknown (expected terminal/popup/background/report) and that terminal is used

#### Scenario: Placeholder inside a shell script

- **WHEN** a `shell = true` plugin's command contains `$NAME`
- **THEN** a warning says sofka does not expand it there and suggests passing it in `args` and using `"$1"`

### Requirement: Hidden plugin key warning

When a plugin, bookmark or workspace chord overlaps a built-in table key whose action applies to the plugin's scopes, sofka SHALL warn `<kind> "N": <chord> is hidden by keys.table.<action> when that action is available`. The Faults toggle is exempt from this check. sofka SHALL flash the warning count at startup and point to `:config`.

#### Scenario: Shadowed key

- **WHEN** a pods-scoped plugin binds `shift-l`, which pods use for provider logs
- **THEN** a warning mentions `keys.table.provider_logs`
- **AND** the startup flash reports `1 warning(s)` and mentions `:config`

#### Scenario: Scope avoids the clash

- **WHEN** a plugin binds `shift-c` (cordon, which applies to nodes) and is scoped only to `pods`
- **THEN** no cordon warning is produced

### Requirement: Plugin key dispatch

When no built-in binding handles a table key, sofka SHALL check bookmark chords, then workspace chords, then plugin chords. It SHALL run the first plugin whose chord matches and whose `scopes` are empty or contain the current resource plural. A built-in key that does not apply to the current kind SHALL fall through to user bindings.

#### Scenario: Same key, different kinds

- **WHEN** one plugin bound to `x` is scoped to `pods` and another bound to `x` is scoped to `deployments`
- **THEN** pressing `x` on pods runs the pods plugin
- **AND** pressing `x` on deployments runs the deployments plugin

#### Scenario: Kind-specific built-in falls through

- **WHEN** a plugin binds `shift-c` for pods and nodes and the user presses Shift+C on pods
- **THEN** the plugin runs
- **AND** on nodes, Shift+C still cordons

### Requirement: Plugin palette commands

A plugin with `palette = "cmd"` SHALL run when the user enters `:cmd [name=value ...]`. Built-in palette commands and resource names SHALL take precedence over plugin palettes. If the plugin's scopes exclude the current kind, sofka SHALL flash `plugin does not apply to this resource kind`. Palette suggestions SHALL include in-scope plugin palettes.

#### Scenario: Built-ins win

- **WHEN** a plugin sets `palette = "q"` and the user enters `:q`
- **THEN** sofka quits and runs no plugin

#### Scenario: Out of scope

- **WHEN** a services-scoped plugin is invoked by palette on pods
- **THEN** the flash says the plugin does not apply

### Requirement: Placeholder substitution

Each element of `args` SHALL be substituted on its own and passed as exactly one argument, with no shell interpretation. The placeholders are `$NAMESPACE`, `$NS`, `$NAME`, `$CONTEXT`, `$CLUSTER`, `$RESOURCE`, `$GROUP`, `$VERSION`, `$KIND` and `$FILTER`. For inline plugins without `shell`, placeholders in `command` SHALL also be substituted.

#### Scenario: Placeholders become separate arguments

- **WHEN** a plugin runs `echo` with args `$RESOURCE $NAMESPACE $NAME` on pod `a` in `default`
- **THEN** the argv is `echo`, `pods`, `default`, `a`

### Requirement: Shell plugins pass values as positional arguments

With `shell = true`, sofka SHALL run `sh -c <command> sofka <args...>`. The command text SHALL NOT be substituted, so scripts read values from `"$1"` onwards.

#### Scenario: Filter with shell metacharacters

- **WHEN** the filter is `a;id` and a shell plugin with command `echo "$1" $FILTER` and args `$FILTER` runs
- **THEN** the argv is `sh`, `-c`, `echo "$1" $FILTER`, `sofka`, `a;id`

### Requirement: Missing executables block the run

Before running, sofka SHALL check that every `requires` entry is an executable (on PATH, or a path). If any is missing, it SHALL flash `missing executables: <list>` followed by ` — <install>` when `install` is set, and SHALL NOT run.

#### Scenario: Missing dependency

- **WHEN** a plugin requires a tool that is not installed and sets `install = "Install the example tool"`
- **THEN** nothing runs and the flash contains `Install the example tool`

### Requirement: Read-only mode gates plugins

In read-only mode, sofka SHALL refuse a plugin whose `mutating` is true (the default) or whose `network_load` is true. The refusal SHALL be flashed before any form or confirmation opens. A plugin with `mutating = false` and no network load SHALL run in read-only mode.

#### Scenario: Mutating plugin blocked

- **WHEN** read-only mode is on and a plugin without `mutating = false` is triggered
- **THEN** the flash mentions read-only and suggests `mutating = false`
- **AND** nothing runs

#### Scenario: Read-only plugin runs

- **WHEN** read-only mode is on and a plugin sets `mutating = false`
- **THEN** it runs

### Requirement: Plugin confirmation

A plugin with `confirm`, `dangerous` or `network_load` set SHALL ask for confirmation before running. The label SHALL read `Run plugin 'N' on K target(s)?`, followed by the context, the namespace (`all` for all namespaces), any port-forward, the command line (truncated to 120 characters with `…`) and the input values. `dangerous` and `network_load` SHALL prefix the label with `⚠ ` and append ` (dangerous)`.

#### Scenario: Dangerous plugin

- **WHEN** a `dangerous = true` plugin is triggered
- **THEN** a confirmation containing `⚠` appears and nothing runs
- **AND** answering `y` runs the plugin

#### Scenario: Declined

- **WHEN** the user answers `n` at the confirmation
- **THEN** the plugin does not run

### Requirement: Plugin guardrails

Plugin runs SHALL be checked against guardrails using the action `plugin:<palette or name>`, before any confirmation (see the safety capability). A context-target plugin run in all-namespaces mode SHALL be checked as an all-namespaces action.

#### Scenario: Denied plugin

- **WHEN** a guardrail denies `plugin:*`
- **THEN** triggering a plugin flashes `blocked by guardrail` and nothing runs

### Requirement: Selection and marked targets

A plugin with the default `target = "selection"` SHALL run once per marked row, or on the selected row when nothing is marked. With no row, sofka SHALL flash `no selection for plugin`. A terminal-output plugin with several targets SHALL be refused with `'N': a marked set needs output = popup or background`.

#### Scenario: Terminal plugin on a marked set

- **WHEN** two pods are marked and a terminal-output plugin is triggered
- **THEN** the flash error mentions `marked set` and nothing runs

#### Scenario: Background plugin on a marked set

- **WHEN** two pods are marked and a background plugin is triggered
- **THEN** the plugin runs once for each marked pod

### Requirement: Context target

A plugin with `target = "context"` SHALL run once without a selected object, using the current namespace (empty for all namespaces). If the current namespace is a multi-namespace pattern, sofka SHALL refuse with `select one namespace or all namespaces for a context plugin`.

#### Scenario: Context plugin without rows

- **WHEN** a context-target popup plugin runs on an empty table
- **THEN** it receives a request with a null `object` and the current namespace

### Requirement: Plugin input arguments

A plugin with `inputs` SHALL accept `name=value` palette arguments separated by whitespace. A value replaces a whole `${input.name}` argument literally and is never expanded again. Missing values SHALL use the defaults. sofka SHALL refuse with:

- `use name=value plugin arguments`
- `unknown input "x"`
- `duplicate input "x"`
- `missing input name=…`
- `<name>: <validation error>`

#### Scenario: Literal value

- **WHEN** the user enters `:example-plugin count=4 value=$(false)`
- **THEN** the argv contains `4` and `$(false)` unchanged

#### Scenario: Out of range

- **WHEN** an integer input with max 100 receives `count=101`
- **THEN** the flash contains `outside range` and nothing runs

### Requirement: Input types

Each input SHALL declare a `type` of `string`, `integer` (unsigned), `boolean` (`true`/`false`) or `duration`. Every input SHALL accept optional `default` and `choices`, and integer and duration inputs SHALL accept `min` and `max`. A violation SHALL produce `expected unsigned integer`, `value outside range …`, `expected one of <choices>` or `invalid <type> value "v"`.

#### Scenario: Choice violation

- **WHEN** a string input with choices `all, misconfig` receives `scan=other`
- **THEN** the error says `expected one of all, misconfig`

### Requirement: When the input form opens

When a plugin is triggered without arguments, sofka SHALL open the input form if the plugin has inputs and either `prompt = "always"` or some input has no default. Otherwise it SHALL run with the defaults. Palette arguments SHALL never open the form. Read-only and selection checks SHALL run before the form opens.

#### Scenario: Required input

- **WHEN** a key chord triggers a plugin whose `port` input has no default
- **THEN** the input form opens and nothing runs yet

#### Scenario: Defaults only

- **WHEN** every input has a default and `prompt` is not `always`
- **THEN** the plugin runs at once with the defaults

### Requirement: Input form editing

The form SHALL list the inputs sorted by name, prefilled with their defaults, and show each field's type, range and whether it is required.

- Up/Down and Tab/Shift-Tab SHALL move focus, wrapping around.
- Left/Right SHALL cycle choices and booleans; typing on those fields SHALL be ignored.
- On other fields, typing SHALL append non-whitespace characters up to 256.
- Backspace SHALL delete the last character.
- Esc SHALL cancel without running.

#### Scenario: Typing and cycling

- **WHEN** the user types `80 80` into the port field, presses Tab and Right
- **THEN** port holds `8080` and the choice field shows the next choice

#### Scenario: Cancel

- **WHEN** the user presses Esc in the form
- **THEN** the form closes and nothing runs

### Requirement: Input form submission

On Enter, the form SHALL validate every field. An empty field without a default SHALL show `required`. Errors SHALL appear on their own row under the field, focus SHALL move to the first invalid field and the values SHALL be kept. Editing a field SHALL clear its error. A valid form SHALL continue to confirmation (when required) and then run.

#### Scenario: Required error

- **WHEN** the user presses Enter with the required port field empty
- **THEN** the form stays open with `required` on port and focus on it

#### Scenario: Confirmation after the form

- **WHEN** a dangerous plugin's form is submitted with port 8080
- **THEN** a confirmation that includes `port=8080` opens before the run

### Requirement: Output modes

`output` SHALL select how results are shown:

- `terminal`: suspend the TUI, run attached to the terminal and flash `plugin: N`.
- `popup`: capture the output and show it as text.
- `report`: capture stdout as a JSON report.
- `background`: run without a document and summarise in the status line.

The captured modes SHALL show the status `plugin: N[ ×count]…`.

#### Scenario: Popup output

- **WHEN** a popup plugin finishes with output
- **THEN** a document titled `N — output` shows the output

#### Scenario: Background bulk summary

- **WHEN** a background plugin finishes over 3 targets with 1 failure
- **THEN** the status reads `plugin N: 2 ok, 1 failed — <reason>`

### Requirement: Captured execution limits

Captured runs SHALL use at most 8 concurrent processes and report results in target order. Each job SHALL be bounded by `timeout` and fail with `timed out after Ns`. stdout and stderr SHALL each be capped at 1 MiB (`plugin output exceeds 1 MiB`). Displayed output SHALL be capped at 5000 lines or 1 MiB, followed by `… output truncated`.

#### Scenario: Timeout

- **WHEN** a plugin with `timeout = "1s"` sleeps for 20 seconds
- **THEN** the job fails with `timed out after 1s`

### Requirement: Captured outcome text

A successful job with empty stdout SHALL show `(no output)`. A failed job SHALL show its output followed by `[stderr] <first non-empty stderr line>`. The failure reason SHALL be `exited code N`, `by signal` or `failed to run: <error>`. With several targets, each result SHALL be preceded by `== <namespace/name> ==`.

#### Scenario: Nonzero exit

- **WHEN** a popup plugin exits with code 2 and writes `boom` to stderr
- **THEN** the result reports the failure with `[stderr] boom`

### Requirement: Completion status

When a captured run completes, sofka SHALL open its result document and set the status to `plugin done`, or to `F of T failed` with the diagnostics shown first. If the activity panel was hidden when the run finished, the result SHALL be kept and the status SHALL end with ` — :plugin-activity to open`, without replacing the current view.

#### Scenario: Hidden completion

- **WHEN** the user hides the panel and the plugin finishes
- **THEN** the current view and palette input stay as they were
- **AND** the status mentions `:plugin-activity`

### Requirement: Plugin activity panel

Captured popup and report runs SHALL open a centred activity panel, at most 100 columns by 18 rows. Its title SHALL show a spinner, the plugin title and the elapsed seconds, and its footer SHALL show the key hints. It SHALL stream sanitised stderr diagnostics and show `Waiting for plugin diagnostics…` until some arrive. Diagnostics from several targets SHALL be prefixed with `[ns/name]`.

#### Scenario: Streaming diagnostics

- **WHEN** a popup plugin writes progress lines to stderr
- **THEN** the panel shows them while the plugin runs

### Requirement: Activity panel keys

Ctrl+Alt+T (`[keys.global].plugin_activity`) SHALL toggle the panel, or flash `no plugin activity` if nothing has run. While the panel is shown:

- Esc SHALL hide it without cancelling.
- Ctrl+C SHALL cancel a running task.
- Enter SHALL open a finished result.
- Up/Down/j/k, PgUp/PgDn, Home/End/g/G and h/l SHALL scroll; G SHALL resume following.
- `:` SHALL hide it and open the palette.

#### Scenario: Hide and reopen

- **WHEN** the user presses Esc while a plugin runs and later presses Ctrl+Alt+T
- **THEN** the plugin keeps running and the panel reappears

#### Scenario: Cancel from the panel

- **WHEN** the user presses Ctrl+C in the panel while a plugin runs
- **THEN** the process group is killed and the status reads `plugin cancelled`

### Requirement: Activity diagnostics are sanitised and bounded

The panel SHALL strip ANSI and OSC escapes, control characters and bidirectional controls. It SHALL replace invalid UTF-8 with `�` and expand tabs. A bare carriage return SHALL redraw the current line. The tail SHALL keep at most 256 lines, 64 KiB and 2048 bytes per line, and SHALL show `older tail discarded` once it drops lines.

#### Scenario: Progress bar redraws

- **WHEN** a plugin redraws a progress line with `\r` many times
- **THEN** the panel shows one line with the latest progress

### Requirement: Plugin activity commands

`:plugin-activity` SHALL open the retained result, or show the panel of a running plugin. `:plugin-cancel` SHALL stop the running plugin and flash `plugin cancelled`. Palette suggestions SHALL offer `plugin-cancel` only for a non-empty query.

#### Scenario: Reopen a retained report

- **WHEN** a hidden run has finished and the user enters `:plugin-activity`
- **THEN** its result document opens

### Requirement: Plugin cancellation and replacement

Starting a new plugin SHALL cancel the previous one. Switching resource, context or mode (other than overlays), quitting, and other built-in palette actions SHALL cancel a running plugin. Results from a cancelled or replaced run SHALL be discarded.

#### Scenario: Context switch cancels

- **WHEN** a plugin runs and the user switches context
- **THEN** the plugin process is killed and its late result is ignored

### Requirement: Plugin journal entries

Each plugin launch SHALL record `plugin: <name>` with the target count (`1 target` or `N targets`) in the action journal. It SHALL NOT record input values.

#### Scenario: Secret input

- **WHEN** a plugin runs with `token=hunter2`
- **THEN** the subprocess receives `hunter2`
- **AND** the journal contains `plugin: <name>` and `1 target` but not `hunter2`

### Requirement: Inline plugin merging across files

Each config file SHALL accept `plugins_remove = [names]` and `plugins_merge = "name"` or `"replace"` (the default). Removal SHALL happen first. With `"name"`, an entry SHALL replace the plugin of the same name in place or be appended. With `"replace"`, the file's list SHALL replace the inherited list. Package and bundled plugins SHALL be unaffected.

#### Scenario: Merge by name

- **WHEN** a later file sets `plugins_merge = "name"` and redefines plugin `p`
- **THEN** after `:reload` the palette runs the new `p` and the other inherited plugins remain

### Requirement: Package location and loading

sofka SHALL load plugin packages from `<config dir>/plugins/<dir>/plugin.toml` in sorted directory order. An invalid package SHALL be skipped with the warning `ignoring <path>: <error>` while the other packages still load. A package command that conflicts with an already loaded plugin (same name, palette, or overlapping key and scopes) SHALL be skipped with `<path>: duplicate plugin name/command; earlier configuration wins`. Inline plugins SHALL take precedence over packages.

#### Scenario: One invalid package

- **WHEN** one package directory has an invalid manifest and another is valid
- **THEN** the valid package's commands are available after `:reload`
- **AND** a warning names the ignored path

### Requirement: Package manifest schemas

`plugin.toml` SHALL use `schema_version = 1` with a `[plugin]` table, or `schema_version = 2` with a non-empty `[[commands]]` array. Mixing the formats SHALL be rejected. Any other schema version SHALL be rejected with `unsupported schema_version (expected 1 or 2)`. Manifests larger than 1 MiB SHALL be rejected.

#### Scenario: Mixed format

- **WHEN** a schema 2 manifest also contains `[plugin]`
- **THEN** the package is ignored with a message saying not to mix the formats

### Requirement: Package command validation

Package commands SHALL reject:

- unknown fields, an empty `name` or `command`, and `shell`;
- `output = "terminal"`, since packages require captured output;
- a palette that is not lowercase letters, digits and hyphens, or that is reserved by sofka;
- `prompt` other than `missing`/`always`, and `target` other than `selection`/`context`;
- duplicate names, palettes, or keys with overlapping scopes within the package.

#### Scenario: Reserved palette

- **WHEN** a package command sets `palette = "pods"` or `"plugin-cancel"`
- **THEN** the package is rejected because the palette command is reserved

### Requirement: Package input validation

Package inputs SHALL use names made of lowercase letters, digits and `_`, and a supported type. `min`/`max` SHALL only appear on integer or duration inputs, and `min` SHALL NOT exceed `max`. Defaults SHALL pass validation. Every `${input.x}` SHALL be a whole argument that names a declared input.

#### Scenario: Embedded input reference

- **WHEN** an argument is `--port=${input.port}`
- **THEN** the package is rejected

### Requirement: Package publication metadata

An optional `[package]` table SHALL require:

- a semver `version`, a non-empty `description` and `license`, and no empty `authors` entries;
- a valid `sofka` version requirement;
- an `https` `repository`;
- distinct non-empty `platforms` and non-empty `tags`;
- requirements with distinct names and install text.

`display_name` SHALL require a `sofka` requirement that excludes versions below 0.27.1.

#### Scenario: Non-HTTPS repository

- **WHEN** `[package].repository` is `http://example.com/x`
- **THEN** the package is rejected

### Requirement: Package command path

A package command starting with `./` SHALL resolve to an existing file inside the package directory; one that leaves it SHALL be refused with `relative command escapes package directory`. Each package command SHALL be added to its own `requires`; a missing requirement SHALL warn `plugin N: missing executables: …` at load but keep the command, and running it SHALL be refused.

#### Scenario: Escaping path

- **WHEN** a package command is `./../other/bin`
- **THEN** the package is rejected

### Requirement: Package JSON request

Package and bundled plugins SHALL receive, on stdin, a JSON request (at most 1 MiB) with these fields: `schema_version: 1`, `context`, `cluster`, `namespace`, `resource`, `name`, `filter` and `inputs`. It SHALL also include `forward` (null, or `host`, `local_port` and `remote_port`) and `object` (the selected object, or null for a context target). The process SHALL run in the package directory.

#### Scenario: Request contents

- **WHEN** a context-target package runs
- **THEN** its stdin is JSON with `schema_version` 1, a null `object` and the current namespace

### Requirement: Managed port-forward for packages

`port_forward` SHALL be accepted only on selection-target report packages, as a port number or an `${input.x}` that resolves to one. sofka SHALL open a temporary forward to the selected pod or service and pass it in `forward`. Ports outside 1–65535 SHALL be refused with `port_forward must resolve to a port from 1 to 65535`, and other kinds with `plugin port-forward requires a pod or service`.

#### Scenario: Invalid port

- **WHEN** the input resolves to port 70000
- **THEN** the run is refused before launch

### Requirement: Report format

`output = "report"` stdout SHALL be JSON with `schema_version: 1`, `title`, and `sections` (each with `title`, `lines`, `columns` and `rows`), plus an optional `action`. Unknown fields SHALL be rejected. sofka SHALL render the title, then each section's title and lines, and its table with `│` separators and a `─┼─` rule. Control characters SHALL be shown as spaces. The document SHALL be searchable with `/`.

#### Scenario: Table rendering

- **WHEN** a report has columns `Resource, Severity` and two rows
- **THEN** the document shows aligned rows separated by `│` under a `────┼────` rule

### Requirement: Invalid reports fail visibly

A report SHALL fail when its schema version is not 1, a row length does not match its columns, its JSON is invalid, or it exceeds 1 MiB. The error SHALL be shown in the result document and as an error status.

#### Scenario: Future schema

- **WHEN** a report has `schema_version: 8`
- **THEN** the result shows `unsupported report schema_version (expected 1)` as an error

### Requirement: Kubeconfig reload report action

A successful report with `action = "reload-kubeconfig"` SHALL open the context selector after the run, and SHALL reconnect only when the user picks a context. Failed, cancelled, stale, invalid or plain-output runs SHALL NOT trigger the action.

#### Scenario: Reload after login

- **WHEN** a login plugin returns a valid report with `action = "reload-kubeconfig"`
- **THEN** the context selector opens with the refreshed contexts

### Requirement: Bundled plugins

sofka SHALL ship built-in packages that need nothing on PATH. They SHALL load after inline and user packages and be skipped when their name or palette is taken. When the session uses `--no-tls-resumption` or `--allow-v1-client-cert`, the same flag SHALL be passed to bundled plugins.

#### Scenario: User override

- **WHEN** a user package defines palette `sanitize`
- **THEN** `:sanitize` runs the user package instead of the bundled one

### Requirement: Sanitize command definition

`:sanitize` (named "Sanitize pods") SHALL be scoped to pods, run against the context, produce a report, and be mutating and dangerous with a 2 minute timeout. Its inputs SHALL be `states` (`terminal`, `stuck` or `all`; default `terminal`) and `dry_run` (default `false`).

#### Scenario: Confirmation first

- **WHEN** the user enters `:sanitize` on pods
- **THEN** a `⚠` confirmation opens and nothing is deleted before `y`

#### Scenario: Read-only

- **WHEN** read-only mode is on
- **THEN** `:sanitize` is refused without confirmation

### Requirement: Sanitize state matching

`terminal` SHALL match pods in Succeeded, Failed, Error, OOMKilled or ContainerStatusUnknown. `stuck` SHALL add CrashLoopBackOff, ImagePullBackOff and ErrImagePull, and `all` SHALL add Pending. Pods already being deleted, or with any running app container, SHALL be excluded. Other `states` values SHALL fail with `unknown states value 'x'`.

#### Scenario: Running container protects a pod

- **WHEN** a pod is Failed but one app container is still running
- **THEN** sanitize does not delete it

### Requirement: Sanitize listing scope

Sanitize SHALL list pods in the current namespace or across all namespaces, in pages of 500. It SHALL pass the view filter's `-l` and `-f` terms as label and field selectors. Any other filter term SHALL fail with an error saying the view filter narrows the table in ways the command cannot reproduce. A listing failure SHALL fail the run.

#### Scenario: Label filter

- **WHEN** the pod view filter is `-l app=web`
- **THEN** only pods matching `app=web` are considered

### Requirement: Sanitize deletion safety

Before deleting, sanitize SHALL re-read each pod and delete it with UID and resourceVersion preconditions. A pod that is gone (404) SHALL count as already gone. A pod that changed (UID, state or 409) SHALL count as changed and be left alone. Other errors SHALL be listed as failed. `dry_run=true` SHALL delete nothing.

#### Scenario: Pod changed since the scan

- **WHEN** a matched pod restarts before deletion
- **THEN** it is left alone and counted in `changed since the scan and left alone`

### Requirement: Sanitize bulk limit

Sanitize SHALL enforce the guardrail `max_bulk` for `plugin:sanitize` itself, using context `default` when there is none. When more pods match than allowed, it SHALL report `N matched, which exceeds the guardrail limit of M for plugin:sanitize. Nothing deleted.`

#### Scenario: Over the limit

- **WHEN** 3 pods match and `max_bulk = 2` applies to `plugin:sanitize`
- **THEN** nothing is deleted and the report states the limit

### Requirement: Sanitize report

The report titled "Sanitize pods" SHALL summarise:

- the scope (`Scanned namespace X.` or `Scanned all namespaces.`) and the matching statuses;
- `N matched; nothing deleted (dry run).` or `N matched, D deleted, F failed.`, plus the changed and already-gone counts.

Its tables SHALL be Failed (Namespace, Pod, Error) and Deleted, Would delete or Blocked (Namespace, Pod, Status), each capped at 500 rows plus `K more rows not shown.`.

#### Scenario: Dry run

- **WHEN** the user runs `:sanitize dry_run=true` with 2 matches
- **THEN** the report says `2 matched; nothing deleted (dry run).` and lists them under Would delete

### Requirement: Help lists plugins

The `?` help SHALL list each plugin under Plugins as `<chord> / :<palette>  <name> (<scopes or "all resources">)`.

#### Scenario: Help entry

- **WHEN** a pods-scoped plugin binds `ctrl-g` with palette `scan` and the user presses `?`
- **THEN** help shows its chord, `:scan`, its name and `(pods)`
