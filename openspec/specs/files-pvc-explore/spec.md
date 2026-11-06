# files-pvc-explore Specification

## Purpose

File access covers copying files to and from a pod with `t` and `kubectl cp`, and browsing a PersistentVolumeClaim's contents in a two-pane file browser opened with `x` or `:pvc-explore`. The browser runs through a pod that already mounts the claim or a short-lived helper pod, supports downloads, uploads with progress bars and a shell in the volume, and `:pvc-clean` removes helper pods a crashed session left behind.

## Requirements

### Requirement: Pod transfer menu

`t` on a pod row, or `t` on a container in the containers picker, SHALL open a menu titled `Transfer: <pod>` (or `<pod>:<container>`) with `Download from pod`, `Upload to pod` and `Cancel`. `esc`, `q` or `Cancel` close it. In read-only mode, choosing `Upload to pod` MUST be refused with the read-only flash; downloads stay available.

#### Scenario: Upload blocked in read-only mode

- **WHEN** read-only mode is on and the transfer menu is open
- **AND** the user chooses `Upload to pod`
- **THEN** no prompt opens and the read-only flash is shown

### Requirement: Transfer path prompts

`Download from pod` SHALL ask `Download from <target> — remote path:` and then `Download <pod>:<src> — local path:` prefilled with the source's base name. `Upload to pod` SHALL ask `Upload to <target> — local file:` and then `Upload <src> to <pod> — remote path:`. An empty answer at either step MUST flash `no path given — transfer cancelled`.

#### Scenario: Download chains two prompts

- **WHEN** the user chooses `Download from pod` on pod `web`
- **AND** enters `/var/log/app.log`
- **THEN** the next prompt reads `Download web:/var/log/app.log — local path:` with `app.log` filled in

### Requirement: Transfer execution

A download SHALL start immediately. An upload SHALL pass the `transfer` guardrail, confirming `Upload <src> to <pod>:<dest>?` only when a rule requires it. The copy SHALL run off-thread as `kubectl cp -n <ns> [-c <container>] <from> <to>` pinned to the active context, show `copying <from> → <to>…`, and flash `copied <from> → <to>` or `cp failed: <error>`, using the most specific line of `kubectl cp`'s error output. It is recorded in `:journal` as `cp upload` or `cp download`.

#### Scenario: Container picker pins the container

- **WHEN** a download is started from the containers picker on container `app` of pod `web`
- **THEN** the command includes `-c app`

### Requirement: Transfer progress

While a copy runs, the status bar SHALL show its percentage (`copying <from> → <to> (<N>%)…`), or the bytes moved so far when the source's total cannot be measured. The total comes from the listing for a single file and from `du` for a directory; what already existed at the destination MUST NOT count as progress.

#### Scenario: Copy without a measurable total

- **WHEN** the pod has no `du` and a directory is downloaded
- **THEN** the status bar shows a growing byte count instead of a percentage

### Requirement: Upload size watcher limits

An upload's destination size SHALL be measured by one `kubectl exec` that prints the size about once a second and stops when its input closes. If sofka exits before closing it, the watcher MUST stop on its own after fifteen minutes, or after 300 passes where the container has no clock.

#### Scenario: Quit during an upload

- **WHEN** sofka is killed while an upload is running
- **THEN** the size loop in the pod ends by its own time or pass limit

### Requirement: PVC explore entry points

`x` on a PVC row, `:pvc-explore` or `:pvc-browse` SHALL open the browser for the selected claim, and `s` on a PVC row SHALL open a shell in the volume instead. The palette commands MUST flash `PVC explore only works on persistentvolumeclaims` outside the PVC table. `:pvc` SHALL keep navigating to the PVC list.

#### Scenario: Palette outside the PVC table

- **WHEN** the pod table is shown
- **AND** the user runs `:pvc-explore`
- **THEN** the flash reads `PVC explore only works on persistentvolumeclaims`

### Requirement: Only bound filesystem claims open

A claim whose `status.phase` is not `Bound` MUST be refused with `<claim> is <phase>, not Bound — nothing to browse`, and a claim with `volumeMode: Block` MUST be refused with `<claim> is a block volume — it has no filesystem to browse`. Neither SHALL create a pod.

#### Scenario: Pending claim

- **WHEN** the selected PVC `data` is `Pending`
- **AND** the user presses `x`
- **THEN** the flash reads `data is Pending, not Bound — nothing to browse`

