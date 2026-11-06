# plugin-catalog Specification

## Purpose

The `sofka plugin` command line finds, verifies, installs, updates, lists and removes plugin packages from the official catalog and from user-configured catalogs. Installations are atomic and recorded, so local edits are detected.

## Requirements

### Requirement: Plugin subcommands

`sofka plugin` SHALL provide these subcommands:

- `search [QUERY] [--offline] [--json]`
- `describe ID[@VERSION] [--offline] [--json]`
- `install ID[@VERSION]... [--offline]`
- `update [ID...] [--offline]`
- `list [--json]`
- `remove ID...`

A failed subcommand SHALL print the error and exit with a non-zero status. `sofka --resource plugin` SHALL open a Kubernetes resource named `plugin`.

#### Scenario: Failed command exit status

- **WHEN** `sofka plugin install unknown-id` cannot find the plugin
- **THEN** the command prints `unknown plugin "unknown-id"` and exits non-zero

### Requirement: Catalog selection flag

The global `--catalog NAME` flag SHALL restrict search, describe, install and update to one enabled catalog. Using it with `list` or `remove` SHALL fail with `--catalog applies to search, describe, install, and update`. An unknown or disabled name SHALL fail with `unknown or disabled catalog "NAME"`.

#### Scenario: Catalog flag on list

- **WHEN** the user runs `sofka plugin --catalog team list`
- **THEN** the command fails with `--catalog applies to search, describe, install, and update`

### Requirement: Plugin ID syntax

Plugin IDs SHALL consist of lowercase ASCII letters, digits and hyphens. Search, describe, install and update SHALL also accept `ID@VERSION`. An invalid ID SHALL fail with `invalid plugin ID "x"; use lowercase ASCII letters, digits, and hyphens`.

#### Scenario: Uppercase ID

- **WHEN** the user runs `sofka plugin describe Trivy`
- **THEN** the command fails with the invalid plugin ID message

### Requirement: Ambiguous plugin IDs

When an ID exists in more than one enabled catalog and `--catalog` is not given, sofka SHALL fail with `plugin "X" exists in multiple catalogs; use --catalog NAME`.

#### Scenario: Same ID in two catalogs

- **WHEN** catalogs `official` and `team` both list `local-test` and the user runs `sofka plugin install local-test`
- **THEN** the command fails with a message that mentions multiple catalogs

### Requirement: Search

`search` SHALL match the query case-insensitively as a substring of the id, display name, description or tags, sorted by id. An empty query SHALL list every plugin. No matches SHALL produce empty output and success. Each text row SHALL be `catalog<TAB>id<TAB>latest|-<TAB>compatible|incompatible[ installed=V][ withdrawn: R]<TAB>description`.

#### Scenario: Query without matches

- **WHEN** the user runs `sofka plugin search zzz-nothing`
- **THEN** no rows are printed and the command exits successfully

### Requirement: Search JSON output

`search --json` SHALL print an array of objects with `catalog`, `id`, `display_name`, `description`, `tags`, `latest_version`, `compatible`, `installed`, `installed_version` and `withdrawal_reason`.

#### Scenario: JSON fields

- **WHEN** the user runs `sofka plugin search --json`
- **THEN** each element contains exactly those documented fields

### Requirement: Search withdrawal reporting

Search SHALL report why a plugin cannot be installed when its versions were withdrawn. When the installed version was withdrawn, search SHALL show the withdrawal next to the newest replacement version.

#### Scenario: Withdrawn installed version

- **WHEN** the installed version 1.0.0 was withdrawn and 1.1.0 is available
- **THEN** the row shows latest `1.1.0`, `installed=1.0.0` and `withdrawn: <reason>`

### Requirement: Describe

`describe` SHALL report the release chosen by `@VERSION`, or else the latest compatible release, or else the highest version. The text output SHALL include, when present:

- display name and id, catalog, version with `active` or `withdrawn` status, and withdrawal reasons;
- description, publisher, repository, license, required sofka version and platforms;
- each command's details (see below);
- the installed version or `no`.

