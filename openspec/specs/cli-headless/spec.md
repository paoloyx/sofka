# cli-headless Specification

## Purpose

Describe how sofka is launched from the command line: arguments, subcommands, startup order, the headless `--check` and `--snapshot` modes, shell completion, plugin validation entry points, the context picker at launch and exit codes.

## Requirements

### Requirement: Launch resource argument

sofka SHALL accept an optional positional resource argument that names an alias, plural or kind, and SHALL open that resource view at launch. The values `ctx` and `contexts` MUST open the context picker instead.

#### Scenario: Alias opens its resource

- **WHEN** the user runs `sofka dp`
- **THEN** sofka starts on the Deployments view

#### Scenario: Context picker keyword

- **WHEN** the user runs `sofka ctx`
- **THEN** sofka opens the context picker instead of a resource view

### Requirement: Explicit resource option

`--resource NAME` SHALL open the named resource and MUST accept names that collide with subcommands, such as `plugin` or `completion`. Passing `--resource` together with the positional resource MUST be rejected as a usage error.

#### Scenario: Resource named like a subcommand

- **WHEN** the user runs `sofka --resource plugin`
- **THEN** sofka opens the `plugin` resource instead of running the `plugin` subcommand

#### Scenario: Both forms given

- **WHEN** the user runs `sofka pods --resource svc`
- **THEN** sofka exits with a usage error

### Requirement: Default launch resource

When no resource is given on the command line, sofka SHALL open the config `default_resource`, and when that is also unset it MUST open `pods`.

#### Scenario: Config default applies

- **WHEN** the config sets `default_resource = "deployments"`
- **AND** the user runs `sofka` with no resource
- **THEN** sofka starts on the Deployments view

#### Scenario: Built-in default

- **WHEN** neither the command line nor the config names a resource
- **THEN** sofka starts on the Pods view

### Requirement: Launch namespace options

`-n/--namespace NAME` SHALL start sofka in that namespace. `-A/--all-namespaces` SHALL start across all namespaces and MUST take precedence over `-n` when both are given.

#### Scenario: All namespaces wins

- **WHEN** the user runs `sofka pods -n kube-system -A`
- **THEN** sofka starts on Pods across all namespaces

### Requirement: Read-only and write flags

`--readonly` SHALL disable every mutating action for the whole session and `--write` SHALL force write mode for the whole session. Either flag MUST win over the config `readonly` option, including per-cluster and per-context overrides, on every context switch. Passing both flags MUST be rejected as a usage error.

#### Scenario: Flag beats per-context config

- **WHEN** the user runs `sofka --write`
- **AND** switches to a context whose config sets `readonly = true`
- **THEN** write mode stays enabled

#### Scenario: Conflicting flags

- **WHEN** the user runs `sofka --readonly --write`
- **THEN** sofka exits with a usage error

### Requirement: Usage errors

sofka MUST reject unknown options, missing option values and conflicting options before loading config or connecting, printing the parser error to stderr and exiting with status 2.

#### Scenario: Unknown option

- **WHEN** the user runs `sofka --no-such-flag`
- **THEN** sofka prints a usage error to stderr
- **AND** exits with status 2

### Requirement: Version and help

`sofka --version` SHALL print the program version and `sofka --help` SHALL list the public options and subcommands. The internal `--plugin-adapter` option MUST NOT appear in the help output.

#### Scenario: Help hides the adapter

- **WHEN** the user runs `sofka --help`
- **THEN** the output lists `--check`, `--snapshot` and `--context`
- **AND** does not list `--plugin-adapter`

### Requirement: Startup order

sofka SHALL handle work at startup in this order: shell completion requests, argument parsing, the `completion` subcommand, `--kubeconfig`, the plugin adapter, plugin validation, the `plugin` subcommand, config loading, `info`, and only then the cluster connection. Steps before config loading MUST NOT load config or connect to a cluster.

#### Scenario: Validation does not connect

- **WHEN** the user runs `sofka --validate-plugin ./my-plugin` with an unreachable cluster
- **THEN** validation completes without any cluster request

### Requirement: Connecting message

Before connecting to a cluster at startup, sofka SHALL print `Connecting to cluster…` to stderr. It MUST NOT print this message when the launch opens the context picker.

#### Scenario: Message on stderr

- **WHEN** the user runs `sofka --check`
- **THEN** stderr starts with `Connecting to cluster…`

### Requirement: Check mode