### Requirement: Browse through an existing pod

sofka SHALL first look for a running, non-terminating pod in the claim's namespace with a running container that mounts the claim, preferring a writable mount, and browse the volume at that container's `mountPath`. Nothing is created, so browsing this way MUST work in read-only mode. While searching the status bar reads `finding a pod that mounts <claim>…`.

#### Scenario: Read-only mode with a consumer pod

- **WHEN** read-only mode is on and a running pod mounts `data` at `/var/lib/data`
- **AND** the user presses `x` on `data`
- **THEN** the browser opens with the volume pane at `/var/lib/data`

### Requirement: Helper pod offer

When nothing mounts the claim, sofka SHALL ask `Nothing mounts <claim>. Create a temporary <image> pod in <ns> to mount it?` before creating a helper pod, always confirming and applying the `pvc-explore` guardrail. In read-only mode it MUST instead flash `browsing <claim> needs a helper pod, which read-only mode blocks`.

#### Scenario: Unused claim

- **WHEN** no running pod mounts `data`
- **AND** the user presses `x` on `data`
- **THEN** the helper pod confirmation opens
- **AND** no pod exists until the user accepts

### Requirement: Helper pod shape

The helper pod SHALL use `generateName: sofka-pvc-explore-`, labels `app.kubernetes.io/managed-by: sofka` and `sofka.dev/component: pvc-explore`, annotation `sofka.dev/pvc: <claim>`, one container `explore` from `[pvc_explore] image` running `sleep <ttl>` with the claim at `/pvc`, `activeDeadlineSeconds` set to the TTL, no service account token, all capabilities dropped, no privilege escalation and the `RuntimeDefault` seccomp profile.

#### Scenario: Helper expires on its own

- **WHEN** a helper pod is created with the default TTL
- **THEN** its `activeDeadlineSeconds` is 1800 and its command is `sleep 1800`

### Requirement: Helper pod startup

After creating the helper, sofka SHALL show `starting a helper pod for <claim>…` and wait up to 90 seconds for it to run. A pod that fails, cannot be read, or does not start in time MUST be deleted and reported, for example `helper pod <name> did not start within 90s (image pull? unschedulable claim?)`. Creating it is recorded in `:journal`.

#### Scenario: Image cannot be pulled

- **WHEN** the helper pod stays pending for 90 seconds
- **THEN** sofka deletes it and flashes the did-not-start message

### Requirement: Helper pod cleanup

A helper pod SHALL be deleted when the browser closes, when any navigation leaves it, when another claim is opened, and on quit, which waits for the delete. A helper that becomes ready after the user moved on MUST be deleted with `<claim> is ready, but you've moved on — press x on it again`. One from a previous context MUST be left alone with a hint to run `:pvc-clean` there.

#### Scenario: Esc removes the helper

- **WHEN** the browser is open through a helper pod
- **AND** the user presses `esc`
- **THEN** the helper pod is deleted and the previous view returns

### Requirement: Two-pane browser

The browser SHALL show the local filesystem on the left, starting at sofka's working directory, and the volume on the right with focus. `tab`, `←` and `→` switch panes; `j`/`k` and `g`/`G` move in the focused pane; `enter` opens a directory; `⌫` or `-` goes up; `r` re-reads both panes; `esc` or `q` close it.

#### Scenario: Switch to the local pane

- **WHEN** the browser has focus on the volume pane
- **AND** the user presses `←`
- **THEN** the local pane has focus

### Requirement: Volume listing

The volume pane SHALL be read with `ls -A -l` over `kubectl exec`, needing `sh`, `ls` and `head` in the container, as a point-in-time read rather than a watch. A listing taking longer than 20 seconds MUST fail with `listing timed out after 20s`. Entries that cannot be stat'ed still appear with an unknown size `?` and a warning.

#### Scenario: Refresh after an external change

- **WHEN** a file is added to the volume by the workload
- **AND** the user presses `r`
- **THEN** the new file appears in the volume pane

### Requirement: Directory entry cap

Each pane SHALL show at most 5,000 entries of one directory; on the volume side the cap is applied by `head` inside the pod so a larger directory is never streamed in full.

#### Scenario: Spool directory

- **WHEN** a volume directory holds 20,000 files
- **THEN** the pane lists 5,000 of them

### Requirement: Navigation confined to the mount

