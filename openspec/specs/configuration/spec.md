# configuration Specification

## Purpose

Describe how sofka finds, parses, layers, validates and live-reloads its user configuration (base file, drop-ins, per-cluster and per-context overrides in TOML or YAML), how it reports configuration problems through stderr, flashes and the `:config` view, and how the general display and session settings that no other capability owns take effect.

## Requirements

### Requirement: Config directory location

sofka SHALL read its configuration from `$XDG_CONFIG_HOME/sofka`. When `XDG_CONFIG_HOME` is unset or empty it SHALL use `$HOME/.config/sofka`, and on Windows it SHALL use `USERPROFILE` as the home directory when `HOME` is unset. When no home directory can be found, sofka MUST run with built-in defaults.

#### Scenario: XDG directory is used

- **WHEN** `XDG_CONFIG_HOME` is `/cfg` and `/cfg/sofka/config.toml` exists
- **THEN** sofka loads `/cfg/sofka/config.toml` as the base config

#### Scenario: Empty XDG variable falls back to HOME

- **WHEN** `XDG_CONFIG_HOME` is set to an empty string and `HOME` is `/home/u`
- **THEN** sofka looks for its config under `/home/u/.config/sofka`

#### Scenario: No home directory

- **WHEN** neither a config home nor a home directory can be found
- **AND** the user opens `:config`
- **THEN** the Sources section shows `no config directory - using defaults`

### Requirement: Supported config file formats

Each config level (base, cluster directory, context directory) SHALL accept exactly one of `config.toml`, `config.yaml` or `config.yml`, and both formats MUST support the same settings, defaults, validation and overrides. Every setting SHALL be optional, and an empty file or a missing file MUST behave like built-in defaults.

#### Scenario: YAML base config

- **WHEN** the config directory contains only `config.yaml` with `readonly: true`
- **THEN** sofka starts in read-only mode exactly as with `readonly = true` in `config.toml`

#### Scenario: Empty config

- **WHEN** `config.toml` exists but is empty
- **THEN** sofka behaves as if no config file existed

### Requirement: Conflicting config files at one level

When more than one supported config file (including symlinks) exists at the same level, sofka MUST report the error `conflicting config files: <A>, <B>; keep one config file at this level` and MUST NOT load any file from that level.

#### Scenario: TOML and YAML side by side

- **WHEN** both `config.toml` and `config.yaml` exist in the base directory at startup
- **THEN** sofka reports the conflict naming both paths
- **AND** starts with default settings
- **AND** `:config` lists both base paths as `conflict - skipped`

#### Scenario: Conflict in an override directory

- **WHEN** a cluster override directory contains both `config.yaml` and `config.yml`
- **THEN** that cluster level is skipped with a warning naming both files
- **AND** the base config and other levels still apply

### Requirement: Invalid base config at startup

When the base config cannot be parsed or has a type error at startup, sofka SHALL print `warning: ignoring invalid <path>: <error>` to stderr, start with default settings, and keep the error visible in the `:config` view. Type errors MUST name the file and the offending key.

#### Scenario: Wrong value type

- **WHEN** the base config contains `readonly = "yes"`
- **THEN** stderr shows a warning naming the config path, `readonly` and `expected a boolean`
- **AND** sofka runs with defaults
- **AND** `:config` lists the base path as `invalid - using defaults` with the error under Warnings

### Requirement: Startup warning reporting

At startup sofka SHALL print every configuration warning to stderr as `warning: <text>` and SHALL keep the warnings for the `:config` view. If a view configuration warning exists, the first one MUST be flashed as an error; otherwise, when any warning exists, sofka SHALL flash `config loaded with N warning(s) — :config for details`.

#### Scenario: Startup with warnings

- **WHEN** the config contains `mouse_scroll_lines = 0`
- **THEN** stderr shows `warning: mouse_scroll_lines: 0 is not allowed — using 1`
- **AND** the TUI flashes `config loaded with 1 warning(s) — :config for details`

