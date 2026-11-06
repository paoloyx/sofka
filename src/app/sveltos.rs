use super::*;
use crate::sveltos::{
    self as evidence, ClusterKey, DeploymentEvidence, Freshness, ProfileKey, ProvisioningBucket,
    ProvisioningCounts, ReadAvailability, SourceKind, SourceRead, SveltosModel, SyncMode,
    TargetRelation,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{sync::Semaphore, task::JoinSet, time::Instant};

const PAGE_SIZE: u32 = 500;
const MAX_CONCURRENT_REQUESTS: usize = 4;
const MAX_LIST_REQUESTS: usize = 200;
const MAX_OBJECTS: usize = 20_000;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const GATHER_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const INVENTORY_ONLY: &str =
    "reported inventory only: downstream objects are not read in this view";
const SOURCE_APIS: [(SourceKind, &str, &str); 6] = [
    (
        SourceKind::SveltosCluster,
        "SveltosCluster",
        evidence::SVELTOS_CLUSTER_GROUP,
    ),
    (
        SourceKind::CapiCluster,
        "Cluster",
        evidence::CAPI_CLUSTER_GROUP,
    ),
    (
        SourceKind::ClusterProfile,
        "ClusterProfile",
        evidence::CONFIG_GROUP,
    ),
    (SourceKind::Profile, "Profile", evidence::CONFIG_GROUP),
    (
        SourceKind::ClusterSummary,
        "ClusterSummary",
        evidence::CONFIG_GROUP,
    ),
    (
        SourceKind::ClusterConfiguration,
        "ClusterConfiguration",
        evidence::CONFIG_GROUP,
    ),
];

/// Deployment identity across refreshes: the reported owner with its UID, the
/// target cluster, and the summary object when one exists.
#[derive(Clone, Debug, PartialEq, Eq)]
struct DeploymentKey {
    owner: Option<ProfileKey>,
    owner_uid: Option<String>,
    cluster: Option<ClusterKey>,
    summary: Option<String>,
}

impl DeploymentKey {
    fn of(deployment: &DeploymentEvidence) -> Self {
        Self {
            owner: deployment
                .reported_owner
                .clone()
                .or_else(|| deployment.profile.clone()),
            owner_uid: deployment.owner_uid.clone(),
            cluster: deployment.cluster.clone(),
            summary: deployment.summary.as_ref().map(|summary| {
                let field = |name: &str| text(summary, &format!("/metadata/{name}"));
                format!(
                    "{}/{}#{}",
                    field("namespace").unwrap_or_default(),
                    field("name").unwrap_or_default(),
                    field("uid").unwrap_or_default()
                )
            }),
        }
    }

    /// The same owner (and owner UID) targeting the same cluster, whether or
    /// not a summary has appeared or been replaced since.
    fn same_target(&self, other: &Self) -> bool {
        self.owner.is_some()
            && self.cluster.is_some()
            && self.owner == other.owner
            && self.owner_uid == other.owner_uid
            && self.cluster == other.cluster
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    Profile,
    Cluster,
    Summary,
    Configuration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Page {
    Clusters,
    Profiles,
    Cluster(ClusterKey),
    Profile(ProfileKey),
    Unresolved,
    Deployment(DeploymentKey),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Row {
    Cluster(ClusterKey),
    Profile(ProfileKey),
    Unresolved,
    Deployment(DeploymentKey),
    Source(Source),
    Inventory(String),
}

type Entry = (Row, Vec<String>);

#[derive(Clone, Debug)]
struct PageFrame {
    page: Page,
    selected: usize,
    filter: String,
}

impl PageFrame {
    fn new(page: Page) -> Self {
        Self {
            page,
            selected: 0,
            filter: String::new(),
        }
    }
}

#[derive(Default)]
pub struct State {
    model: SveltosModel,
    inventory: Vec<evidence::InventoryEvidence>,
    source_objects: BTreeMap<SourceKind, Vec<Value>>,
    pages: Vec<PageFrame>,
    inactive_tab: Option<PageFrame>,
    pub(crate) filter_editing: bool,
    pub(crate) filter_draft: String,
    detail_scroll: u16,
    pub refreshing: bool,
    pub last_success: Option<String>,
    pub warning: Option<String>,
}

impl State {
    fn open(&mut self) {
        *self = Self {
            pages: vec![PageFrame::new(Page::Clusters)],
            inactive_tab: Some(PageFrame::new(Page::Profiles)),
            refreshing: true,
            ..Self::default()
        };
    }

    fn frame(&self) -> Option<&PageFrame> {
        self.pages.last()
    }

    fn on_deployment(&self) -> bool {
        matches!(
            self.frame().map(|frame| &frame.page),
            Some(Page::Deployment(_))
        )
    }

    /// Exact identity first, then the one deployment of the same owner and
    /// cluster; anything else is reported as gone rather than guessed.
    fn deployment(&self, key: &DeploymentKey) -> Option<&DeploymentEvidence> {
        let deployments = &self.model.deployments;
        if let Some(found) = deployments
            .iter()
            .find(|deployment| DeploymentKey::of(deployment) == *key)
        {
            return Some(found);
        }
        let mut similar = deployments
            .iter()
            .filter(|deployment| key.same_target(&DeploymentKey::of(deployment)));
        let first = similar.next()?;
        similar.next().is_none().then_some(first)
    }

    fn entries(&self, page: &Page) -> Vec<Entry> {
        match page {
            Page::Clusters => {
                let mut seen = BTreeSet::new();
                let mut rows = self
                    .model
                    .clusters
                    .iter()
                    .map(|cluster| &cluster.key)
                    .chain(
                        self.model
                            .deployments
                            .iter()
                            .filter_map(|deployment| deployment.cluster.as_ref())
                            .filter(|key| key.is_well_formed()),
                    )
                    .filter(|key| seen.insert(*key))
                    .map(|key| (Row::Cluster(key.clone()), self.cluster_cells(key)))
                    .collect::<Vec<_>>();
                rows.extend(self.unresolved_entry());
                rows
            }
            Page::Profiles => {
                let mut seen = BTreeSet::new();
                let mut rows = self
                    .model
                    .profiles
                    .iter()
                    .map(|profile| &profile.key)
                    .filter(|key| seen.insert(*key))
                    .map(|key| (Row::Profile(key.clone()), self.profile_cells(key)))
                    .collect::<Vec<_>>();
                rows.extend(self.unresolved_entry());
                rows
            }
            Page::Cluster(key) => {
                self.deployment_entries(|deployment| deployment.cluster.as_ref() == Some(key))
            }
            Page::Profile(key) => {
                self.deployment_entries(|deployment| deployment.profile.as_ref() == Some(key))
            }
            Page::Unresolved => self.deployment_entries(is_unresolved),
            Page::Deployment(key) => {
                let Some(deployment) = self.deployment(key) else {
                    return Vec::new();
                };
                let mut rows = [
                    Source::Profile,
                    Source::Cluster,
                    Source::Summary,
                    Source::Configuration,
                ]
                .into_iter()
                .map(|source| {
                    let label = match self.source_object(deployment, source) {
                        Ok(object) => object_name(object),
                        Err(reason) => reason,
                    };
                    (
                        Row::Source(source),
                        vec![format!("source · {}: {label}", source_label(source))],
                    )
                })
                .collect::<Vec<_>>();
                if let Ok(section) = self.inventory_section(deployment) {
                    rows.extend(
                        inventory_labels(section)
                            .into_iter()
                            .map(|label| (Row::Inventory(label.clone()), vec![label])),
                    );
                }
                rows
            }
        }
    }

    fn deployment_entries(&self, include: impl Fn(&DeploymentEvidence) -> bool) -> Vec<Entry> {
        self.model
            .deployments
            .iter()
            .filter(|deployment| include(deployment))
            .map(|deployment| {
                (
                    Row::Deployment(DeploymentKey::of(deployment)),
                    deployment_cells(deployment),
                )
            })
            .collect()
    }

    fn unresolved_entry(&self) -> Option<Entry> {
        // Counts rescan every deployment per row; fine for the 20,000-object
        // gather ceiling, index by key if that ceiling grows.
        let count = self
            .model
            .deployments
            .iter()
            .filter(|deployment| is_unresolved(deployment))
            .count();
        (count > 0).then(|| {
            (
                Row::Unresolved,
                vec![
                    "Unresolved".into(),
                    "-".into(),
                    format!("{count} deployments"),
                ],
            )
        })
    }

    fn cluster_cells(&self, key: &ClusterKey) -> Vec<String> {
        let cluster = self
            .model
            .clusters
            .iter()
            .find(|cluster| cluster.key == *key);
        let mut readiness = cluster
            .and_then(|cluster| cluster.readiness.as_deref())
            .map_or_else(|| "unknown".to_owned(), |ready| format!("ready {ready}"));
        match cluster.and_then(|cluster| cluster.control_plane_initialized) {
            Some(true) => readiness.push_str(" · control plane initialized"),
            Some(false) => readiness.push_str(" · control plane not initialized"),
            None => {}
        }
        let flags = cluster.map_or_else(
            || "not observed".to_owned(),
            |cluster| {
                [
                    cluster.paused.then_some("paused"),
                    cluster.deleting.then_some("deleting"),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ")
            },
        );
        vec![
            cluster_type(key).to_owned(),
            key.namespace.clone(),
            key.name.clone(),
            readiness,
            cluster
                .and_then(|cluster| cluster.connectivity.clone())
                .unwrap_or_else(|| "not reported".into()),
            flags,
            counts_label(&self.model.counts_for_cluster(key)),
        ]
    }

    fn profile_cells(&self, key: &ProfileKey) -> Vec<String> {
        let profile = self
            .model
            .profiles
            .iter()
            .find(|profile| profile.key == *key);
        let targets = match profile.and_then(|profile| profile.matching_clusters.as_ref()) {
            None => "unknown".to_owned(),
            Some(clusters) if clusters.is_empty() => "0 · no matching clusters".to_owned(),
            Some(clusters) => clusters.len().to_string(),
        };
        let failures = profile
            .and_then(|profile| profile.failed_clusters.as_ref())
            .map_or(0, Vec::len);
        let suspended_deployments = self
            .model
            .deployments
            .iter()
            .filter(|deployment| {
                deployment.profile.as_ref() == Some(key)
                    && deployment.summary.as_ref().is_some_and(suspended)
            })
            .count();
        let flags = [
            (failures > 0).then(|| format!("{failures} reported failures")),
            profile
                .is_some_and(|profile| suspended(&profile.raw))
                .then(|| "suspended".to_owned()),
            (suspended_deployments > 0)
                .then(|| format!("{suspended_deployments} suspended deployments")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        vec![
            key.kind.clone(),
            key.namespace.clone().unwrap_or_else(|| "<cluster>".into()),
            key.name.clone(),
            profile.map_or_else(|| "unknown".into(), sync_label),
            targets,
            profile.map_or_else(|| "unknown".into(), rollout_summary),
            counts_label(&self.model.counts_for_profile(key)),
            flags,
        ]
    }
}

impl State {
    fn filtered(&self, frame: &PageFrame) -> Vec<Entry> {
        let mut entries = self.entries(&frame.page);
        let filter = frame.filter.to_lowercase();
        if !filter.is_empty() {
            entries.retain(|(_, cells)| cells.join(" ").to_lowercase().contains(&filter));
        }
        entries
    }

    fn rows(&self) -> Vec<Entry> {
        self.frame()
            .map_or_else(Vec::new, |frame| self.filtered(frame))
    }

    fn selected_row(&self) -> Option<Row> {
        let frame = self.frame()?;
        self.filtered(frame)
            .into_iter()
            .nth(frame.selected)
            .map(|(row, _)| row)
    }

    fn step(&mut self, delta: isize) {
        let len = self.rows().len();
        if let Some(frame) = self.pages.last_mut() {
            frame.selected = frame
                .selected
                .saturating_add_signed(delta)
                .min(len.saturating_sub(1));
        }
    }

    fn switch_tab(&mut self) {
        if let ([root], Some(tab)) = (self.pages.as_mut_slice(), self.inactive_tab.as_mut()) {
            std::mem::swap(root, tab);
        }
    }

    pub(crate) fn display_title(&self) -> String {
        match self.frame().map(|frame| &frame.page) {
            None => "Sveltos".into(),
            Some(Page::Clusters) => "Sveltos · Clusters".into(),
            Some(Page::Profiles) => "Sveltos · Profiles".into(),
            Some(Page::Cluster(key)) => format!("{} · deployments", cluster_name(key)),
            Some(Page::Profile(key)) => format!("{} · targets", profile_name(key)),
            Some(Page::Unresolved) => "Sveltos · unresolved evidence".into(),
            Some(Page::Deployment(key)) => match self.deployment(key) {
                Some(deployment) => {
                    let cells = deployment_cells(deployment);
                    format!("{} → {}", cells[0], cells[1])
                }
                None => format!(
                    "{} → {}",
                    key.owner
                        .as_ref()
                        .map_or_else(|| "unresolved profile".into(), profile_name),
                    key.cluster
                        .as_ref()
                        .map_or_else(|| "unresolved cluster".into(), cluster_name)
                ),
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn display_rows(&self) -> Vec<String> {
        self.rows()
            .into_iter()
            .map(|(_, cells)| cells.join(" "))
            .collect()
    }

    pub(crate) fn table_data(&self) -> (Vec<&'static str>, Vec<Vec<String>>) {
        let headings = match self.frame().map(|frame| &frame.page) {
            Some(Page::Clusters) => vec![
                "Type",
                "Namespace",
                "Name",
                "Readiness",
                "Connectivity",
                "Flags",
                "Provisioning",
            ],
            Some(Page::Profiles) => vec![
                "Kind",
                "Namespace",
                "Name",
                "Sync mode",
                "Targets",
                "Rollout (reported)",
                "Provisioning",
                "Flags",
            ],
            Some(Page::Deployment(_)) => vec!["Sources · reported inventory"],
            _ => vec![
                "Profile",
                "Cluster",
                "Rollout (reported)",
                "Provisioning",
                "Flags",
            ],
        };
        let rows = self.rows().into_iter().map(|(_, cells)| cells).collect();
        (headings, rows)
    }

    pub(crate) fn selected(&self) -> usize {
        self.frame().map_or(0, |frame| frame.selected)
    }

    pub(crate) fn filter(&self) -> &str {
        self.frame().map_or("", |frame| frame.filter.as_str())
    }

    pub(crate) fn detail_scroll(&self) -> u16 {
        self.detail_scroll
    }

    fn read_label(&self, kind: SourceKind) -> String {
        match self.model.source_read(kind) {
            None if self.refreshing => "reading".into(),
            None => "not read".into(),
            Some(read) => match (read.availability, read.freshness) {
                (ReadAvailability::Available, Freshness::Stale) => format!(
                    "stale since {}",
                    read.captured_at.as_deref().unwrap_or("unknown")
                ),
                (ReadAvailability::Available, _) => "available".into(),
                (ReadAvailability::Absent, _) => "unavailable (not served)".into(),
                (ReadAvailability::Incomplete, _) => "incomplete".into(),
                (ReadAvailability::Failed, _) => "unreadable".into(),
            },
        }
    }

    pub(crate) fn status_lines(&self) -> Vec<String> {
        vec![
            "management context · all namespaces · read-only · Sveltos v1.16.0 baseline".into(),
            format!(
                "Last successful read: {} · {}",
                self.last_success.as_deref().unwrap_or("never"),
                if self.refreshing {
                    "refreshing"
                } else {
                    "idle"
                }
            ),
            SOURCE_APIS
                .iter()
                .map(|(kind, _, _)| format!("{}: {}", source_name(*kind), self.read_label(*kind)))
                .collect::<Vec<_>>()
                .join(" · "),
        ]
    }

    pub(crate) fn context_lines(&self) -> Vec<String> {
        match self.frame().map(|frame| &frame.page) {
            Some(Page::Profile(key)) => {
                let Some(profile) = self
                    .model
                    .profiles
                    .iter()
                    .find(|profile| profile.key == *key)
                else {
                    return vec![format!(
                        "{} is no longer reported by the management cluster; no other profile is shown in its place.",
                        profile_name(key)
                    )];
                };
                let cells = self.profile_cells(key);
                let spec = profile.raw.get("spec");
                let declared = [
                    "clusterSelector",
                    "clusterRefs",
                    "setRefs",
                    "dependsOn",
                    "tier",
                    "maxUpdate",
                    "stopMatchingBehavior",
                    "continueOnConflict",
                    "continueOnError",
                ]
                .into_iter()
                .filter_map(|field| Some(format!("{field}: {}", spec?.get(field)?)))
                .collect::<Vec<_>>();
                vec![
                    format!(
                        "Targets (status.matchingClusters): {} · Rollout: {} · Sync mode: {}",
                        cells[4], cells[5], cells[3]
                    ),
                    if declared.is_empty() {
                        "Declared targeting: none".into()
                    } else {
                        format!("Declared: {}", declared.join(" · "))
                    },
                ]
            }
            Some(Page::Cluster(key)) => {
                if self.model.clusters.iter().any(|cluster| cluster.key == *key) {
                    let cells = self.cluster_cells(key);
                    vec![format!(
                        "Readiness: {} · Connectivity: {} · Flags: {}",
                        cells[3],
                        cells[4],
                        if cells[5].is_empty() {
                            "none"
                        } else {
                            &cells[5]
                        }
                    )]
                } else {
                    vec![
                        "Cluster object not observed (API unavailable, unreadable, or removed); rows come from profile and summary references."
                            .into(),
                    ]
                }
            }
            Some(Page::Unresolved) => vec![
                "Evidence whose profile owner or cluster reference could not be resolved; nothing is inferred."
                    .into(),
            ],
            _ => Vec::new(),
        }
    }

    pub(crate) fn detail_lines(&self) -> Vec<String> {
        let Some(Page::Deployment(key)) = self.frame().map(|frame| &frame.page) else {
            return Vec::new();
        };
        let Some(deployment) = self.deployment(key) else {
            return vec![
                "Deployment no longer reported by the management cluster.".into(),
                "Its profile, target, or summary disappeared or was replaced; nothing else is shown in its place."
                    .into(),
            ];
        };
        let cells = deployment_cells(deployment);
        let mut lines = vec![
            format!("Profile: {}", cells[0]),
            format!("Cluster: {}", cells[1]),
            format!("Relation: {}", relation_label(deployment.relation)),
            format!("Provisioning: {}", cells[3]),
            format!("Rollout (controller-reported): {}", cells[2]),
        ];
        if deployment.bucket == ProvisioningBucket::Provisioned
            && deployment.rollout_updated != Some(true)
        {
            lines.push(
                "Features report Provisioned, but the profile does not report this target updated; the latest configuration is not confirmed applied."
                    .into(),
            );
        }
        lines.push(format!(
            "Sync mode: {}",
            match deployment.sync_mode {
                SyncMode::Continuous => "Continuous",
                SyncMode::OneTime => "OneTime (apply-once)",
                SyncMode::DryRun => "DryRun (configuration is not applied)",
                SyncMode::Unknown => "unknown",
            }
        ));
        if let Some(diagnostic) = &deployment.diagnostic {
            lines.push(format!("Evidence warning: {diagnostic}"));
        }
        if let Some(message) = &deployment.summary_failure_message {
            lines.push(format!("Summary failure: {message}"));
        }
        let Some(summary) = &deployment.summary else {
            lines.push("Features: no ClusterSummary reported; feature status unknown".into());
            lines.extend(self.closing_lines(deployment));
            return lines;
        };
        if let Some(dependencies) = text(summary, "/status/dependencies") {
            lines.push(format!("Dependencies: {dependencies}"));
        }
        if suspended(summary) {
            lines.push(format!(
                "Reconciliation suspended: {}",
                text(summary, "/status/suspensionReason")
                    .unwrap_or_else(|| "no reason reported".into())
            ));
        }
        if let Some(time) = text(summary, "/status/nextReconcileTime") {
            lines.push(format!("Next reconcile: {time}"));
        }
        if let Some(time) = text(summary, "/metadata/deletionTimestamp") {
            lines.push(format!(
                "Summary removal requested at {time}; Removing and Blocked states describe removal."
            ));
        }
        lines.push("Features (raw controller states):".into());
        let spec_known = summary.pointer("/spec/clusterProfileSpec").is_some();
        for id in ["Helm", "Resources", "Kustomize"] {
            if !deployment.features.iter().any(|feature| feature.id == id) {
                lines.push(format!(
                    "  {id}: {}",
                    if spec_known {
                        "not configured"
                    } else {
                        "configuration unknown"
                    }
                ));
            }
        }
        for feature in &deployment.features {
            lines.push(format!(
                "  {}: {}{}",
                feature.id,
                feature
                    .reported_state
                    .as_deref()
                    .unwrap_or("pending/unknown (no report)"),
                if feature.configured {
                    ""
                } else {
                    " (reported, not configured)"
                }
            ));
            if let Some(count) = feature.consecutive_failures.filter(|count| *count > 0) {
                lines.push(format!("    consecutive failures: {count}"));
            }
            for (label, value) in [
                ("last applied", &feature.last_applied_time),
                ("failure reason", &feature.failure_reason),
                ("failure message", &feature.failure_message),
            ] {
                if let Some(value) = value.as_deref().filter(|value| !value.is_empty()) {
                    lines.push(format!("    {label}: {value}"));
                }
            }
        }
        for release in summary
            .pointer("/status/helmReleaseSummaries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let field = |name: &str| text(release, &format!("/{name}"));
            lines.push(format!(
                "Helm release {}/{}: {}",
                field("releaseNamespace").unwrap_or_else(|| "?".into()),
                field("releaseName").unwrap_or_else(|| "?".into()),
                field("status").unwrap_or_else(|| "status not reported".into())
            ));
            if let Some(chart) = field("chartName") {
                lines.push(format!(
                    "    chart {chart} {} {}",
                    field("chartVersion").unwrap_or_default(),
                    field("repoURL").unwrap_or_default()
                ));
            }
            for (label, name) in [
                ("conflict", "conflictMessage"),
                ("failure", "failureMessage"),
            ] {
                if let Some(message) = field(name) {
                    lines.push(format!("    {label}: {message}"));
                }
            }
        }
        lines.extend(self.closing_lines(deployment));
        lines
    }

    fn closing_lines(&self, deployment: &DeploymentEvidence) -> Vec<String> {
        let inventory = match self.inventory_section(deployment) {
            Ok(section) => match section.features.iter().map(feature_entries).sum::<usize>() {
                0 => "reported empty".to_owned(),
                count => format!("{count} reported entries (listed below)"),
            },
            Err(reason) => reason,
        };
        let caveat = self
            .source_caveat(SourceKind::ClusterConfiguration)
            .map_or_else(String::new, |caveat| format!(" ({caveat})"));
        vec![
            format!("Reported inventory: {inventory}{caveat}"),
            String::new(),
            "Downstream workload runtime health is not verified.".into(),
        ]
    }

    /// Why a source's evidence cannot be used at all, if it cannot.
    fn source_problem(&self, kind: SourceKind) -> Option<String> {
        let Some(read) = self.model.source_read(kind) else {
            return Some("not read yet".into());
        };
        match read.availability {
            ReadAvailability::Absent => Some("unavailable (not served)".into()),
            ReadAvailability::Failed | ReadAvailability::Incomplete
                if !self.source_objects.contains_key(&kind) =>
            {
                Some(format!(
                    "unavailable: {}",
                    read.error.as_deref().unwrap_or("read failed")
                ))
            }
            _ => None,
        }
    }

    fn source_caveat(&self, kind: SourceKind) -> Option<String> {
        let read = self.model.source_read(kind)?;
        match (read.availability, read.freshness) {
            (ReadAvailability::Available, Freshness::Stale) => Some(format!(
                "stale since {}",
                read.captured_at.as_deref().unwrap_or("unknown")
            )),
            (ReadAvailability::Incomplete | ReadAvailability::Failed, _)
                if self.source_objects.contains_key(&kind) =>
            {
                Some("incomplete".into())
            }
            _ => None,
        }
    }
}

impl State {
    /// The management-cluster object behind the selected row, for `y`.
    fn selected_object(&self) -> Result<Value, String> {
        let gone = || "no longer reported by the management cluster".to_owned();
        match self.selected_row() {
            Some(Row::Cluster(key)) => self
                .model
                .clusters
                .iter()
                .find(|cluster| cluster.key == key)
                .map(|cluster| cluster.raw.clone())
                .ok_or_else(|| "cluster object not observed; only references were reported".into()),
            Some(Row::Profile(key)) => self
                .model
                .profiles
                .iter()
                .find(|profile| profile.key == key)
                .map(|profile| profile.raw.clone())
                .ok_or_else(gone),
            Some(Row::Deployment(key)) => {
                let deployment = self.deployment(&key).ok_or_else(gone)?;
                self.source_object(deployment, Source::Summary).cloned()
            }
            Some(Row::Source(source)) => {
                let Some(Page::Deployment(key)) = self.frame().map(|frame| &frame.page) else {
                    return Err(gone());
                };
                let deployment = self.deployment(key).ok_or_else(gone)?;
                self.source_object(deployment, source).cloned()
            }
            Some(Row::Inventory(_)) => Err(INVENTORY_ONLY.into()),
            Some(Row::Unresolved) => Err("open the unresolved list to view a source".into()),
            None => Err("nothing selected".into()),
        }
    }

    fn source_object<'a>(
        &'a self,
        deployment: &'a DeploymentEvidence,
        source: Source,
    ) -> Result<&'a Value, String> {
        let well_formed_cluster = deployment
            .cluster
            .as_ref()
            .filter(|cluster| cluster.is_well_formed());
        let (kinds, candidates): (&[SourceKind], Vec<&Value>) = match source {
            Source::Summary => {
                return deployment
                    .summary
                    .as_ref()
                    .ok_or_else(|| "no ClusterSummary reported".into());
            }
            Source::Profile => {
                let Some(key) = &deployment.profile else {
                    return Err(format!(
                        "profile source unresolved ({})",
                        deployment
                            .diagnostic
                            .as_deref()
                            .unwrap_or("owner not resolved")
                    ));
                };
                (
                    &[SourceKind::ClusterProfile, SourceKind::Profile][..],
                    self.model
                        .profiles
                        .iter()
                        .filter(|profile| {
                            profile.key == *key
                                && (deployment.owner_uid.is_none()
                                    || profile.uid == deployment.owner_uid)
                        })
                        .map(|profile| &profile.raw)
                        .collect(),
                )
            }
            Source::Cluster => {
                let Some(key) = well_formed_cluster else {
                    return Err("cluster reference unresolved".into());
                };
                (
                    &[SourceKind::SveltosCluster, SourceKind::CapiCluster][..],
                    self.model
                        .clusters
                        .iter()
                        .filter(|cluster| cluster.key == *key)
                        .map(|cluster| &cluster.raw)
                        .collect(),
                )
            }
            Source::Configuration => {
                let Some(key) = well_formed_cluster else {
                    return Err("not correlated (cluster reference unresolved)".into());
                };
                (
                    &[SourceKind::ClusterConfiguration][..],
                    SveltosModel::inventory_for_cluster(&self.inventory, key)
                        .into_iter()
                        .map(|inventory| &inventory.raw)
                        .collect(),
                )
            }
        };
        match candidates.as_slice() {
            [object] => Ok(object),
            [] => Err(kinds
                .iter()
                .find_map(|kind| self.source_problem(*kind))
                .unwrap_or_else(|| "no longer reported by the management cluster".into())),
            _ => Err(format!(
                "ambiguous: {} objects share this identity",
                candidates.len()
            )),
        }
    }

    fn inventory_section(
        &self,
        deployment: &DeploymentEvidence,
    ) -> Result<&evidence::InventorySection, String> {
        let Some(profile) = &deployment.profile else {
            return Err("not correlated (profile unresolved)".into());
        };
        self.source_object(deployment, Source::Configuration)?;
        let cluster = deployment.cluster.as_ref().expect("correlated above");
        let configuration = SveltosModel::inventory_for_cluster(&self.inventory, cluster)[0];
        let (prefix, field) = if profile.kind == "ClusterProfile" {
            ("clusterProfileResources[", "clusterProfileName")
        } else {
            ("profileResources[", "profileName")
        };
        configuration
            .sections
            .iter()
            .find(|section| {
                section.name.starts_with(prefix)
                    && text(&section.raw, &format!("/{field}")).as_deref()
                        == Some(profile.name.as_str())
            })
            .ok_or_else(|| "no inventory section for this profile".into())
    }

    /// Records one source read; returns its capture time when it succeeded.
    fn apply_source_result(&mut self, result: SourceResult) -> Option<String> {
        let kind = result.kind;
        match result.read.availability {
            ReadAvailability::Available => {
                self.source_objects.insert(kind, result.objects);
                let captured_at = result.read.captured_at.clone();
                self.model.set_source_read(kind, result.read);
                captured_at
            }
            ReadAvailability::Absent => {
                self.source_objects.remove(&kind);
                self.model.set_source_read(kind, result.read);
                None
            }
            ReadAvailability::Incomplete | ReadAvailability::Failed => {
                let error = result
                    .read
                    .error
                    .clone()
                    .unwrap_or_else(|| "read incomplete".into());
                let previous = self
                    .model
                    .source_read(kind)
                    .filter(|read| {
                        read.availability == ReadAvailability::Available
                            && self.source_objects.contains_key(&kind)
                    })
                    .cloned();
                match previous {
                    Some(previous) => self.model.set_source_read(kind, previous.stale(error)),
                    None => {
                        if !result.objects.is_empty() {
                            self.source_objects.insert(kind, result.objects);
                        }
                        self.model.set_source_read(kind, result.read);
                    }
                }
                None
            }
        }
    }

    fn rebuild_model(&mut self) {
        let reads = std::mem::take(&mut self.model.source_reads);
        let values = |kind: SourceKind| {
            self.source_objects
                .get(&kind)
                .map_or(&[][..], Vec::as_slice)
        };
        let profiles = [
            values(SourceKind::ClusterProfile),
            values(SourceKind::Profile),
        ]
        .concat();
        let clusters = [
            values(SourceKind::SveltosCluster),
            values(SourceKind::CapiCluster),
        ]
        .concat();
        let mut model =
            SveltosModel::build(&profiles, values(SourceKind::ClusterSummary), &clusters);
        model.source_reads = reads;
        if model
            .source_read(SourceKind::ClusterSummary)
            .is_none_or(|read| read.availability != ReadAvailability::Available)
        {
            for deployment in &mut model.deployments {
                if deployment.bucket == ProvisioningBucket::MissingSummary {
                    deployment.bucket = ProvisioningBucket::Unknown;
                    deployment.diagnostic.get_or_insert_with(|| {
                        "ClusterSummary evidence is unreadable or incomplete".into()
                    });
                }
            }
        }
        self.inventory = evidence::inventory_evidence(values(SourceKind::ClusterConfiguration));
        self.model = model;
    }

    fn rebuild_warning(&mut self) {
        let problems = SOURCE_APIS
            .iter()
            .filter_map(|(kind, _, _)| {
                let read = self.model.source_read(*kind)?;
                let name = source_name(*kind);
                let error = read.error.as_deref().unwrap_or("read incomplete");
                match (read.availability, read.freshness) {
                    (ReadAvailability::Available, Freshness::Stale) => Some(format!(
                        "{name} stale since {}: {error}",
                        read.captured_at.as_deref().unwrap_or("unknown")
                    )),
                    (ReadAvailability::Incomplete, _) => {
                        Some(format!("{name} incomplete: {error}"))
                    }
                    (ReadAvailability::Failed, _) => Some(format!("{name} unreadable: {error}")),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        self.warning = (!problems.is_empty()).then(|| problems.join("; "));
    }

    fn frames(&self) -> impl Iterator<Item = &PageFrame> {
        self.pages.iter().chain(&self.inactive_tab)
    }

    fn anchors(&self) -> Vec<Option<Row>> {
        self.frames()
            .map(|frame| {
                self.filtered(frame)
                    .into_iter()
                    .nth(frame.selected)
                    .map(|(row, _)| row)
            })
            .collect()
    }

    /// Follows open deployment pages and every frame's selection to the same
    /// identities after a rebuild, clamping when an identity is gone.
    fn restore(&mut self, anchors: Vec<Option<Row>>) {
        let followed = self
            .pages
            .iter()
            .map(|frame| match &frame.page {
                Page::Deployment(key) => self.deployment(key).map(DeploymentKey::of),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (frame, key) in self.pages.iter_mut().zip(followed) {
            if let Some(key) = key {
                frame.page = Page::Deployment(key);
            }
        }
        let selections = self
            .frames()
            .zip(anchors)
            .map(|(frame, anchor)| {
                let rows = self.filtered(frame);
                anchor
                    .and_then(|anchor| find_row(&rows, &anchor))
                    .unwrap_or(frame.selected)
                    .min(rows.len().saturating_sub(1))
            })
            .collect::<Vec<_>>();
        for (frame, selected) in self
            .pages
            .iter_mut()
            .chain(&mut self.inactive_tab)
            .zip(selections)
        {
            frame.selected = selected;
        }
    }
}

fn find_row(rows: &[Entry], anchor: &Row) -> Option<usize> {
    if let Some(index) = rows.iter().position(|(row, _)| row == anchor) {
        return Some(index);
    }
    let Row::Deployment(key) = anchor else {
        return None;
    };
    let mut similar = rows
        .iter()
        .enumerate()
        .filter(|(_, (row, _))| matches!(row, Row::Deployment(other) if key.same_target(other)));
    let (index, _) = similar.next()?;
    similar.next().is_none().then_some(index)
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    match value.pointer(pointer)? {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn suspended(object: &Value) -> bool {
    object
        .pointer("/status/reconciliationSuspended")
        .and_then(Value::as_bool)
        == Some(true)
}

fn is_unresolved(deployment: &DeploymentEvidence) -> bool {
    deployment.relation == TargetRelation::Unresolved
        || deployment.profile.is_none()
        || !deployment
            .cluster
            .as_ref()
            .is_some_and(ClusterKey::is_well_formed)
}

fn cluster_type(key: &ClusterKey) -> &str {
    match (key.api_group.as_str(), key.kind.as_str()) {
        (evidence::CAPI_CLUSTER_GROUP, "Cluster") => "CAPI Cluster",
        (_, "") => "unknown type",
        (_, kind) => kind,
    }
}

fn cluster_name(key: &ClusterKey) -> String {
    format!("{} {}/{}", cluster_type(key), key.namespace, key.name)
}

fn profile_name(key: &ProfileKey) -> String {
    match &key.namespace {
        Some(namespace) => format!("{} {namespace}/{}", key.kind, key.name),
        None => format!("{} {}", key.kind, key.name),
    }
}

fn counts_label(counts: &ProvisioningCounts) -> String {
    let parts = [
        (counts.provisioned, "provisioned"),
        (counts.failed, "failed"),
        (counts.progressing, "progressing"),
        (counts.removing, "removing"),
        (counts.blocked, "blocked"),
        (counts.missing_summary, "missing summary"),
        (counts.no_features_configured, "no features configured"),
        (counts.dry_run, "DryRun (not applied)"),
        (counts.unknown, "unknown"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, label)| format!("{count} {label}"))
    .collect::<Vec<_>>();
    if parts.is_empty() {
        "none".into()
    } else {
        parts.join(" · ")
    }
}

fn sync_label(profile: &evidence::ProfileEvidence) -> String {
    match (
        profile
            .raw
            .pointer("/spec/syncMode")
            .and_then(Value::as_str),
        profile.sync_mode,
    ) {
        (None, _) => "not set (default Continuous)".into(),
        (Some(_), SyncMode::OneTime) => "OneTime (apply-once)".into(),
        (Some(_), SyncMode::DryRun) => "DryRun (not applied)".into(),
        (Some(raw), _) => raw.into(),
    }
}

fn rollout_summary(profile: &evidence::ProfileEvidence) -> String {
    let count = |clusters: &Option<Vec<ClusterKey>>| {
        clusters
            .as_ref()
            .map_or_else(|| "unknown".into(), |clusters| clusters.len().to_string())
    };
    let mut label = format!(
        "updated {} · updating {}",
        count(&profile.updated_clusters),
        count(&profile.updating_clusters)
    );
    if let (Some(updated), Some(updating)) = (&profile.updated_clusters, &profile.updating_clusters)
        && updated.iter().any(|cluster| updating.contains(cluster))
    {
        label.push_str(" · inconsistent");
    }
    if profile.sync_mode == SyncMode::DryRun {
        label.push_str(" · not applied");
    }
    label
}

fn deployment_cells(deployment: &DeploymentEvidence) -> Vec<String> {
    let profile = match (&deployment.profile, &deployment.reported_owner) {
        (Some(key), _) => profile_name(key),
        (None, Some(owner)) => format!("{} (unresolved)", profile_name(owner)),
        (None, None) => "unresolved".into(),
    };
    let cluster = match &deployment.cluster {
        Some(key) if key.is_well_formed() => cluster_name(key),
        Some(key) => format!("{} (unresolved)", cluster_name(key)),
        None => "unresolved".into(),
    };
    let rollout = match (
        deployment.rollout_conflict,
        deployment.rollout_updated,
        deployment.rollout_updating,
    ) {
        (true, _, _) => "inconsistent",
        (_, Some(true), _) => "updated",
        (_, _, Some(true)) => "updating",
        (_, Some(false), Some(false)) => "not updated",
        _ => "unknown",
    };
    let provisioning = match deployment.bucket {
        ProvisioningBucket::Provisioned => "provisioned",
        ProvisioningBucket::Failed => "failed",
        ProvisioningBucket::Progressing => "progressing",
        ProvisioningBucket::Removing => "removing",
        ProvisioningBucket::Blocked => "blocked",
        ProvisioningBucket::MissingSummary => "missing summary",
        ProvisioningBucket::NoFeaturesConfigured => "no features configured",
        ProvisioningBucket::DryRun => "DryRun (not applied)",
        ProvisioningBucket::Unknown => "unknown",
    };
    let flags = [
        match deployment.relation {
            TargetRelation::Observed => None,
            TargetRelation::ReportedFailure => Some("reported failure"),
            TargetRelation::SummaryWithoutObservedTarget => Some("not in matchingClusters"),
            TargetRelation::RemovingAfterMatch => Some("summary removing"),
            TargetRelation::Unresolved => Some("unresolved"),
        },
        deployment.helm_conflict.then_some("helm conflict"),
        (deployment.sync_mode == SyncMode::OneTime).then_some("apply-once"),
        (deployment.sync_mode == SyncMode::DryRun).then_some("not applied"),
        deployment
            .summary
            .as_ref()
            .is_some_and(suspended)
            .then_some("suspended"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    vec![profile, cluster, rollout.into(), provisioning.into(), flags]
}

fn relation_label(relation: TargetRelation) -> &'static str {
    match relation {
        TargetRelation::Observed => "observed in profile matchingClusters",
        TargetRelation::ReportedFailure => "profile reports a failure for this target",
        TargetRelation::SummaryWithoutObservedTarget => {
            "summary exists, target not in profile matchingClusters"
        }
        TargetRelation::RemovingAfterMatch => "summary is being removed (no longer matching)",
        TargetRelation::Unresolved => "unresolved; nothing is inferred",
    }
}

fn source_label(source: Source) -> &'static str {
    match source {
        Source::Profile => "profile",
        Source::Cluster => "cluster",
        Source::Summary => "ClusterSummary",
        Source::Configuration => "ClusterConfiguration",
    }
}

fn source_name(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::SveltosCluster => "SveltosCluster",
        SourceKind::CapiCluster => "CAPI Cluster",
        SourceKind::ClusterProfile => "ClusterProfile",
        SourceKind::Profile => "Profile",
        SourceKind::ClusterSummary => "ClusterSummary",
        SourceKind::ClusterConfiguration => "ClusterConfiguration",
    }
}

fn object_name(object: &Value) -> String {
    let kind = text(object, "/kind").unwrap_or_default();
    let name = text(object, "/metadata/name").unwrap_or_default();
    match text(object, "/metadata/namespace") {
        Some(namespace) => format!("{kind} {namespace}/{name}"),
        None => format!("{kind} {name}"),
    }
}

fn feature_entries(feature: &Value) -> usize {
    ["resources", "charts"]
        .into_iter()
        .filter_map(|field| feature.get(field).and_then(Value::as_array))
        .map(Vec::len)
        .sum()
}

fn inventory_labels(section: &evidence::InventorySection) -> Vec<String> {
    let mut labels = Vec::new();
    for feature in &section.features {
        let id = text(feature, "/featureID").unwrap_or_else(|| "<missing featureID>".into());
        let entries = |field: &str| {
            feature
                .get(field)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        };
        for resource in entries("resources") {
            let field = |name: &str| text(resource, &format!("/{name}"));
            labels.push(format!(
                "reported: {id} · {} · {}/{} {} {}{}{}",
                field("deploymentType").unwrap_or_else(|| "destination not reported".into()),
                field("group").unwrap_or_else(|| "core".into()),
                field("version").unwrap_or_default(),
                field("kind").unwrap_or_default(),
                field("namespace").map_or_else(String::new, |namespace| format!("{namespace}/")),
                field("name").unwrap_or_default(),
                field("lastAppliedTime")
                    .map_or_else(String::new, |time| format!(" · applied {time}"))
            ));
        }
        for chart in entries("charts") {
            let field = |name: &str| text(chart, &format!("/{name}"));
            labels.push(format!(
                "reported: {id} · chart {} in {} · {} · {}{}",
                field("releaseName").unwrap_or_default(),
                field("namespace").unwrap_or_else(|| "?".into()),
                field("chartVersion").unwrap_or_else(|| "version not reported".into()),
                field("repoURL").unwrap_or_else(|| "repository not reported".into()),
                field("lastAppliedTime")
                    .map_or_else(String::new, |time| format!(" · applied {time}"))
            ));
        }
    }
    labels
}

fn unresolved_source(kind: SourceKind, discovery_failed: bool) -> SourceResult {
    SourceResult {
        kind,
        objects: Vec::new(),
        read: if discovery_failed {
            SourceRead::failed("API discovery failed for source group")
        } else {
            SourceRead::absent()
        },
    }
}

#[derive(Debug)]
pub struct SourceResult {
    pub(crate) kind: SourceKind,
    pub(crate) objects: Vec<Value>,
    pub(crate) read: SourceRead,
}

impl App {
    pub fn open_sveltos(&mut self) {
        let has_config_api = ["ClusterProfile", "Profile", "ClusterSummary"]
            .iter()
            .any(|kind| {
                self.cluster
                    .resolve_in_group(kind, evidence::CONFIG_GROUP)
                    .is_some()
            });
        let discovery_uncertain = self
            .cluster
            .discovery_warnings
            .iter()
            .any(|warning| warning.contains(evidence::CONFIG_GROUP));
        if !has_config_api && !discovery_uncertain {
            self.flash_warn("Sveltos APIs unavailable (config.projectsveltos.io)");
            return;
        }
        self.bump_generation();
        self.sveltos.open();
        self.mode = Mode::Sveltos;
        self.spawn_sveltos_gather();
    }

    fn spawn_sveltos_gather(&mut self) {
        self.sveltos_request = self.sveltos_request.wrapping_add(1);
        let request = self.sveltos_request;
        let generation = self.generation;
        self.sveltos.refreshing = true;
        let client = self.cluster.client.clone();
        let tx = self.tx.clone();
        let kinds = SOURCE_APIS
            .into_iter()
            .map(|(source, name, group)| {
                (
                    source,
                    self.cluster
                        .resolve_in_group(name, group)
                        .map(|kind| kind.ar),
                    self.cluster
                        .discovery_warnings
                        .iter()
                        .any(|warning| warning.contains(group)),
                )
            })
            .collect::<Vec<_>>();
        let previous = self.sveltos_task.take();
        if let Some(task) = &previous {
            task.abort();
        }
        self.sveltos_task = Some(tokio::spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
            let requests = Arc::new(AtomicUsize::new(0));
            let objects = Arc::new(AtomicUsize::new(0));
            let deadline = Instant::now() + GATHER_TIMEOUT;
            let mut tasks = JoinSet::new();
            for (source, kind, discovery_failed) in kinds {
                let client = client.clone();
                let semaphore = semaphore.clone();
                let requests = requests.clone();
                let objects = objects.clone();
                tasks.spawn(async move {
                    match kind {
                        Some(resource) => {
                            gather_source(
                                client, resource, source, semaphore, requests, objects, deadline,
                            )
                            .await
                        }
                        None => unresolved_source(source, discovery_failed),
                    }
                });
            }
            let mut reads = Vec::with_capacity(6);
            while let Some(result) = tasks.join_next().await {
                if let Ok(result) = result {
                    reads.push(result);
                }
            }
            let _ = tx
                .send(Msg::SveltosData {
                    generation,
                    request,
                    reads,
                })
                .await;
            tokio::time::sleep(POLL_INTERVAL).await;
            let _ = tx
                .send(Msg::SveltosPoll {
                    generation,
                    request,
                })
                .await;
        }));
    }

    pub(super) fn key_sveltos(&mut self, key: KeyInput) {
        if self.sveltos.filter_editing {
            self.key_sveltos_filter(key);
            return;
        }
        let detail = self.sveltos.on_deployment();
        match key.action {
            Some(Action::Back) if self.sveltos.pages.len() > 1 => {
                self.sveltos.pages.pop();
                self.sveltos.detail_scroll = 0;
            }
            Some(Action::Back | Action::Close) => self.close_sveltos(),
            Some(Action::NextView | Action::PreviousView) => self.sveltos.switch_tab(),
            Some(Action::Refresh) => self.spawn_sveltos_gather(),
            Some(Action::Filter) => {
                self.sveltos.filter_draft = self.sveltos.filter().to_owned();
                self.sveltos.filter_editing = true;
            }
            Some(Action::PageDown) if detail => {
                self.sveltos.detail_scroll = self.sveltos.detail_scroll.saturating_add(10);
            }
            Some(Action::PageUp) if detail => {
                self.sveltos.detail_scroll = self.sveltos.detail_scroll.saturating_sub(10);
            }
            Some(Action::Down) => self.sveltos.step(1),
            Some(Action::Up) => self.sveltos.step(-1),
            Some(Action::PageDown) => self.sveltos.step(10),
            Some(Action::PageUp) => self.sveltos.step(-10),
            Some(Action::First) => self.sveltos.step(isize::MIN),
            Some(Action::Last) => self.sveltos.step(isize::MAX),
            Some(Action::Accept) => self.accept_sveltos_row(),
            Some(Action::Yaml) => self.sveltos_yaml(),
            _ => {}
        }
    }

    fn key_sveltos_filter(&mut self, key: KeyInput) {
        let state = &mut self.sveltos;
        match (key.action, key.code) {
            (Some(Action::Accept), _) => {
                state.filter_editing = false;
                let filter = std::mem::take(&mut state.filter_draft);
                if let Some(frame) = state.pages.last_mut() {
                    frame.filter = filter;
                    frame.selected = 0;
                }
            }
            (Some(Action::Back), _) => state.filter_editing = false,
            (Some(Action::ClearLine), _) => state.filter_draft.clear(),
            (Some(Action::Backspace), _) => {
                state.filter_draft.pop();
            }
            (Some(Action::DeleteWord), _) => {
                state.filter_draft = state
                    .filter_draft
                    .trim_end()
                    .rsplit_once(char::is_whitespace)
                    .map_or(String::new(), |(head, _)| head.to_owned());
            }
            (None, KeyCode::Char(character)) => state.filter_draft.push(character),
            _ => {}
        }
    }

    fn accept_sveltos_row(&mut self) {
        let page = match self.sveltos.selected_row() {
            Some(Row::Cluster(key)) => Page::Cluster(key),
            Some(Row::Profile(key)) => Page::Profile(key),
            Some(Row::Unresolved) => Page::Unresolved,
            Some(Row::Deployment(key)) => Page::Deployment(key),
            Some(Row::Source(_)) => {
                self.sveltos_yaml();
                return;
            }
            Some(Row::Inventory(_)) => {
                self.flash_warn(INVENTORY_ONLY);
                return;
            }
            None => return,
        };
        self.sveltos.pages.push(PageFrame::new(page));
        self.sveltos.detail_scroll = 0;
    }

    fn sveltos_yaml(&mut self) {
        let object = self.sveltos.selected_object().and_then(|value| {
            serde_json::from_value::<DynamicObject>(value)
                .map_err(|error| format!("source object is malformed: {error}"))
        });
        match object {
            Ok(object) => {
                self.set_return_mode();
                self.show_yaml(&object);
                // A snapshot of the evidence as read: no live re-read, no edit.
                self.clear_document_source();
            }
            Err(message) => self.flash_warn(&message),
        }
    }

    fn close_sveltos(&mut self) {
        self.sveltos = State::default();
        self.mode = Mode::Table;
        self.bump_generation();
        self.start_watch();
    }

    pub(super) fn receive_sveltos(
        &mut self,
        generation: u64,
        request: u64,
        reads: Vec<SourceResult>,
    ) {
        if generation != self.generation || request != self.sveltos_request {
            return;
        }
        let state = &mut self.sveltos;
        state.refreshing = false;
        let anchors = state.anchors();
        for result in reads {
            if let Some(captured_at) = state.apply_source_result(result) {
                state.last_success = Some(captured_at);
            }
        }
        state.rebuild_model();
        state.rebuild_warning();
        state.restore(anchors);
    }

    pub(super) fn receive_sveltos_poll(&mut self, generation: u64, request: u64) {
        if generation != self.generation
            || request != self.sveltos_request
            || self.sveltos.refreshing
        {
            return;
        }
        if self.sveltos_reachable() {
            self.spawn_sveltos_gather();
            return;
        }
        // Another view covers the dashboard but may still return to it: keep
        // the schedule alive without reading.
        let tx = self.tx.clone();
        self.sveltos_task = Some(tokio::spawn(async move {
            tokio::time::sleep(POLL_INTERVAL).await;
            let _ = tx
                .send(Msg::SveltosPoll {
                    generation,
                    request,
                })
                .await;
        }));
    }

    /// The dashboard is on screen, or under overlays that return to it.
    fn sveltos_reachable(&self) -> bool {
        let mut mode = self.mode;
        for _ in 0..4 {
            mode = match mode {
                Mode::Sveltos => return true,
                Mode::Command => self.palette_return,
                Mode::Help => self.help_return,
                Mode::DocFilter => self.doc_filter_return,
                Mode::Detail => self.return_mode,
                _ => return false,
            };
        }
        false
    }
}

async fn gather_source(
    client: kube::Client,
    resource: kube::api::ApiResource,
    source: SourceKind,
    semaphore: Arc<Semaphore>,
    requests: Arc<AtomicUsize>,
    object_count: Arc<AtomicUsize>,
    deadline: Instant,
) -> SourceResult {
    let api: Api<DynamicObject> = Api::all_with(client, &resource);
    let mut objects = Vec::new();
    let mut token = None;
    loop {
        if Instant::now() >= deadline {
            return SourceResult {
                kind: source,
                objects,
                read: SourceRead::incomplete("30s gather deadline reached"),
            };
        }
        let request_number = requests.fetch_add(1, Ordering::Relaxed);
        if request_number >= MAX_LIST_REQUESTS {
            return SourceResult {
                kind: source,
                objects,
                read: SourceRead::incomplete("200 list-request limit reached"),
            };
        }
        let permit =
            match tokio::time::timeout_at(deadline, semaphore.clone().acquire_owned()).await {
                Ok(Ok(permit)) => permit,
                Ok(Err(_)) => {
                    return SourceResult {
                        kind: source,
                        objects,
                        read: SourceRead::incomplete("gather cancelled"),
                    };
                }
                Err(_) => {
                    return SourceResult {
                        kind: source,
                        objects,
                        read: SourceRead::incomplete("30s gather deadline reached"),
                    };
                }
            };
        let mut params = kube::api::ListParams::default().limit(PAGE_SIZE);
        if let Some(token) = token.as_deref() {
            params = params.continue_token(token);
        }
        let response = tokio::time::timeout_at(
            deadline,
            tokio::time::timeout(REQUEST_TIMEOUT, api.list(&params)),
        )
        .await;
        drop(permit);
        let list = match response {
            Ok(Ok(Ok(list))) => list,
            Ok(Ok(Err(error))) => {
                return SourceResult {
                    kind: source,
                    objects,
                    read: SourceRead::failed(error.to_string()),
                };
            }
            Ok(Err(_)) => {
                return SourceResult {
                    kind: source,
                    objects,
                    read: SourceRead::incomplete("5s request timeout"),
                };
            }
            Err(_) => {
                return SourceResult {
                    kind: source,
                    objects,
                    read: SourceRead::incomplete("30s gather deadline reached"),
                };
            }
        };
        let next = list.metadata.continue_.clone();
        let requested_objects = list.items.len();
        let previous = object_count.fetch_add(requested_objects, Ordering::Relaxed);
        let remaining = MAX_OBJECTS.saturating_sub(previous);
        if remaining == 0
            && (requested_objects > 0 || next.as_ref().is_some_and(|token| !token.is_empty()))
        {
            return SourceResult {
                kind: source,
                objects,
                read: SourceRead::incomplete("20,000 object limit reached"),
            };
        }
        let truncated = requested_objects > remaining;
        let page_objects = list.items.into_iter().take(remaining).collect::<Vec<_>>();
        objects.extend(page_objects.into_iter().filter_map(|mut object| {
            object.types.get_or_insert_with(|| TypeMeta {
                api_version: resource.api_version.clone(),
                kind: resource.kind.clone(),
            });
            serde_json::to_value(object).ok()
        }));
        if truncated
            || (next.as_ref().is_some_and(|token| !token.is_empty())
                && object_count.load(Ordering::Relaxed) >= MAX_OBJECTS)
        {
            return SourceResult {
                kind: source,
                objects,
                read: SourceRead::incomplete("20,000 object limit reached"),
            };
        }
        match next.filter(|token| !token.is_empty()) {
            Some(next) => token = Some(next),
            None => {
                return SourceResult {
                    kind: source,
                    objects,
                    read: SourceRead::available(k8s_openapi::jiff::Timestamp::now().to_string()),
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::TcpListener,
    };

    #[test]
    fn gather_allowlist_and_budgets_are_fixed() {
        assert_eq!(
            SOURCE_APIS.map(|(source, name, group)| (source, name, group)),
            [
                (
                    SourceKind::SveltosCluster,
                    "SveltosCluster",
                    evidence::SVELTOS_CLUSTER_GROUP
                ),
                (
                    SourceKind::CapiCluster,
                    "Cluster",
                    evidence::CAPI_CLUSTER_GROUP
                ),
                (
                    SourceKind::ClusterProfile,
                    "ClusterProfile",
                    evidence::CONFIG_GROUP
                ),
                (SourceKind::Profile, "Profile", evidence::CONFIG_GROUP),
                (
                    SourceKind::ClusterSummary,
                    "ClusterSummary",
                    evidence::CONFIG_GROUP
                ),
                (
                    SourceKind::ClusterConfiguration,
                    "ClusterConfiguration",
                    evidence::CONFIG_GROUP
                ),
            ]
        );
        assert_eq!(PAGE_SIZE, 500);
        assert_eq!(MAX_CONCURRENT_REQUESTS, 4);
        assert_eq!(MAX_LIST_REQUESTS, 200);
        assert_eq!(MAX_OBJECTS, 20_000);
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(5));
        assert_eq!(GATHER_TIMEOUT, Duration::from_secs(30));
        assert_eq!(POLL_INTERVAL, Duration::from_secs(5));
    }

    #[test]
    fn confirmed_absence_differs_from_discovery_failure() {
        assert_eq!(
            unresolved_source(SourceKind::Profile, false)
                .read
                .availability,
            ReadAvailability::Absent
        );
        assert_eq!(
            unresolved_source(SourceKind::Profile, true)
                .read
                .availability,
            ReadAvailability::Failed
        );
    }

    #[test]
    fn partial_failure_keeps_previous_source_as_stale() {
        let mut state = State::default();
        let previous = json!({"metadata":{"name":"previous"}});
        let partial = json!({"metadata":{"name":"partial"}});

        state.apply_source_result(SourceResult {
            kind: SourceKind::Profile,
            objects: vec![previous.clone()],
            read: SourceRead::available("before"),
        });
        state.apply_source_result(SourceResult {
            kind: SourceKind::Profile,
            objects: vec![partial],
            read: SourceRead::failed("second page denied"),
        });
        state.rebuild_warning();

        assert_eq!(
            state.source_objects.get(&SourceKind::Profile),
            Some(&vec![previous])
        );
        let read = state.model.source_read(SourceKind::Profile).unwrap();
        assert_eq!(read.availability, ReadAvailability::Available);
        assert_eq!(read.freshness, evidence::Freshness::Stale);
        assert_eq!(read.error.as_deref(), Some("second page denied"));
        assert_eq!(
            state.warning.as_deref(),
            Some("Profile stale since before: second page denied")
        );
    }

    #[test]
    fn first_partial_read_retains_partial_evidence() {
        let mut state = State::default();
        let partial = json!({"metadata":{"name":"first-page"}});

        state.apply_source_result(SourceResult {
            kind: SourceKind::ClusterSummary,
            objects: vec![partial.clone()],
            read: SourceRead::incomplete("request limit reached"),
        });

        assert_eq!(
            state.source_objects.get(&SourceKind::ClusterSummary),
            Some(&vec![partial])
        );
        assert_eq!(
            state
                .model
                .source_read(SourceKind::ClusterSummary)
                .unwrap()
                .availability,
            ReadAvailability::Incomplete
        );
    }

    #[tokio::test]
    async fn mock_gather_lists_only_allowlisted_sources_and_paginates() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let paths = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum_active = Arc::new(AtomicUsize::new(0));
        let server_paths = paths.clone();
        let server_active = active.clone();
        let server_maximum = maximum_active.clone();
        let server = tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                let paths = server_paths.clone();
                let active = server_active.clone();
                let maximum_active = server_maximum.clone();
                tokio::spawn(async move {
                    let (read, mut write) = socket.into_split();
                    let mut reader = BufReader::new(read);
                    let mut request = String::new();
                    if reader.read_line(&mut request).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let path = request.split_whitespace().nth(1).unwrap_or("").to_owned();
                    loop {
                        let mut header = String::new();
                        if reader.read_line(&mut header).await.unwrap_or(0) == 0 || header == "\r\n"
                        {
                            break;
                        }
                    }
                    paths.lock().unwrap().push(path.clone());
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum_active.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(20)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    let next = path.contains("/profiles") && !path.contains("continue=next");
                    let body = json!({
                        "apiVersion":"v1",
                        "kind":"List",
                        "metadata":{"continue":if next {"next"} else {""}},
                        "items":[{
                            "apiVersion":"config.projectsveltos.io/v1beta1",
                            "kind":"Profile",
                            "metadata":{"name":"fixture"}
                        }]
                    })
                    .to_string();
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = write.write_all(response.as_bytes()).await;
                });
            }
        });
        let mut config = kube::Config::new(format!("http://{address}").parse().unwrap());
        config.default_retry = false;
        let client = kube::Client::try_from(config).unwrap();
        let plural = [
            "sveltosclusters",
            "clusters",
            "clusterprofiles",
            "profiles",
            "clustersummaries",
            "clusterconfigurations",
        ];
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
        let requests = Arc::new(AtomicUsize::new(0));
        let objects = Arc::new(AtomicUsize::new(0));
        let deadline = Instant::now() + GATHER_TIMEOUT;
        let mut tasks = JoinSet::new();
        for ((source, name, group), plural) in SOURCE_APIS.into_iter().zip(plural) {
            let resource = kube::discovery::ApiResource {
                group: group.into(),
                version: "v1beta1".into(),
                api_version: format!("{group}/v1beta1"),
                kind: name.into(),
                plural: plural.into(),
            };
            tasks.spawn(gather_source(
                client.clone(),
                resource,
                source,
                semaphore.clone(),
                requests.clone(),
                objects.clone(),
                deadline,
            ));
        }
        let mut reads = Vec::new();
        while let Some(result) = tasks.join_next().await {
            reads.push(result.unwrap());
        }
        server.abort();

        assert!(
            reads
                .iter()
                .all(|read| read.read.availability == ReadAvailability::Available)
        );
        assert_eq!(
            reads
                .iter()
                .find(|read| read.kind == SourceKind::Profile)
                .unwrap()
                .objects
                .len(),
            2
        );
        let mut paths = paths.lock().unwrap().clone();
        paths.sort();
        assert_eq!(paths.len(), 7);
        assert!(paths.iter().all(|path| {
            path.starts_with("/apis/")
                && path.contains("limit=500")
                && !path.contains("/namespaces/")
                && SOURCE_APIS
                    .iter()
                    .any(|(_, _, group)| path.starts_with(&format!("/apis/{group}/")))
        }));
        for plural in plural {
            assert!(
                paths
                    .iter()
                    .any(|path| path.contains(&format!("/{plural}?")))
            );
        }
        assert_eq!(
            maximum_active.load(Ordering::SeqCst),
            MAX_CONCURRENT_REQUESTS
        );
    }

    #[tokio::test]
    async fn stale_gather_results_are_ignored() {
        let (tx, _rx) = tokio::sync::mpsc::channel(8);
        let mut app = App::new(crate::k8s::Cluster::fake(), tx);
        app.sveltos.open();
        app.mode = Mode::Sveltos;
        let generation = app.generation;
        let request = app.sveltos_request;

        app.receive_sveltos(
            generation.wrapping_sub(1),
            request,
            vec![SourceResult {
                kind: SourceKind::Profile,
                objects: vec![json!({"metadata":{"name":"stale"}})],
                read: SourceRead::available("stale"),
            }],
        );
        app.receive_sveltos(
            generation,
            request.wrapping_sub(1),
            vec![SourceResult {
                kind: SourceKind::Profile,
                objects: vec![json!({"metadata":{"name":"stale"}})],
                read: SourceRead::available("stale"),
            }],
        );
        assert!(app.sveltos.refreshing);
        assert!(app.sveltos.source_objects.is_empty());
    }

    #[tokio::test]
    async fn gather_stops_at_each_budget_and_keeps_partial_pages() {
        // Page one has two Profiles and a continue token; page two has one more.
        fn client(page_two_delay: Duration, page_two_denied: bool) -> kube::Client {
            kube::Client::new(
                tower::service_fn(move |req: http::Request<kube::client::Body>| async move {
                    let page_two = req
                        .uri()
                        .query()
                        .is_some_and(|query| query.contains("continue=next"));
                    if page_two {
                        tokio::time::sleep(page_two_delay).await;
                    }
                    let (status, body) = if page_two && page_two_denied {
                        (
                            403,
                            json!({"apiVersion":"v1","kind":"Status","status":"Failure","reason":"Forbidden","message":"profiles denied","code":403}),
                        )
                    } else {
                        let names: &[&str] = if page_two { &["c"] } else { &["a", "b"] };
                        (
                            200,
                            json!({
                                "apiVersion":"v1",
                                "kind":"List",
                                "metadata":{"continue":if page_two {""} else {"next"}},
                                "items":names.iter().map(|name| json!({"metadata":{"name":name}})).collect::<Vec<_>>()
                            }),
                        )
                    };
                    Ok::<_, std::convert::Infallible>(
                        http::Response::builder()
                            .status(status)
                            .body(http_body_util::Full::new(hyper::body::Bytes::from(
                                body.to_string(),
                            )))
                            .unwrap(),
                    )
                }),
                "default",
            )
        }
        async fn gather(
            client: kube::Client,
            requests: usize,
            objects: usize,
            budget: Duration,
        ) -> (ReadAvailability, Option<String>, usize) {
            let resource = kube::discovery::ApiResource {
                group: "config.projectsveltos.io".into(),
                version: "v1beta1".into(),
                api_version: "config.projectsveltos.io/v1beta1".into(),
                kind: "Profile".into(),
                plural: "profiles".into(),
            };
            let result = gather_source(
                client,
                resource,
                SourceKind::Profile,
                Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
                Arc::new(AtomicUsize::new(requests)),
                Arc::new(AtomicUsize::new(objects)),
                Instant::now() + budget,
            )
            .await;
            (
                result.read.availability,
                result.read.error,
                result.objects.len(),
            )
        }
        let fast = || client(Duration::ZERO, false);
        let incomplete = |error: &str, objects| {
            (
                ReadAvailability::Incomplete,
                Some(error.to_owned()),
                objects,
            )
        };

        assert_eq!(
            gather(fast(), 0, 0, GATHER_TIMEOUT).await,
            (ReadAvailability::Available, None, 3)
        );
        assert_eq!(
            gather(fast(), 0, MAX_OBJECTS - 1, GATHER_TIMEOUT).await,
            incomplete("20,000 object limit reached", 1)
        );
        assert_eq!(
            gather(fast(), MAX_LIST_REQUESTS - 1, 0, GATHER_TIMEOUT).await,
            incomplete("200 list-request limit reached", 2)
        );
        let slow = client(Duration::from_secs(2), false);
        assert_eq!(
            gather(slow, 0, 0, Duration::from_millis(300)).await,
            incomplete("30s gather deadline reached", 2)
        );
        let stalled = client(Duration::from_secs(6), false);
        assert_eq!(
            gather(stalled, 0, 0, GATHER_TIMEOUT).await,
            incomplete("5s request timeout", 2)
        );
        let (availability, error, objects) =
            gather(client(Duration::ZERO, true), 0, 0, GATHER_TIMEOUT).await;
        assert_eq!((availability, objects), (ReadAvailability::Failed, 2));
        assert!(
            error
                .as_deref()
                .is_some_and(|error| error.contains("profiles denied")),
            "{error:?}"
        );
    }
}