Going up in the volume pane MUST stop at the mount point with `already at the top of the volume`. Every listing SHALL verify with `pwd -P` that it landed inside the volume, refusing a symlink that leads outside with `that link points outside the volume — the browser stays inside the mount`. Going up from the local filesystem root flashes `already at the filesystem root`.

#### Scenario: Symlink to the root

- **WHEN** the volume contains a symlink `escape` pointing at `/`
- **AND** the user presses `enter` on it
- **THEN** the pane stays in the current directory and shows the outside-the-volume message

### Requirement: Untrusted entry names

Volume entries named `.` or `..`, or containing `/`, MUST be discarded, so a forged row cannot reach outside the mount or the download directory. A name that is not valid UTF-8 MUST be refused for navigation and copying with `<name> is not valid UTF-8 — sofka cannot address it`.

#### Scenario: Name with a slash

- **WHEN** the listing output contains an entry named `../etc`
- **THEN** that entry is not shown

### Requirement: Failed navigation keeps the view

When a directory cannot be listed, the pane SHALL stay on the directory it was showing, with its cursor, and flash the reason. When the failure concerns the directory on screen and nothing is listed, the pane MUST show the error instead of reading empty.

#### Scenario: Permission denied

- **WHEN** the user presses `enter` on an unreadable volume directory
- **THEN** the pane keeps the previous listing
- **AND** the flash names the error

### Requirement: Copy across panes

`c` SHALL copy the focused pane's selected entry into the directory shown by the other pane: a download when the volume pane has focus, an upload when the local pane does. With nothing selected it flashes `nothing selected to download` or `nothing selected to upload`. After a copy completes both panes SHALL be re-read.

#### Scenario: Download a file

- **WHEN** the volume pane has focus on `dump.sql`
- **AND** the user presses `c`
- **THEN** `dump.sql` is copied into the local pane's directory
- **AND** it appears in the local pane once the copy finishes

### Requirement: Colon paths refused

A copy whose source or destination path contains `:` MUST be refused before running `kubectl cp`, flashing `kubectl cp cannot address "<path>" — a ':' separates pod from path`.

#### Scenario: Timestamped file name

- **WHEN** the selected volume file is `backup-12:00.tar`
- **AND** the user presses `c`
- **THEN** the copy is refused with the colon message

### Requirement: Download overwrite confirmation

A download from the browser SHALL need no guardrail and work in read-only mode, but when the local destination already exists it MUST ask `<dest> already exists. Overwrite it?` first, and accepting MUST run the download, never an upload.

#### Scenario: Existing local file

- **WHEN** `dump.sql` already exists in the local directory
- **AND** the user presses `c` on `dump.sql` in the volume pane
- **THEN** the overwrite confirmation opens

### Requirement: Upload gating

An upload from the browser MUST be refused with `<claim> is mounted read-only in <pod> — uploads would fail` when the serving mount is read-only, and blocked in read-only mode. It SHALL pass both the `transfer` guardrail on the serving pod and the `pvc-upload` guardrail on the claim, the stronger confirmation winning, labelled `Upload <src> into <claim>:<dest>?`.

#### Scenario: Pod transfer rule covers the claim

- **WHEN** a guardrail requires confirmation for `transfer` on pods in `prod`
- **AND** the user uploads into a claim served by a pod in `prod`
- **THEN** the upload confirmation opens before any copy

### Requirement: Progress bar on the row

While a browser copy runs, the row being copied SHALL show a progress bar in place of its size, for the matching claim, pane and directory only. A download's bar is measured from the local destination; an upload's from the volume destination; a folder is measured as a whole.

#### Scenario: Navigating away hides the bar

- **WHEN** an upload into `/data/in` is running
- **AND** the volume pane shows a different directory with a same-named entry
- **THEN** that entry shows no progress bar

### Requirement: PVC shell

`s` on a PVC row SHALL open a shell at the mount point, and `s` in the browser at the directory the volume pane shows, by running `kubectl exec -it` in the serving container. If that directory is gone the shell starts where the container does with a notice. Both are blocked in read-only mode and recorded in `:journal`.

#### Scenario: Shell from the browser

- **WHEN** the volume pane shows `/data/logs`
- **AND** the user presses `s`
- **THEN** the shell starts in `/data/logs`
- **AND** the browser and its pod remain when the shell exits

### Requirement: PVC shell guardrail