#### Scenario: Startup without warnings

- **WHEN** the config produces no warnings
- **THEN** sofka does not change the startup flash message

### Requirement: Drop-in config files

sofka SHALL merge every regular `*.toml`, `*.yaml` and `*.yml` file in `conf.d/` over the base config in sorted file-name order, before cluster and context overrides. Other entries in `conf.d/` MUST be ignored.

#### Scenario: Later drop-in wins

- **WHEN** `conf.d/10-team.toml` sets `default_resource = "deployments"`
- **AND** `conf.d/20-personal.toml` sets `default_resource = "pods"`
- **THEN** the effective `default_resource` is `pods`

#### Scenario: Context override beats drop-ins

- **WHEN** a drop-in sets `readonly = false` and the active context override sets `readonly = true`
- **THEN** the effective `readonly` is `true`

### Requirement: Invalid drop-in files are skipped

A drop-in file with a syntax, parse or type error MUST be skipped with the warning `ignoring invalid <path>: <error>` while the base config and other drop-ins still apply. `:config` SHALL list each drop-in with the state `loaded` or `invalid - skipped`.

#### Scenario: One bad drop-in

- **WHEN** `conf.d/10-bad.toml` contains `readonly = "yes"` and `conf.d/20-good.toml` sets `hide_header = true`
- **THEN** a warning names `conf.d/10-bad.toml`
- **AND** `hide_header` is `true`

### Requirement: Per-cluster and per-context override files

sofka SHALL read a cluster override from `clusters/<cluster>/config.*` and a context override from `clusters/<cluster>/<context>/config.*`, where `<cluster>` is the kubeconfig cluster name and `<context>` the kubeconfig context name. An empty cluster name MUST skip both override levels, and an empty context name MUST skip only the context level. A context file SHALL NOT require a cluster file.

#### Scenario: Context-only override

- **WHEN** only `clusters/prod-cluster/prod-admin/config.toml` exists with `readonly = true`
- **AND** the user switches to context `prod-admin` on cluster `prod-cluster`
- **THEN** sofka runs in read-only mode for that context

### Requirement: Override directory name sanitising

sofka MUST derive override directory names by replacing every character outside `A-Z`, `a-z`, `0-9`, `.`, `_` and `-` with `-`. A name made only of dots SHALL have each dot replaced with `-`, and an empty name SHALL become `-`.

#### Scenario: EKS ARN context

- **WHEN** the context is `arn:aws:eks:eu-west-1:123456789:cluster/prod`
- **THEN** sofka reads its context override from the directory `arn-aws-eks-eu-west-1-123456789-cluster-prod`

#### Scenario: Dot-only name

- **WHEN** a kubeconfig name is `..`
- **THEN** the override directory name is `--`

### Requirement: Layering order and merge rules

sofka SHALL build the effective config by merging, in order, the base file, drop-ins, the cluster override and the context override. Tables MUST merge key by key, while strings, numbers, booleans and arrays (including `[[plugins]]`) MUST replace the earlier value.

#### Scenario: Tables merge, arrays replace

- **WHEN** the base sets `[aliases] dep = "deployments"` and `favorite_namespaces = ["a", "b"]`
- **AND** the cluster override sets `[aliases] st = "statefulsets"` and `favorite_namespaces = ["c"]`
- **THEN** both aliases `dep` and `st` are active
- **AND** the favourite namespaces are exactly `["c"]`

### Requirement: Invalid override files are skipped

A cluster or context override file with a syntax or parse error MUST be skipped with the warning `ignoring invalid <path>: <error>`. If the merged drop-ins and overrides produce a type error, sofka MUST fall back to the base config alone with the warning `ignoring drop-in and cluster overrides: <error>`.

#### Scenario: Malformed cluster override

- **WHEN** `clusters/prod/config.toml` contains invalid TOML
- **THEN** sofka warns `ignoring invalid` with that path
- **AND** applies the base config and any valid context override

#### Scenario: Merged type error