`--check` SHALL connect to the cluster, run API discovery, print a connection summary to stdout and exit with status 0 without starting the terminal UI or requiring a TTY.

#### Scenario: Successful check

- **WHEN** the user runs `sofka --check` against a reachable cluster
- **THEN** stdout starts with `✓ connected`
- **AND** sofka exits with status 0

### Requirement: Check summary identity lines

The `--check` summary SHALL print `context:`, `cluster:`, `server:`, `k8s rev:` and `namespace:` lines after `✓ connected`. `k8s rev:` MUST show `unknown` when the server version could not be read.

#### Scenario: Unknown server version

- **WHEN** the `/version` request fails during `sofka --check`
- **THEN** the summary prints `  k8s rev:    unknown`

### Requirement: Check summary discovery lines

The `--check` summary SHALL print `kinds:      N resource types discovered`, where N is the number of entries in the discovered resource catalog. When some API groups could not be read it MUST also print `not read:   N API group(s). Refer to the warnings above.`, using `group` when N is 1 and `groups` otherwise.

#### Scenario: One unreadable group

- **WHEN** discovery skips one API group during `sofka --check`
- **THEN** the summary prints `  not read:   1 API group. Refer to the warnings above.`

### Requirement: Check summary alias resolution

The `--check` summary SHALL print one `resolve` line for each of `pods`, `po`, `dp`, `svc`, `no`, `ns` and `cm`, showing the resolved resource title and `namespaced=true|false`, or `<unresolved>` when the name does not resolve.

#### Scenario: Alias resolves

- **WHEN** the cluster serves Deployments
- **THEN** the summary contains a line ` resolve dp    →` followed by the Deployments title and `(namespaced=true)`

### Requirement: Check summary namespace count

The `--check` summary SHALL end with `namespaces: N`, the number of namespaces listed, or `namespaces: error: <error>` when listing fails. A failed namespace list MUST NOT change the exit status.

#### Scenario: Namespaces forbidden

- **WHEN** the user cannot list namespaces
- **THEN** the summary prints ` namespaces: error:` followed by the error
- **AND** sofka still exits with status 0

### Requirement: Snapshot mode

`--snapshot` SHALL connect, start the launch view, collect watch data for about 3 seconds, render one 120x32 frame to stdout with trailing spaces removed from each line, and exit without requiring a TTY.

#### Scenario: Headless frame

- **WHEN** the user runs `sofka dp -A --snapshot`
- **THEN** stdout contains one rendered frame of the Deployments view across all namespaces
- **AND** sofka exits

### Requirement: Headless connect failure

When the connection fails under `--check` or `--snapshot`, sofka MUST print `error: <cause>` to stderr and exit with status 1.

#### Scenario: Unreachable cluster in CI

- **WHEN** the user runs `sofka --check` and the API server is unreachable
- **THEN** stderr contains `error:` and the cause
- **AND** sofka exits with status 1

### Requirement: Headless missing current context

When the kubeconfig has no current context and no `--context` is given, `--check` and `--snapshot` MUST exit with status 1 and print `no current-context in <path>; use --context <name> or run sofka ctx to select a context`, where `<path>` is `KUBECONFIG` or `~/.kube/config`. The message MUST NOT mention in-cluster configuration.

#### Scenario: Empty current context

- **WHEN** the kubeconfig has no `current-context`
- **AND** the user runs `sofka --check`
- **THEN** stderr names the kubeconfig path and suggests `--context <name>` or `sofka ctx`
- **AND** sofka exits with status 1

### Requirement: Interactive connect failure

When the connection fails in interactive mode for a reason other than a missing current context, sofka SHALL print `warning: <cause>` to stderr, start the terminal UI disconnected in the context picker and flash `cannot connect to '<context>': <cause> — pick another context`.

#### Scenario: Failed launch connection

- **WHEN** the user runs `sofka --context west` and connecting to `west` fails
- **THEN** the context picker opens
- **AND** the flash names `west` and the cause

### Requirement: Interactive missing current context

When the kubeconfig has no current context and no `--context` is given, interactive sofka SHALL open the context picker instead of failing.

#### Scenario: Pick a context on first run

- **WHEN** the kubeconfig has no `current-context`
- **AND** the user runs `sofka`
- **THEN** the context picker opens

### Requirement: Context picker at launch

`sofka ctx` and `sofka contexts` SHALL open the context picker before connecting to any cluster. Selecting a context MUST connect to it and open its configured default resource, or Pods. `-n` and `-A` MUST apply to the first successful selection only.

