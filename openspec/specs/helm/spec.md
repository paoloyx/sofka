# helm Specification

## Purpose

The native Helm release inspector: `:helm` lists Helm releases by decoding Helm's own release storage Secrets with no `helm` binary, drills into revision history, values, manifests and notes, and shells out to the real `helm` binary only for rollback and uninstall.

## Requirements

### Requirement: Helm view opens from the palette

`:helm` and `:hm` SHALL open the Helm release view, watching Secrets labelled `owner=helm` with field selector `type=helm.sh/release.v1` across the current namespace scope. It SHALL clear the drill stack and filter, flash "Viewing Helm releases", and SHALL NOT be recorded in the `[`/`]` view history. Without a secrets kind it SHALL flash "secrets kind unavailable".

#### Scenario: Open the Helm view

- **WHEN** the user runs `:helm`
- **THEN** the view titled "helm" lists Helm releases
- **AND** the flash reads "Viewing Helm releases"

### Requirement: View titles name Helm, not secrets

The resource and list titles SHALL read "helm" for the release list and "helm history" for revision history, never "secrets".

#### Scenario: History title

- **WHEN** the user drills from a release into its history
- **THEN** the title reads "helm history"

### Requirement: One row per release at its latest revision

The release list SHALL show exactly one row per release and namespace, for the highest revision number, like `helm list`. A newly arriving revision SHALL replace the previous row even after rows were displayed.

#### Scenario: Superseded revision hidden

- **WHEN** release `myapp` has revision 1 superseded and revision 2 deployed, and release `other` has revision 1
- **THEN** two rows are shown and `myapp` shows revision 2

#### Scenario: New revision arrives

- **WHEN** the list shows `myapp` at revision 1 and revision 2 is stored
- **THEN** the `myapp` row shows revision 2

### Requirement: Release list columns

The release list SHALL show NAME (the release name, not the Secret name), REVISION ("-" when unknown), STATUS (decoded release status, or "<invalid>" when the payload cannot be decoded), CHART ("<chart>-<version>", or "<unknown>"), APP VERSION and UPDATED.

#### Scenario: Release row cells

- **WHEN** release `myapp` revision 2 of chart `mychart` 1.0.0 is deployed
- **THEN** the row reads `myapp`, `2`, `deployed`, `mychart-1.0.0`

### Requirement: History columns

The revision history SHALL show REVISION, STATUS, CHART, APP VERSION, DESCRIPTION and UPDATED, one row per stored revision with no deduplication.

#### Scenario: All revisions listed

- **WHEN** release `myapp` has revisions 1 and 2
- **THEN** the history shows two rows

### Requirement: UPDATED is a live relative time

UPDATED SHALL show the humanized time since the release's last deployment (e.g. "59s", "1m", "1h"), advancing with the clock in both views without decoding the release again. A deploy time in the future SHALL show "0s", and a missing or unparseable time SHALL show "<unknown>".

#### Scenario: Clock advances

- **WHEN** a release deployed 59 seconds ago is shown and one more second passes
- **THEN** UPDATED changes from "59s" to "1m"

#### Scenario: Unparseable deploy time

- **WHEN** the release's last-deployed time is invalid
- **THEN** UPDATED reads "<unknown>"

### Requirement: Numeric sorting of REVISION and UPDATED

Sorting by REVISION SHALL order numerically, and sorting by UPDATED SHALL order by the deploy timestamp with the most recent first and unknown times last, never by the displayed text.

#### Scenario: Revision sort

- **WHEN** releases at revisions 4, 61 and 11951 are sorted by REVISION
- **THEN** they appear as 4, 61, 11951

### Requirement: Filter matches release names

The `/` filter in Helm views SHALL match the release name and SHALL NOT match the storage Secret name.

#### Scenario: Secret name not matched

- **WHEN** the user filters by "sh.helm.release"
- **THEN** no rows are shown

### Requirement: Release payload decoding

sofka SHALL decode the `release` data of a Helm storage Secret as Helm does (base64 twice, gunzip, JSON). Only Helm's Secret storage driver SHALL be supported. A payload that cannot be decoded SHALL be shown as unrenderable rather than failing the view.

#### Scenario: Corrupt payload

- **WHEN** a release Secret holds an undecodable payload
- **THEN** its STATUS cell reads "<invalid>"

### Requirement: Enter on a release opens its history

Enter on a release row SHALL drill into that release's revision history in the release's namespace, labelled `owner=helm,name=<release>`, with scope label "helm/<release>" and flash "↳ <release> history". `esc` SHALL return to the release list. A row that is not a release Secret SHALL flash "not a Helm release secret".

#### Scenario: Drill and back

- **WHEN** the user presses Enter on release `myapp` and then `esc`
- **THEN** the history for `myapp` opens and `esc` returns to the release list