- **WHEN** a context override sets `hide_header = "yes"`
- **THEN** sofka warns `ignoring drop-in and cluster overrides:` with the type error
- **AND** uses the base config for that context

### Requirement: Overrides re-read on context switch

sofka SHALL re-read drop-ins and the target cluster and context override files on every context switch and on `:reload`, so override edits apply without a restart. On a switch it SHALL reapply the resolved live settings for the new context and flash `context: <name>`, appending the first config warning when one exists.

#### Scenario: Edited override applies on next switch

- **WHEN** the user edits `clusters/prod/config.toml` to add `readonly = true`
- **AND** switches to a context on cluster `prod` with `:ctx`
- **THEN** read-only mode applies without restarting sofka

#### Scenario: Warning shown on switch

- **WHEN** the target context override produces a config warning
- **THEN** the context switch flash includes `context: <name>` and the warning text

### Requirement: Inline plugin merge by name

A config file with `plugins_merge = "name"` SHALL merge its `[[plugins]]` into the inherited list by exact, case-sensitive name: a new name MUST be appended and an existing name MUST be replaced in place. With the default `plugins_merge = "replace"`, a file's `plugins` list MUST replace the inherited list, and `plugins = []` MUST clear it. The setting applies only to the file that declares it.

#### Scenario: Name merge replaces in place

- **WHEN** the base defines plugins `a` and `b`
- **AND** a drop-in with `plugins_merge = "name"` defines `b` and `c`
- **THEN** the plugin order is `a`, the drop-in's `b`, then `c`

#### Scenario: Empty list in name mode

- **WHEN** a file sets `plugins_merge = "name"` and `plugins = []`
- **THEN** the inherited plugins are unchanged

#### Scenario: Default replace

- **WHEN** a cluster override without `plugins_merge` sets `plugins = []`
- **THEN** no inline plugins are active in that cluster

### Requirement: Inline plugin removal

`plugins_remove` SHALL remove inherited inline plugins by exact name before the same file's plugin additions are applied. Unknown names MUST have no effect.

#### Scenario: Remove then add

- **WHEN** the base defines plugin `a`
- **AND** a drop-in sets `plugins_merge = "name"`, `plugins_remove = ["a", "missing"]` and defines plugin `a` again
- **THEN** exactly one plugin `a`, from the drop-in, is active

### Requirement: Invalid plugin merge controls

A file whose `plugins_merge` is not `"replace"` or `"name"`, or whose `plugins_remove` is not an array of strings, MUST be treated as an invalid file and skipped with a warning.

#### Scenario: Bad merge mode

- **WHEN** a drop-in sets `plugins_merge = 'invalid'`
- **THEN** that drop-in is skipped with an `ignoring invalid` warning

#### Scenario: Remove is not a list

- **WHEN** a drop-in sets `plugins_remove = 'a'`
- **THEN** that drop-in is skipped with an `ignoring invalid` warning

### Requirement: YAML document shape

A YAML config MUST be a single mapping document from setting names to values. An empty file, a comment-only file or `{}` SHALL mean defaults. A non-mapping root MUST be rejected with `config must be a mapping of setting names to values`, and YAML anchors and aliases SHALL be resolved.

#### Scenario: Root is a list

- **WHEN** `config.yaml` contains `- readonly`
- **THEN** the file is rejected with `config must be a mapping of setting names to values`

#### Scenario: Anchors and aliases

- **WHEN** `config.yaml` defines a value with an anchor and reuses it with an alias
- **THEN** both settings receive the anchored value

### Requirement: Unsupported YAML constructs

sofka MUST reject a YAML config that contains a null value anywhere, a non-string mapping key, a duplicate key, a merge key `<<`, an explicit tag, multiple documents, or an integer outside the signed 64-bit range. Null values MUST report `null is not supported; omit the setting`, non-string keys `mapping keys must be strings`, and merge keys `YAML merge keys are not supported; use explicit settings`.

#### Scenario: Null value