#### Scenario: Launch picker connects on Enter

- **WHEN** the user runs `sofka ctx -n payments`
- **AND** selects context `prod` and presses Enter
- **THEN** sofka connects to `prod` in namespace `payments`

### Requirement: Launch picker initial context

With `sofka ctx --context NAME`, sofka SHALL place the picker cursor on `NAME`. When `NAME` is not in the kubeconfig, sofka MUST exit with the error `context 'NAME' not found in kubeconfig` before starting the terminal UI.

#### Scenario: Unknown context

- **WHEN** the user runs `sofka ctx --context missing`
- **THEN** sofka exits with an error containing `context 'missing' not found in kubeconfig`

### Requirement: Launch picker requires interactive mode

`ctx` and `contexts` MUST be rejected with `ctx and contexts require interactive mode; remove --check or --snapshot` when combined with `--check` or `--snapshot`.

#### Scenario: Picker with check

- **WHEN** the user runs `sofka ctx --check`
- **THEN** sofka exits with that error and does not connect

### Requirement: Startup warnings

sofka SHALL print API discovery fallback notes, unreadable API group warnings and config warnings to stderr as `warning: ...` before the terminal UI starts. In the terminal UI, unreadable groups MUST also produce the flash `API discovery could not read N API group(s). Refer to :info for details.`

#### Scenario: Unreadable group at launch

- **WHEN** discovery skips two API groups
- **THEN** stderr shows one warning per group
- **AND** the first screen flashes `API discovery could not read 2 API groups. Refer to :info for details.`

### Requirement: Terminal restore on exit

When the terminal UI exits, sofka SHALL disable mouse capture, clear the terminal title it set, and show the cursor again. Headless modes MUST NOT change the terminal title.

#### Scenario: Normal quit

- **WHEN** the user quits the terminal UI
- **THEN** the shell gets a visible cursor and no mouse capture

### Requirement: Info subcommand

`sofka info` SHALL connect to the cluster briefly, print the diagnostics report and exit. `sofka info --offline` MUST print the report without connecting. When the connection fails, sofka MUST print a warning and still print the report. `--info` SHALL remain a deprecated alias for `sofka info --offline`.

#### Scenario: Offline report

- **WHEN** the user runs `sofka info --offline`
- **THEN** sofka prints the diagnostics report without any cluster request

#### Scenario: Unreachable cluster

- **WHEN** the user runs `sofka info` and the connection fails
- **THEN** stderr shows a warning
- **AND** stdout still contains the diagnostics report

### Requirement: Plugin package validation

`--validate-plugin DIR` SHALL validate a plugin package without executing it or connecting to a cluster, print `valid plugin: <name>` for each command and exit with an error when the package is invalid. It MUST be rejected together with `--check`, `--snapshot`, `--info` or `--validate-plugin-report`.

#### Scenario: Valid package

- **WHEN** the user runs `sofka --validate-plugin ./my-plugin` on a valid package with one command `hello`
- **THEN** stdout contains `valid plugin: hello`

### Requirement: Plugin report validation

`--validate-plugin-report FILE` SHALL validate a versioned plugin JSON report without a cluster and print its rendered lines. Oversized files MUST be rejected. It MUST be rejected together with `--check`, `--snapshot` or `--info`.

#### Scenario: Render a report

- **WHEN** the user runs `sofka --validate-plugin-report report.json` with a valid report
- **THEN** sofka prints the rendered report lines

### Requirement: Internal plugin adapter

The hidden `--plugin-adapter NAME` option SHALL run a core plugin adapter that reads a request on stdin and writes a report on stdout, before loading config or connecting. Only `sanitize` MUST be accepted; any other name MUST fail with `unknown core plugin adapter '<name>'`.

#### Scenario: Unknown adapter

- **WHEN** sofka runs with `--plugin-adapter other`
- **THEN** it fails with `unknown core plugin adapter 'other'`

### Requirement: Completion script generation

`sofka completion <shell>` SHALL print a completion script for `bash`, `zsh`, `fish`, `elvish` or `powershell` that registers dynamic completion through `SOFKA_COMPLETE`. It MUST NOT load config or connect, and MUST succeed with empty stderr even when the config file or kubeconfig is invalid.

#### Scenario: Invalid config

- **WHEN** the config file is invalid
- **AND** the user runs `sofka completion zsh`
- **THEN** stdout contains a zsh script mentioning `sofka` and `SOFKA_COMPLETE`
- **AND** stderr is empty