#### Scenario: Exact version

- **WHEN** the user runs `sofka plugin describe trivy@1.0.0`
- **THEN** version 1.0.0 is described
- **AND** an unknown version fails with `plugin trivy has no version V`

#### Scenario: Only release withdrawn

- **WHEN** a plugin's only release was withdrawn
- **THEN** describe still reports it with status `withdrawn` and the reason

### Requirement: Describe command details

Describe SHALL list each command of the release with its own safety settings: its name, its palette (or `key only`), scopes, command, target, output, mutating, confirmation and network load.

#### Scenario: Multi-command package

- **WHEN** a schema 2 release has two commands with different `mutating` values
- **THEN** describe shows each command with its own mutating setting

### Requirement: Describe JSON output

`describe --json` SHALL print an object with these fields: `catalog`, `id`, `display_name`, `description`, `tags`, `publisher`, `repository`, `version`, `status`, `withdrawal_reason`, `license`, `readme`, `sofka`, `platforms`, `requirements`, `confirmation`, `installed`, `installed_version` and `installed_withdrawal_reason`.

#### Scenario: JSON object fields

- **WHEN** the user runs `sofka plugin describe ID --json`
- **THEN** the object carries the documented fields

### Requirement: Describe JSON execution fields

In `describe --json`, schema 2 releases SHALL list `commands` (`name`, optional `palette` and `key`, `args`, `scopes`, `command`, `target`, `output`, `mutating`, `confirm`, `dangerous`, `network_load`). Legacy releases SHALL put the execution fields at the top level. `confirmation` SHALL be true when any command sets `confirm`, `dangerous` or `network_load`.

#### Scenario: Confirmation flag

- **WHEN** a release has one command with `network_load = true`
- **THEN** `describe --json` reports `confirmation: true`

### Requirement: Describe lists requirements

For each runtime requirement, describe SHALL print `requires: <name> or <alternative>... — <install text>`, naming every command that satisfies it.

#### Scenario: Alternatives

- **WHEN** a requirement `docker` has the alternative `podman`
- **THEN** describe prints `requires: docker or podman — <install>`

### Requirement: Version selection

Without `@VERSION`, sofka SHALL choose the highest stable (non-prerelease), active release that is compatible with the running sofka and has an artifact for this platform or `any`. Failures SHALL be reported as:

- `plugin X has no compatible stable version`
- `plugin X version V was withdrawn: R`
- `plugin X version V requires sofka R, current version is C`
- `plugin X version V has no artifact for P`

#### Scenario: Explicit withdrawn version

- **WHEN** the user installs `x@1.0.0` and 1.0.0 is withdrawn
- **THEN** the install fails with `plugin x version 1.0.0 was withdrawn: <reason>`

### Requirement: Supported platforms

Artifacts SHALL target `x86_64` or `aarch64` combined with `unknown-linux-gnu`, `apple-darwin` or `pc-windows-msvc`, or `any`.

#### Scenario: Platform-independent artifact

- **WHEN** a release only has an `any` artifact
- **THEN** it can be installed on every supported platform

### Requirement: Install output

`install` SHALL print one line per package: `installed`, `updated`, `rolled back` or `already installed`, followed by `ID@VERSION`. If anything changed, it SHALL then print `Run :reload in an existing sofka session to load the changes.`. Reinstalling the same intact version SHALL change no files and print no reload hint.

#### Scenario: Reinstall

- **WHEN** `local-test@1.0.0` is installed and intact, and the user installs it again
- **THEN** the output is `already installed local-test@1.0.0` with no reload hint

#### Scenario: Rollback

- **WHEN** 2.0.0 is installed and the user installs `ID@1.0.0`
- **THEN** the output is `rolled back ID@1.0.0`

### Requirement: Missing external tools are warned, not enforced

Before installing or updating, sofka SHALL print one `warning: <id> requires <names> — <install text>` line for each runtime requirement that is missing on this machine. It SHALL still install the package.

#### Scenario: Missing tool

- **WHEN** a package requires `trivy`, which is not on PATH
- **THEN** the warning names `trivy` once and the package is installed