### Requirement: Enter on a revision shows values

Enter on a history row SHALL open a document titled "<release> v<revision> — values" showing that revision's user-supplied values as YAML.

#### Scenario: Show values

- **WHEN** the user presses Enter on revision 2 of `myapp`
- **THEN** a document titled "myapp v2 — values" lists the supplied values

### Requirement: y shows the rendered manifest

`y` on a release or revision row SHALL open a document titled "<release> v<revision> — manifest" with the rendered manifest instead of the Secret's YAML.

#### Scenario: Manifest view

- **WHEN** the user presses `y` on release `myapp` revision 1
- **THEN** a document titled "myapp v1 — manifest" shows the rendered resources

### Requirement: d shows release notes

`d` on a release or revision row SHALL open a document titled "<release> v<revision> — notes" with the rendered NOTES, or "<no notes>" when there are none, instead of describing the Secret.

#### Scenario: Notes view

- **WHEN** the user presses `d` on a release whose notes say "thanks for installing"
- **THEN** a notes document containing that text opens

### Requirement: Undecodable revisions are reported

Enter on a revision, `y` and `d` SHALL flash "could not decode this Helm release revision" when the payload cannot be decoded.

#### Scenario: Corrupt revision

- **WHEN** the user presses `y` on a revision with a corrupt payload
- **THEN** the warning "could not decode this Helm release revision" is shown

### Requirement: Rollback from history with r

`r` on a history row SHALL ask "Roll back <release> in <namespace> to revision <revision>?" and, on confirmation, run `helm [--kube-context <context>] rollback <release> <revision> -n <namespace>`. It SHALL be refused in read-only mode, SHALL flash "could not determine this revision's number" without a revision, and SHALL be journaled as "helm rollback to <revision>".

#### Scenario: Roll back to revision 1

- **WHEN** the user selects revision 1 of `myapp` and presses `r`
- **THEN** a confirmation mentioning "revision 1" appears

### Requirement: Rollback progress and result

During a rollback the status SHALL show "rolling back <release> to revision <revision>…"; success SHALL flash "rolled back <release> to revision <revision>", and failure SHALL flash "helm rollback <release> failed: <first stderr line>" or "helm not found on PATH" as an error.

#### Scenario: helm missing

- **WHEN** a confirmed rollback runs without `helm` on PATH
- **THEN** the error "helm rollback <release> failed: helm not found on PATH" is shown

### Requirement: Uninstall replaces delete

`ctrl-d` on Helm rows SHALL offer `helm uninstall` instead of deleting the storage Secret, for the marked releases or the selected one, refused in read-only mode. It SHALL confirm "Uninstall Helm release <release> in <namespace>? This deletes all its resources." or "Uninstall N Helm releases? This deletes all their resources." and run `helm [--kube-context <context>] uninstall <release> -n <namespace>` per release.

#### Scenario: Uninstall confirmation

- **WHEN** the user presses `ctrl-d` on release `myapp` and confirms with `y`
- **THEN** sofka returns to the table showing "uninstalling myapp…"
- **AND** no Kubernetes delete is sent for the Secret

### Requirement: Uninstall progress and result

Uninstall SHALL show "uninstalling <release>…" or "uninstalling N Helm releases…", then "uninstalled <release>" or "uninstalled N Helm releases" when all succeed. Each failure SHALL flash "helm uninstall <release> failed: <reason>" and suppress the success message. It SHALL be journaled as "helm uninstall".

#### Scenario: One uninstall fails

- **WHEN** one of two confirmed uninstalls exits non-zero
- **THEN** an error flash names that release and no success message is shown

### Requirement: Helm commands target the active context

Every `helm` invocation SHALL pass `--kube-context <active context>` so it acts on the cluster sofka is viewing.

#### Scenario: Context pinned

- **WHEN** sofka is connected to context `test` and runs a Helm action
- **THEN** the command begins `helm --kube-context test`

### Requirement: Unsupported actions on Helm rows

On Helm views, `X` explain SHALL flash "explain is not available for Helm releases", `u` "adjacent is not available for Helm releases", `:gitops` "GitOps view is not available for Helm releases", `:argocd` "Argo CD view is not available for Helm releases", `:notify` "notify is not available for Helm views", and `:bundle` "bundle is not available for Helm releases".

#### Scenario: Explain refused

- **WHEN** the user presses `X` on a Helm release
- **THEN** the warning "explain is not available for Helm releases" is shown

### Requirement: Helm key hints

The key hints SHALL show history, yaml, describe and uninstall on the release list, and values, rollback and uninstall on revision history.

#### Scenario: History hints

- **WHEN** the revision history is shown
- **THEN** the hints include values, rollback and uninstall