### Requirement: Completion shell argument errors

`sofka completion` without a shell, or with an unsupported shell, MUST exit with status 2, print nothing to stdout and report the shell problem on stderr.

#### Scenario: Unsupported shell

- **WHEN** the user runs `sofka completion tcsh`
- **THEN** sofka exits with status 2
- **AND** stdout is empty

### Requirement: Bounded dynamic completion

Dynamic completion SHALL return within about 2 seconds in total. Cluster queries MUST have their own shorter limit. On timeout or failure sofka MUST print no candidates and no stderr output, and MUST stop every process it started for the request.

#### Scenario: Slow cluster

- **WHEN** the user presses Tab after `sofka -n ` and the API server does not answer
- **THEN** completion returns no candidates within the time limit
- **AND** the shell shows no error output

### Requirement: Context completion

Completion for `--context` SHALL offer kubeconfig context names, honouring a merged `KUBECONFIG` list and an earlier `--kubeconfig PATH` on the same command line.

#### Scenario: Earlier kubeconfig flag

- **WHEN** the user types `sofka --kubeconfig ./other.yaml --context ` and presses Tab
- **THEN** the candidates are the contexts in `./other.yaml`

### Requirement: Namespace completion

Completion for `-n/--namespace` SHALL offer namespace names from the cluster of an earlier `--context`, or of the current context. It MUST request only the namespace list and MUST NOT run API discovery.

#### Scenario: Namespaces from the chosen context

- **WHEN** the user types `sofka --context west -n ` and presses Tab
- **THEN** the candidates are the namespaces of `west`

### Requirement: Resource completion

Completion for the positional resource and `--resource` SHALL offer `ctx`, `contexts`, built-in aliases and their targets, config aliases including per-cluster and per-context overrides, and discovered plurals, lowercase kinds, `plural.group` names and short names. Reading config for completion MUST NOT write any file. Words starting with `-` MUST get no resource candidates.

#### Scenario: Discovered short name

- **WHEN** the cluster serves MachineDeployments with short name `md`
- **AND** the user types `sofka m` and presses Tab
- **THEN** the candidates include `md` and `machinedeployments`

### Requirement: Path completion

Completion SHALL offer file paths for `--kubeconfig` and `--validate-plugin-report`, and directories only for `--validate-plugin`.

#### Scenario: Plugin directory

- **WHEN** the user types `sofka --validate-plugin ` and presses Tab
- **THEN** only directories are offered

### Requirement: Plugin command completion

Completion SHALL offer cached catalog IDs and `ID@VERSION` for `plugin describe` and `plugin install`. `plugin install` MUST exclude withdrawn releases and releases incompatible with the running sofka version or platform. `plugin update` and `plugin remove` SHALL offer installed managed package IDs, and the `plugin search` query MUST get no candidates.

#### Scenario: Withdrawn release

- **WHEN** a cached catalog release is withdrawn
- **THEN** `plugin install` completion does not offer it
- **AND** `plugin describe` completion still offers it

### Requirement: Completion candidate hygiene

Completion candidates SHALL be filtered by the typed prefix, sorted and deduplicated, and candidates containing control characters MUST be dropped.

#### Scenario: Control characters

- **WHEN** a namespace name contains a control character
- **THEN** completion does not offer it

### Requirement: Attached option values

Completion SHALL handle values attached to options, so `--context=pr` completes to `--context=prod` and `-npay` completes to `-npayments`.

#### Scenario: Equals form

- **WHEN** the user types `sofka --context=pr` and presses Tab with a context `prod`
- **THEN** the candidate is `--context=prod`

### Requirement: Completion reads earlier words safely

Completion SHALL read earlier words only up to `--` and MUST stop at subcommands. A word consumed as an option value MUST NOT be read as another option.

#### Scenario: Consumed value

- **WHEN** the user types `sofka --validate-plugin --context=wrong -n ` and presses Tab
- **THEN** namespace completion does not use context `wrong`

### Requirement: Completion connection settings

Completion cluster queries SHALL apply `--allow-v1-client-cert` and `--no-tls-resumption` from the command line. Exec credential helpers MUST run non-interactively, and a stalled helper MUST yield an empty, quiet result within 5 seconds.

#### Scenario: Stalled credential helper

- **WHEN** the exec credential helper hangs during namespace completion
- **THEN** completion returns no candidates and no stderr output within 5 seconds