### Requirement: Batch install preparation is all-or-nothing

sofka SHALL download, verify and stage every requested package before it activates any of them. If any preparation fails, it SHALL change no installed package. Requesting two different versions of one ID SHALL fail with `conflicting versions requested for <id>: A and B`, and a repeated ID SHALL install once.

#### Scenario: One bad package in a batch

- **WHEN** a batch contains one valid package and one with a checksum mismatch
- **THEN** the command fails and neither package is installed

### Requirement: Batch activation reports partial failure

Activation SHALL proceed package by package. An activation failure SHALL print `<id>: <error>`, keep the package that was being replaced, and keep earlier successes. The command SHALL end with the reload hint (if anything changed) and fail with `failed to activate: <ids>`.

#### Scenario: Second package fails to activate

- **WHEN** the first of two packages activates and the second fails
- **THEN** the output names the first as installed, prints the reload hint and fails with `failed to activate: <second>`

### Requirement: Conflicting packages warning

When a package being installed shares a plugin name, palette or overlapping key with another installed or staged package, sofka SHALL warn `warning: <id> shares a plugin name or palette command with the package at <path>; sofka loads the first of the two and ignores the other`.

#### Scenario: Two packages claim one palette

- **WHEN** a batch installs two packages that both define palette `scan`
- **THEN** a conflict warning names the other package path

### Requirement: Installation location and record

Managed packages SHALL be installed in `<config dir>/plugins/<id>`, where the config directory is `$XDG_CONFIG_HOME/sofka` or `~/.config/sofka`. Each SHALL contain a `.sofka-install.json` record with the id, version, catalog source and commit, source commit, artifact BLAKE3 and a BLAKE3 digest of every file.

#### Scenario: Record after install

- **WHEN** `install local-test` succeeds
- **THEN** `<config dir>/plugins/local-test/.sofka-install.json` lists every package file with its digest

### Requirement: Installed source is fixed

An installed package SHALL keep its catalog source. Installing or updating it from a different catalog SHALL fail with `plugin <id> belongs to a different catalog; remove it before changing its source`. A different artifact digest for the same version SHALL be refused as a change to an immutable version.

#### Scenario: Different catalog

- **WHEN** `local-test` was installed from catalog `team` and the user installs it with `--catalog other`
- **THEN** the command fails with a message containing `different catalog`

### Requirement: Destination safety

Before writing anything, install SHALL refuse:

- a symlinked destination: `refusing symlinked plugin destination`;
- a destination that is not a directory;
- a directory without a valid installation record, with `refusing unmanaged plugin directory <path>: ...; move or remove it manually`;
- a record that belongs to another id;
- a locally modified installation.

#### Scenario: Manual directory in the way

- **WHEN** `<config dir>/plugins/x` exists without `.sofka-install.json` and the user installs `x`
- **THEN** the command refuses and leaves the directory untouched

### Requirement: Artifact verification

Each downloaded archive SHALL match the catalog's size and BLAKE3 digest before it is extracted. A mismatch SHALL fail with `artifact size mismatch: catalog says X, downloaded Y` or `artifact BLAKE3 mismatch: expected X, received Y`, and nothing SHALL be installed.

#### Scenario: Bad checksum

- **WHEN** a catalog's BLAKE3 for a package does not match the archive
- **THEN** the install fails and no plugin directory is created

### Requirement: Safe extraction

Archives SHALL be zstd-compressed tar files. Extraction SHALL reject:

- links and special files, duplicate paths, and unsafe or non-plain paths (absolute, `..`, `.`, backslashes);
- the reserved names `.sofka-install.json` and `.sofka-install-stage`;
- more than 2000 entries, or content that expands beyond 200 MiB.

The archive SHALL contain `plugin.toml` at its root. Extracted files SHALL be mode 0755 if executable in the archive, and 0644 otherwise.

#### Scenario: Symlink in archive

- **WHEN** an archive contains a symlink entry
- **THEN** the install fails with `archive contains link or special file <path>`