- **WHEN** `config.yaml` contains `default_namespace: ~`
- **THEN** the file is rejected with an error containing `null is not supported`

#### Scenario: Merge key

- **WHEN** `config.yaml` uses `<<: *defaults`
- **THEN** the file is rejected with `YAML merge keys are not supported; use explicit settings`

### Requirement: Reload configuration live

`:reload` SHALL re-read the base config, drop-ins and active overrides and reapply live settings, including aliases, favourite namespaces, `remember_sort`, `mouse_scroll_lines`, `hide_header`, `terminal_title`, plugins, bookmarks, workspaces, guardrails, keys, views, thresholds, node roles, providers, notifications, saved forwards, skin, `readonly` and `experimental.native_describe`. Running port-forwards MUST keep running.

#### Scenario: Reload applies a change

- **WHEN** the user adds `hide_header = true` to the config and runs `:reload`
- **THEN** the header and logo disappear without a restart

#### Scenario: Reload success flash

- **WHEN** `:reload` finishes with no warnings
- **THEN** sofka flashes `config reloaded`

#### Scenario: Reload with warnings

- **WHEN** `:reload` finishes with two warnings
- **THEN** sofka flashes `config reloaded with 2 warning(s) — :config for details`

### Requirement: Failed reload keeps the previous config

When the base config is invalid or conflicting during `:reload`, sofka MUST keep the previously loaded configuration, record the error as a config warning and flash `config reload failed — previous config kept (:config for details)`. When the base file has been removed, `:reload` SHALL load defaults.

#### Scenario: Syntax error on reload

- **WHEN** the user introduces a TOML syntax error and runs `:reload`
- **THEN** all previously applied settings stay in effect
- **AND** sofka flashes `config reload failed — previous config kept (:config for details)`
- **AND** `:config` shows the base path as `not loaded - previous config kept`

#### Scenario: Base file deleted

- **WHEN** the base config file is deleted and the user runs `:reload`
- **THEN** sofka applies default settings

### Requirement: Reload does not reset session state

`:reload` MUST NOT change the current resource view, namespace or active sort. `default_namespace`, `default_resource` and `prefer_context_namespace` SHALL affect only later starts and context switches. `compact_mode`, `detail_wrap`, `mouse`, `logs.fullscreen`, `[logging]` and `[journal]` MUST NOT be reapplied by `:reload`.

#### Scenario: Default resource change

- **WHEN** the user is viewing pods, sets `default_resource = "deployments"` and runs `:reload`
- **THEN** the pods view stays open

#### Scenario: Compact toggle survives reload

- **WHEN** the config sets `compact_mode = true` and the user turns compact mode off with `ctrl-e`
- **AND** runs `:reload`
- **THEN** compact mode stays off

### Requirement: Config information view

`:config` (aliases `config`, `cfg`) SHALL open a document titled `config — :reload to re-read` with a Sources section, an Active section and a warnings section. Sources MUST list each base path with one of `loaded`, `absent - using defaults`, `invalid - using defaults`, `conflict - skipped`, `conflict - previous config kept` or `not loaded - previous config kept`, followed by each drop-in and active override path with its state.

#### Scenario: Clean config

- **WHEN** a valid `config.toml` and one valid drop-in exist and the user runs `:config`
- **THEN** the base path is listed as `loaded`
- **AND** the drop-in path is listed as `loaded`
- **AND** the document ends with `No validation warnings.`

#### Scenario: Missing override

- **WHEN** the active cluster has no override file
- **THEN** its override path is listed as `absent`

### Requirement: Config view active summary and warnings

The `:config` Active section SHALL show the active skin name (or `auto`), the read-only state, the number of user aliases and the number of plugins. When warnings exist the view MUST show `Warnings [N]` followed by one bullet per warning from the last load, reload or context switch.

#### Scenario: Warnings listed

- **WHEN** the last reload produced the warning `node_roles.label_prefixes: empty prefix ignored`
- **AND** the user runs `:config`
- **THEN** the view shows `Warnings [1]` and a bullet with that text