A PVC shell SHALL pass the same `shell` guardrail as a shell on the serving pod, asking `Shell into <claim> via <pod>?` when a rule requires it. When a rule denies shells in the namespace, sofka MUST refuse before creating a helper pod, flashing `blocked by guardrail: shell pods — not creating a helper pod for <claim>`.

#### Scenario: Denied shell creates nothing

- **WHEN** a guardrail denies `shell` on pods in the claim's namespace and nothing mounts the claim
- **AND** the user presses `s` on the PVC row
- **THEN** no helper pod confirmation opens

### Requirement: Shell helper lifetime

A shell opened from a PVC row through a helper pod SHALL keep the helper until the shell returns and then delete it. Cancelling the shell's confirmation or prompt MUST delete the helper as well.

#### Scenario: Shell from a row through a helper

- **WHEN** a shell from a PVC row runs in a helper pod
- **AND** the user exits the shell
- **THEN** the helper pod is deleted

### Requirement: Missing browsing tools recovery

When the first listing through an existing pod fails because `sh`, `ls` or `head` is missing, sofka SHALL try up to 16 other running containers that mount the same claim with the same `subPath`, keeping any read-only restriction. If none works it SHALL offer a helper pod, showing the original error and `Browse with helper pod? Create a temporary <image> pod in <ns> for <claim>.`

#### Scenario: Distroless consumer

- **WHEN** the only consumer container has no shell and no other container mounts the claim
- **THEN** sofka offers a helper pod with the original error shown above the question

### Requirement: Recovery helper placement

A recovery helper SHALL keep the original mount's `subPath` and read-only setting. For an in-use `ReadWriteOnce` claim it SHALL be pinned to the consumers' node. Recovery MUST NOT create a helper for an occupied `ReadWriteOncePod` claim, a `subPathExpr` mount, unknown access modes, or consumers on several nodes, and SHALL report why.

#### Scenario: ReadWriteOncePod in use

- **WHEN** a `ReadWriteOncePod` claim is in use by a pod without a shell
- **THEN** the browser shows `ReadWriteOncePod is already in use. A second pod cannot mount this claim.`

### Requirement: Recovery limits

Recovery SHALL start only for missing-tool errors on the first listing; permission errors, connection failures and invalid paths MUST keep their own messages. Recovery obeys read-only mode and the `pvc-explore` guardrail. Cancelling, or another view or dialog opening, SHALL leave the original error in the browser; reopening the claim starts a new attempt.

#### Scenario: Cancel the recovery helper

- **WHEN** the recovery helper confirmation is shown
- **AND** the user presses `n`
- **THEN** the browser still shows the original error
- **AND** no helper pod is created

### Requirement: Helper pod sweep

`:pvc-clean` (also `:pvc-cleanup`) SHALL ask `Delete every sofka PVC-explore helper pod in <scope>, including any another session is using?` for the current namespace, or all namespaces when the view spans them. It SHALL delete only pods with the `sofka-pvc-explore-` prefix, both helper labels and the `sofka.dev/pvc` annotation, skipping the helper the open browser uses.

#### Scenario: Sweep the current namespace

- **WHEN** namespace `data` holds a leftover helper and an unrelated pod
- **AND** the user runs `:pvc-clean` and accepts
- **THEN** only the helper is deleted

### Requirement: Helper sweep gating and result

`:pvc-clean` MUST be blocked in read-only mode, matched by the `pvc-explore` guardrail (treated as cluster-wide for an all-namespaces sweep), recorded in `:journal`, and refused on a namespace pattern with `select one namespace or all namespaces to clean PVC helpers`. It SHALL report `removed <N> PVC helper pod(s)` or `pvc-clean: removed <N>, <M> failed — <first errors>`.

#### Scenario: Namespace guardrail on a cluster-wide sweep

- **WHEN** a guardrail requires confirmation for `pvc-explore` in `prod` and the view spans all namespaces
- **AND** the user runs `:pvc-clean`
- **THEN** the guardrail's confirmation applies

### Requirement: PVC explore configuration

`[pvc_explore] image` SHALL set the helper image (default `busybox:1.37`) and `ttl` its lifetime (default `30m`). An empty image MUST warn `pvc_explore: image is empty — helper pods cannot be created`; an unparseable or non-positive TTL MUST warn and fall back to 30 minutes.

#### Scenario: Invalid TTL

- **WHEN** `[pvc_explore] ttl = "0s"`
- **THEN** config warnings include `pvc_explore: ttl: "0s" must be a positive duration; using 30m`
- **AND** helper pods live 30 minutes