#### Scenario: Missing manifest

- **WHEN** an archive has no `plugin.toml` at its root
- **THEN** the install fails with `archive has no plugin.toml at its root`

### Requirement: Manifest reconciliation with the catalog

A staged package SHALL include a `[package]` table whose version and `sofka` requirement equal the catalog release. Its commands SHALL equal the catalog's commands in identity, arguments, scopes and execution settings. Otherwise sofka SHALL refuse it with `package <id> contradicts the catalog entry it was selected from (...)`. The staged manifest SHALL also pass normal package validation.

#### Scenario: Catalog says non-mutating, manifest mutates

- **WHEN** the catalog lists a command as `mutating = false` but the package manifest sets it to true
- **THEN** the install is refused and nothing is activated

### Requirement: Atomic activation

A package SHALL be staged in a marked directory and moved into place only when complete. A replaced version SHALL be backed up until the swap succeeds. If the swap fails, the previous package SHALL be restored. A destination edited since preparation SHALL be refused with `refusing to replace <id>: ...`.

#### Scenario: Failed swap

- **WHEN** activating a new version fails after the old one was moved aside
- **THEN** the old version is back in place

### Requirement: Operation lock and recovery

Install, update and remove SHALL hold an exclusive lock on `<config dir>/.plugin-install.lock`. A concurrent operation SHALL fail with `another sofka plugin operation holds <path>: ...`. After taking the lock, sofka SHALL finish any interrupted operation: it SHALL remove marked staging directories, restore or discard backups, and empty its own trash.

#### Scenario: Concurrent install

- **WHEN** one `sofka plugin install` is running and a second starts
- **THEN** the second fails with the lock message

#### Scenario: Interrupted replacement

- **WHEN** a previous run stopped after moving the old version to a backup but before activation
- **THEN** the next plugin operation restores that backup to `plugins/<id>`

### Requirement: Trash directory ownership

Removals and replacements SHALL move packages into `<config dir>/.plugin-trash` before deleting them. sofka SHALL claim that directory with a marker file and empty only a directory that carries its marker. An existing unmarked `.plugin-trash` with contents SHALL be refused with `refusing to use <path> for removals: it holds files sofka did not put there; move or remove it manually`.

#### Scenario: Foreign trash directory

- **WHEN** `.plugin-trash` exists without the sofka marker and contains a file
- **THEN** remove fails and the file is not deleted

### Requirement: Update

`update` without IDs SHALL update every managed package, limited to the `--catalog` catalog when given; with none it SHALL print `No managed plugins are installed.`. It SHALL install a newer selectable version, print `<id> is current at V` when none exists, and fetch only the catalogs the selected packages came from.

#### Scenario: Targeted update

- **WHEN** the user runs `sofka plugin update local-test`
- **THEN** only the catalog that `local-test` came from is loaded

#### Scenario: Already current

- **WHEN** the installed version is the newest selectable one
- **THEN** the output is `<id> is current at <version>`

### Requirement: Update reports what it cannot update

An update run SHALL warn about each plugin it cannot update, update the rest, and finally fail with `could not update: <ids>`. Warnings cover:

- `<id> is not a managed installation`
- `<id> at P has local modifications; update refused`
- `<id>: its installed catalog is not enabled or does not match --catalog`
- an installed version the catalog no longer lists
- a catalog error

#### Scenario: One modified package

- **WHEN** two managed packages need updating and one has local modifications
- **THEN** the other is updated
- **AND** the command fails with `could not update: <modified id>`

#### Scenario: Unlisted installed version

- **WHEN** the installed version is no longer in the catalog
- **THEN** a warning says the catalog no longer lists it and names the newest installable version, and the version is never reported as current

### Requirement: Withdrawn installed version during update

When the installed version was withdrawn and no newer release exists, update SHALL warn `<id> is installed at V, which was withdrawn: R; the newest version sofka can install is N`. This SHALL NOT be counted as a failure and SHALL never be reported as current.

#### Scenario: Recalled version

- **WHEN** the installed 1.0.0 was withdrawn and nothing newer is selectable
- **THEN** update prints the withdrawal warning and does not print `is current`