### Requirement: Default namespace selection

At startup and on context changes sofka SHALL choose the namespace in this order: an explicit destination, the namespace saved for that context, `default_namespace`, then the Kubernetes fallback. With `prefer_context_namespace = true` it MUST instead use: explicit destination, the namespace explicitly set in the target kubeconfig context, the saved namespace, `default_namespace`, then the fallback. The target context's effective config SHALL decide the policy.

#### Scenario: Saved namespace beats default

- **WHEN** `default_namespace = "kube-system"` and the user last chose `monitoring` in this context
- **THEN** sofka opens the context in `monitoring`

#### Scenario: Prefer kubeconfig namespace

- **WHEN** the target context override sets `prefer_context_namespace = true`
- **AND** the kubeconfig context sets namespace `team-a` and the saved namespace is `monitoring`
- **THEN** sofka opens the context in `team-a`

### Requirement: Default resource

sofka SHALL open `default_resource` when a context is selected at startup or when a context switch does not keep the previous kind, and MUST fall back to pods when the setting is absent.

#### Scenario: No default resource

- **WHEN** no `default_resource` is configured and no resource is given on the command line
- **THEN** sofka opens the pods view

#### Scenario: Configured default

- **WHEN** `default_resource = "deployments"`
- **THEN** sofka opens the deployments view at startup

### Requirement: Hide header setting

`hide_header = true` SHALL hide the header and logo, including the one-line compact header, while the footer and command input keep their behaviour. The default MUST be `false`, and the setting SHALL follow cluster and context overrides and `:reload`.

#### Scenario: Hidden header gives more rows

- **WHEN** `hide_header = true` is reloaded while a table is open
- **THEN** the table header moves up and the table gains the rows the header used

#### Scenario: Command input still visible

- **WHEN** `hide_header = true` and compact mode is on
- **AND** the user types `:reload`
- **THEN** the command input `:reload` is visible

### Requirement: Compact mode setting

`compact_mode = true` SHALL start sofka with a one-line header and no footer, resolved from the startup context's effective config. The default MUST be `false`. `ctrl-e` SHALL toggle compact mode for the session, and neither `:reload` nor a context switch MUST change the current compact state.

#### Scenario: Context override at startup

- **WHEN** the base sets `compact_mode = false` and the startup context override sets `compact_mode = true`
- **THEN** sofka starts in compact mode

#### Scenario: Context switch keeps toggle

- **WHEN** the user toggles compact mode on and switches to a context whose config sets `compact_mode = false`
- **THEN** compact mode stays on

### Requirement: Document wrap setting

`detail_wrap = true` SHALL enable line wrapping at startup for document views. The default MUST be `false`. The `w` key SHALL change wrapping for the session, and new documents, `:reload` and context switches MUST keep the current session wrap state. The `w` choice MUST NOT be written to the config file.

#### Scenario: Wrap from config

- **WHEN** `detail_wrap = true` and the user opens a YAML document
- **THEN** long lines wrap

### Requirement: Mouse capture setting

`mouse` SHALL control terminal mouse capture at startup and MUST default to `false`, keeping native text selection. `:mouse` SHALL toggle capture for the session and flash `mouse capture on` or `mouse capture off: drag to select text`. `:reload` MUST NOT change the mouse state.

#### Scenario: Toggle on

- **WHEN** sofka starts without `mouse` configured and the user runs `:mouse`
- **THEN** mouse capture turns on
- **AND** sofka flashes `mouse capture on`

#### Scenario: Toggle off from config

- **WHEN** the config sets `mouse = true` and the user runs `:mouse`
- **THEN** sofka flashes `mouse capture off: drag to select text`

### Requirement: Mouse wheel step setting

`mouse_scroll_lines` SHALL set how many navigation steps one wheel event triggers in captured views and MUST default to `3`. A value of `0` MUST be treated as `1` with the warning `mouse_scroll_lines: 0 is not allowed — using 1`, and a value above `100` MUST be capped with `mouse_scroll_lines: N is above the maximum of 100 — using 100`. The setting SHALL follow overrides and `:reload`.