### Requirement: List

`list` SHALL work offline and print one row per directory in `<config dir>/plugins`: `id<TAB>catalog source<TAB>version|-<TAB>managed|manual[ modified][ withdrawn: R]<TAB>path`. A package is managed if it has an installation record, and modified if its files differ from that record. A symlinked package SHALL always be manual. Withdrawal reasons SHALL come from the cached catalog, and missing cache SHALL NOT be an error.

#### Scenario: Managed, manual and edited

- **WHEN** one package was installed, one copied manually and one installed then edited
- **THEN** list labels them `managed`, `manual` and `managed modified`

#### Scenario: Manual package never withdrawn

- **WHEN** a manual package has the same id as a withdrawn catalog release
- **THEN** list shows no withdrawal for it

### Requirement: List JSON output

`list --json` SHALL print an array of objects with `catalog_source`, `id`, `version`, `path`, `managed`, `modified` and `withdrawal_reason`.

#### Scenario: JSON list

- **WHEN** the user runs `sofka plugin list --json` with one managed package
- **THEN** the element has `managed: true` and its installed `version`

### Requirement: Modification detection

An installed package SHALL be modified when any file was added, changed or removed relative to its record. Refusals SHALL name up to 5 moved files (`added`, `changed` or `removed <path>`, then `and N more`) and say `restore it or manage the directory manually`.

#### Scenario: Edited file

- **WHEN** a user edits `plugin.toml` inside a managed package and runs `sofka plugin remove <id>`
- **THEN** removal fails with a message containing `changed plugin.toml`

### Requirement: Remove

`remove ID...` SHALL accept IDs without versions, otherwise failing with `remove accepts plugin IDs without versions`. It SHALL check every ID before removing anything and refuse missing, symlinked, unmanaged or modified packages. It SHALL print `removed <id> from <path>` for each removal, then the reload hint. If a later removal fails, it SHALL still report the earlier ones.

#### Scenario: Not installed

- **WHEN** the user runs `sofka plugin remove ghost`
- **THEN** the command fails with `plugin ghost is not installed at <path>`

### Requirement: Official catalog fetch

Search, describe, install and update SHALL fetch the official catalog once per command. sofka SHALL resolve the head commit of the official catalog repository through the GitHub API (a 40-hex SHA) and then download `index.json` at that commit. A rate limit SHALL fail with `GitHub API rate limit reached; retry later`. Network failure SHALL NOT silently fall back to the cache.

#### Scenario: Network failure with a cache

- **WHEN** the network is unavailable and a cached catalog exists
- **THEN** the error suggests `--offline` to use the cached catalog

### Requirement: Download restrictions

Official downloads SHALL use HTTPS and only `api.github.com`, `github.com`, `raw.githubusercontent.com` and the GitHub release-asset hosts. Otherwise they SHALL fail with `catalog downloads require HTTPS` or `refusing download from untrusted host`. Redirects SHALL be limited to 5. A catalog SHALL be at most 10 MiB and an artifact at most 50 MiB.

#### Scenario: Untrusted redirect

- **WHEN** a download redirects to a host outside the allowed list
- **THEN** the download fails with `refusing download from untrusted host`

### Requirement: Download time budgets

Metadata downloads SHALL be bounded by a 30 second total time. Package downloads SHALL be bounded by 15 minutes in total. Both SHALL be dropped after 30 seconds without progress.

#### Scenario: Stalled package download

- **WHEN** a package download stops receiving data for 30 seconds
- **THEN** the download fails

### Requirement: Offline mode

`--offline` SHALL use the validated cached catalog and print to stderr `using cached catalog from <age> ago; withdrawal information may be stale`. Without a cache it SHALL fail with `offline catalog is unavailable at <path>: ...; run without --offline once`. Offline installs SHALL need a cached artifact, otherwise failing with `artifact <digest> is not cached; run install without --offline once`.

#### Scenario: Offline install from cache

- **WHEN** a package was downloaded once and the user runs `install ID --offline`
- **THEN** it installs from the cached archive without network access

### Requirement: Catalog and artifact cache

The official catalog SHALL be cached at `$XDG_CACHE_HOME/sofka/plugins/catalog-cache.json`, or under `~/.cache` when that is unset. Artifacts SHALL be cached by BLAKE3 digest. A corrupt cached artifact SHALL be removed and SHALL NOT be installed.

#### Scenario: Corrupt cached artifact

- **WHEN** a cached archive no longer matches its digest
- **THEN** the cache entry is deleted and the install does not use it

### Requirement: Catalog index validation

Catalog `schema_version` SHALL be 1 or 2. sofka SHALL reject an index with:

- duplicate ids or versions, or empty display metadata;
- non-HTTPS repositories, or invalid compatibility, source commit, license or readme;
- withdrawn versions without a reason, or active versions with one;
- empty or duplicate artifact platforms;
- BLAKE3 digests that are not 64 hex characters, or sizes outside 1 byte to 50 MiB.

#### Scenario: Withdrawn without reason

- **WHEN** a catalog version is withdrawn without a reason
- **THEN** the catalog is rejected

### Requirement: Catalog command arrays

Command arrays SHALL require catalog schema 2. A release SHALL NOT mix command arrays with legacy execution fields. Command names, palettes and keys SHALL be valid and unique within a release.

#### Scenario: Mixed release

- **WHEN** a release has both `commands` and top-level execution fields
- **THEN** the catalog fails with `catalog release mixes commands and legacy execution fields`

### Requirement: Official artifact origin

Artifacts in the official catalog SHALL be downloaded from the release download URL of the official `sofka-plugins` repository, and any other URL SHALL be rejected.

#### Scenario: Foreign artifact URL

- **WHEN** an official catalog artifact points to another repository
- **THEN** the catalog is rejected

### Requirement: Custom catalogs configuration

`<config dir>/catalogs.toml` SHALL accept `official` (default `true`) and `[[catalogs]]` entries with `name`, `url` and `trusted`, and SHALL reject unknown fields. Names SHALL be valid IDs, unique and not `official`. Locations SHALL be file paths (relative to the config directory) or HTTP/HTTPS URLs, and SHALL be unique. If no catalog is enabled, commands SHALL fail with `no plugin catalogs are enabled`.

#### Scenario: Reserved name

- **WHEN** a catalog is named `official`
- **THEN** commands fail with `duplicate or reserved catalog name`

#### Scenario: Official disabled

- **WHEN** `official = false` and no `[[catalogs]]` exist
- **THEN** commands fail with `no plugin catalogs are enabled`

### Requirement: Untrusted catalogs are refused

A configured catalog without `trusted = true` SHALL make commands that use it fail with `catalog "<name>" is not trusted; review its plugins, then set trusted = true in catalogs.toml`.

#### Scenario: Untrusted local catalog

- **WHEN** `catalogs.toml` lists catalog `team` without `trusted = true`
- **THEN** `sofka plugin search` fails with a message containing `not trusted`

### Requirement: Custom catalog sources

A local catalog SHALL reference package files inside its index directory, otherwise failing with `package path must stay inside the catalog directory`. Catalog and package files SHALL be regular files. An HTTP catalog SHALL download artifacts only from its own scheme and host, without credentials or fragments, otherwise failing with `custom catalog downloads must use the catalog HTTP origin`. Offline HTTP catalogs SHALL use their cache.

#### Scenario: Escaping package path

- **WHEN** a local catalog artifact path is `../outside.tar.zst`
- **THEN** the catalog is rejected

#### Scenario: Offline update from local catalog

- **WHEN** a package was installed from a local catalog and a newer version is published there
- **THEN** `sofka plugin update --offline` installs the newer version from the same source

### Requirement: Search and describe name the catalog

Search and describe SHALL include the catalog name in both text and JSON output. List SHALL include the source identity of each managed package.

#### Scenario: Custom catalog in search

- **WHEN** a plugin comes from catalog `team`
- **THEN** its search row begins with `team`