#### Scenario: Single step

- **WHEN** `mouse_scroll_lines = 1` and mouse capture is on
- **AND** the user scrolls down one wheel notch in a table
- **THEN** the selection moves down one row

#### Scenario: Too large

- **WHEN** `mouse_scroll_lines = 500`
- **THEN** a config warning reports `mouse_scroll_lines: 500 is above the maximum of 100 — using 100`

### Requirement: Terminal title setting

`terminal_title` SHALL default to `true`, letting sofka set the terminal title. `terminal_title = false` MUST stop sofka from changing the title and clear a title it already set. The setting SHALL follow cluster and context overrides and `:reload`. Title content and timing are specified by the navigation capability.

#### Scenario: Disable on reload

- **WHEN** sofka has set the terminal title and the user reloads with `terminal_title = false`
- **THEN** sofka clears the title and stops updating it

### Requirement: Remember sort setting

`remember_sort` SHALL default to `true`, saving and restoring sort choices per resource kind. With `remember_sort = false`, sort changes MUST be temporary while existing saved choices stay on disk. The setting SHALL follow overrides and `:reload`, and a reload MUST keep the active sort.

#### Scenario: Sort not saved

- **WHEN** `remember_sort = false` and the user changes the sort with `S`
- **AND** restarts sofka
- **THEN** the view opens with its configured or default sort

### Requirement: Experimental native describe setting

`experimental.native_describe = true` SHALL switch the `d` describe backend to the native implementation, following overrides and `:reload`. The `--experimental-describe` flag MUST enable it for the whole process regardless of config, across reloads and context switches.

#### Scenario: CLI opt-in survives reload

- **WHEN** sofka starts with `--experimental-describe` and the config does not enable it
- **AND** the user runs `:reload`
- **THEN** describe still uses the native backend

### Requirement: Node role label sources

`[node_roles]` SHALL accept `label_prefixes` and `label_keys` that add role sources to the node `ROLES` column while the built-in sources stay active. Empty prefixes MUST be ignored with the warning `node_roles.label_prefixes: empty prefix ignored`. The settings SHALL follow overrides and `:reload`.

#### Scenario: Custom prefix

- **WHEN** `label_prefixes = ["node-role.example.com/"]` and a node has label `node-role.example.com/gpu`
- **THEN** the node's ROLES include `gpu`

#### Scenario: Empty prefix

- **WHEN** `label_prefixes = [""]`
- **THEN** a config warning reports `node_roles.label_prefixes: empty prefix ignored`

### Requirement: Feature sections configured elsewhere

The config file SHALL accept sections whose behaviour other capabilities own, and those sections MUST take part in the same layering, overrides, validation and reload rules: `[views]`, `[thresholds]`, `[[plugins]]`, `[[bookmarks]]`, `[[workspaces]]`, `[[forwards]]`, `[[guardrails]]`, `readonly`, `[journal]`, `[logs]`, `[notify]`, `[keys]`, `[skin]`, `[debug]`, `[bundle]`, `[logging]`, `[pvc_explore]`, `[providers]`, `[fleet]`, `[aliases]` and `favorite_namespaces`.

#### Scenario: Override for an owned section

- **WHEN** the base config sets `[aliases] dep = "deployments"`
- **AND** a cluster override sets `[aliases] dep = "daemonsets"`
- **THEN** in that cluster `:dep` opens daemonsets

### Requirement: Restart-only settings

`[logging]` SHALL be read from the base config only, and `[logging]` and `[journal]` MUST take effect only at startup. `compact_mode`, `detail_wrap`, `mouse` and `logs.fullscreen` SHALL seed session state once at startup.

#### Scenario: Logging override ignored

- **WHEN** a cluster override sets a different `[logging]` level
- **THEN** sofka keeps the logging configured by the base config
