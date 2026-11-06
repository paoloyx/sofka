use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const CONFIG_GROUP: &str = "config.projectsveltos.io";
pub const SVELTOS_CLUSTER_GROUP: &str = "lib.projectsveltos.io";
pub const CAPI_CLUSTER_GROUP: &str = "cluster.x-k8s.io";
pub const CLUSTER_NAME_LABEL: &str = "projectsveltos.io/cluster-name";
pub const CLUSTER_TYPE_LABEL: &str = "projectsveltos.io/cluster-type";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadAvailability {
    Available,
    Absent,
    Incomplete,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SourceKind {
    SveltosCluster,
    CapiCluster,
    ClusterProfile,
    Profile,
    ClusterSummary,
    ClusterConfiguration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceRead {
    pub availability: ReadAvailability,
    pub freshness: Freshness,
    pub captured_at: Option<String>,
    pub error: Option<String>,
}

impl SourceRead {
    pub fn available(captured_at: impl Into<String>) -> Self {
        Self {
            availability: ReadAvailability::Available,
            freshness: Freshness::Fresh,
            captured_at: Some(captured_at.into()),
            error: None,
        }
    }

    pub fn absent() -> Self {
        Self {
            availability: ReadAvailability::Absent,
            freshness: Freshness::Unknown,
            captured_at: None,
            error: None,
        }
    }

    pub fn incomplete(error: impl Into<String>) -> Self {
        Self {
            availability: ReadAvailability::Incomplete,
            freshness: Freshness::Unknown,
            captured_at: None,
            error: Some(error.into()),
        }
    }

    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            availability: ReadAvailability::Failed,
            freshness: Freshness::Unknown,
            captured_at: None,
            error: Some(error.into()),
        }
    }

    pub fn stale(mut self, error: impl Into<String>) -> Self {
        self.freshness = Freshness::Stale;
        self.error = Some(error.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClusterKey {
    pub api_group: String,
    pub kind: String,
    pub namespace: String,
    pub name: String,
}

impl ClusterKey {
    pub fn from_object(object: &Value) -> Option<Self> {
        let kind = string_at(object, &["kind"])?;
        let group = api_group(object)?;
        if !matches!(
            (group.as_str(), kind.as_str()),
            (SVELTOS_CLUSTER_GROUP, "SveltosCluster") | (CAPI_CLUSTER_GROUP, "Cluster")
        ) {
            return None;
        }
        let metadata = object.get("metadata")?;
        let key = Self {
            api_group: group,
            kind,
            namespace: string_at(metadata, &["namespace"])?,
            name: string_at(metadata, &["name"])?,
        };
        key.is_well_formed().then_some(key)
    }

    pub fn is_well_formed(&self) -> bool {
        !self.api_group.is_empty()
            && !self.kind.is_empty()
            && !self.namespace.is_empty()
            && !self.name.is_empty()
    }

    pub fn from_sveltos_type(cluster_type: &str, namespace: &str, name: &str) -> Self {
        let (api_group, kind) = match cluster_type {
            "Sveltos" => (SVELTOS_CLUSTER_GROUP, "SveltosCluster"),
            "Capi" => (CAPI_CLUSTER_GROUP, "Cluster"),
            _ => ("", cluster_type),
        };
        Self {
            api_group: api_group.to_owned(),
            kind: kind.to_owned(),
            namespace: namespace.to_owned(),
            name: name.to_owned(),
        }
    }

    pub fn from_configuration_labels(object: &Value) -> Option<Self> {
        let metadata = object.get("metadata")?;
        let labels = metadata.get("labels")?;
        let cluster_type = labels.get(CLUSTER_TYPE_LABEL)?.as_str()?;
        let namespace = metadata.get("namespace")?.as_str()?;
        let name = labels.get(CLUSTER_NAME_LABEL)?.as_str()?;
        Some(Self::from_sveltos_type(cluster_type, namespace, name))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProfileKey {
    pub api_group: String,
    pub kind: String,
    pub namespace: Option<String>,
    pub name: String,
}

impl ProfileKey {
    pub fn from_object(object: &Value) -> Option<Self> {
        let kind = string_at(object, &["kind"])?;
        let group = api_group(object)?;
        if group != CONFIG_GROUP || !matches!(kind.as_str(), "ClusterProfile" | "Profile") {
            return None;
        }
        let metadata = object.get("metadata")?;
        let name = string_at(metadata, &["name"])?;
        if name.is_empty() {
            return None;
        }
        let namespace = if kind == "ClusterProfile" && group == CONFIG_GROUP {
            None
        } else {
            Some(string_at(metadata, &["namespace"])?)
        };
        if namespace.as_deref() == Some("") {
            return None;
        }
        Some(Self {
            api_group: group,
            kind,
            namespace,
            name,
        })
    }

    fn from_owner_reference(owner: &Value, summary: &Value) -> Option<Self> {
        let kind = string_at(owner, &["kind"])?;
        let group = owner.get("apiVersion")?.as_str()?.split_once('/')?.0;
        if group != CONFIG_GROUP || !matches!(kind.as_str(), "Profile" | "ClusterProfile") {
            return None;
        }
        let namespace = match kind.as_str() {
            "ClusterProfile" => None,
            "Profile" => Some(string_at(summary, &["metadata", "namespace"])?),
            _ => return None,
        };
        let name = string_at(owner, &["name"])?;
        if name.is_empty() || namespace.as_deref() == Some("") {
            return None;
        }
        Some(Self {
            api_group: group.to_owned(),
            kind,
            namespace,
            name,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncMode {
    Continuous,
    OneTime,
    DryRun,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileEvidence {
    pub key: ProfileKey,
    pub uid: Option<String>,
    pub raw: Value,
    pub sync_mode: SyncMode,
    pub matching_clusters: Option<Vec<ClusterKey>>,
    pub failed_clusters: Option<Vec<ClusterKey>>,
    pub updated_clusters: Option<Vec<ClusterKey>>,
    pub updating_clusters: Option<Vec<ClusterKey>>,
}

impl ProfileEvidence {
    pub fn from_object(object: &Value) -> Option<Self> {
        let key = ProfileKey::from_object(object)?;
        let status = object.get("status");
        let spec = object.get("spec").unwrap_or(&Value::Null);
        Some(Self {
            key,
            uid: string_at(object, &["metadata", "uid"]),
            raw: object.clone(),
            sync_mode: sync_mode(string_at(spec, &["syncMode"]).as_deref()),
            matching_clusters: status.and_then(|s| cluster_refs(s.get("matchingClusters"))),
            failed_clusters: status.and_then(|s| failed_refs(s.get("failedClusters"))),
            updated_clusters: status.and_then(|s| rollout_refs(s.get("updatedClusters"))),
            updating_clusters: status.and_then(|s| rollout_refs(s.get("updatingClusters"))),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClusterEvidence {
    pub key: ClusterKey,
    pub uid: Option<String>,
    pub raw: Value,
    pub readiness: Option<String>,
    pub control_plane_initialized: Option<bool>,
    pub connectivity: Option<String>,
    pub paused: bool,
    pub deleting: bool,
}

impl ClusterEvidence {
    pub fn from_object(object: &Value) -> Option<Self> {
        let key = ClusterKey::from_object(object)?;
        let status = object.get("status").unwrap_or(&Value::Null);
        let conditions = status.get("conditions").and_then(Value::as_array);
        let ready = conditions
            .and_then(|items| {
                items
                    .iter()
                    .find(|condition| string_at(condition, &["type"]).as_deref() == Some("Ready"))
            })
            .and_then(|condition| string_at(condition, &["status"]));
        let control_plane_initialized = status
            .get("initialization")
            .and_then(|value| value.get("controlPlaneInitialized"))
            .and_then(Value::as_bool)
            .or_else(|| status.get("initialized").and_then(Value::as_bool));
        let readiness = ready.or_else(|| {
            if key.kind == "SveltosCluster" {
                status.get("ready").and_then(scalar_string)
            } else {
                None
            }
        });
        let connectivity = if key.kind == "SveltosCluster" {
            string_at(status, &["connectionStatus"])
        } else {
            None
        };
        let metadata = object.get("metadata").unwrap_or(&Value::Null);
        Some(Self {
            key,
            uid: string_at(metadata, &["uid"]),
            raw: object.clone(),
            readiness,
            control_plane_initialized,
            connectivity,
            paused: metadata
                .get("annotations")
                .and_then(|annotations| annotations.get("cluster.x-k8s.io/paused"))
                .and_then(Value::as_str)
                == Some("true")
                || object
                    .get("spec")
                    .and_then(|spec| spec.get("paused"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            deleting: metadata
                .get("deletionTimestamp")
                .is_some_and(|value| !value.is_null()),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeatureEvidence {
    pub id: String,
    pub configured: bool,
    pub raw_report: Option<Value>,
    pub reported_state: Option<String>,
    pub failure_reason: Option<String>,
    pub failure_message: Option<String>,
    pub consecutive_failures: Option<u64>,
    pub last_applied_time: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisioningBucket {
    Provisioned,
    Failed,
    Progressing,
    Removing,
    Blocked,
    MissingSummary,
    NoFeaturesConfigured,
    DryRun,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetRelation {
    Observed,
    ReportedFailure,
    SummaryWithoutObservedTarget,
    RemovingAfterMatch,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeploymentEvidence {
    pub profile: Option<ProfileKey>,
    pub reported_owner: Option<ProfileKey>,
    pub owner_uid: Option<String>,
    pub cluster: Option<ClusterKey>,
    pub summary: Option<Value>,
    pub relation: TargetRelation,
    pub bucket: ProvisioningBucket,
    pub features: Vec<FeatureEvidence>,
    pub sync_mode: SyncMode,
    pub rollout_updated: Option<bool>,
    pub rollout_updating: Option<bool>,
    pub rollout_conflict: bool,
    pub summary_failure_message: Option<String>,
    pub helm_conflict: bool,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProvisioningCounts {
    pub provisioned: usize,
    pub failed: usize,
    pub progressing: usize,
    pub removing: usize,
    pub blocked: usize,
    pub missing_summary: usize,
    pub no_features_configured: usize,
    pub dry_run: usize,
    pub unknown: usize,
}

impl ProvisioningCounts {
    pub fn add(&mut self, bucket: ProvisioningBucket) {
        let count = match bucket {
            ProvisioningBucket::Provisioned => &mut self.provisioned,
            ProvisioningBucket::Failed => &mut self.failed,
            ProvisioningBucket::Progressing => &mut self.progressing,
            ProvisioningBucket::Removing => &mut self.removing,
            ProvisioningBucket::Blocked => &mut self.blocked,
            ProvisioningBucket::MissingSummary => &mut self.missing_summary,
            ProvisioningBucket::NoFeaturesConfigured => &mut self.no_features_configured,
            ProvisioningBucket::DryRun => &mut self.dry_run,
            ProvisioningBucket::Unknown => &mut self.unknown,
        };
        *count += 1;
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SveltosModel {
    pub profiles: Vec<ProfileEvidence>,
    pub clusters: Vec<ClusterEvidence>,
    pub deployments: Vec<DeploymentEvidence>,
    pub source_reads: BTreeMap<SourceKind, SourceRead>,
}

impl SveltosModel {
    pub fn set_source_read(&mut self, source: SourceKind, read: SourceRead) {
        self.source_reads.insert(source, read);
    }

    pub fn source_read(&self, source: SourceKind) -> Option<&SourceRead> {
        self.source_reads.get(&source)
    }

    pub fn build(
        profile_objects: &[Value],
        summary_objects: &[Value],
        cluster_objects: &[Value],
    ) -> Self {
        let profiles = profile_objects
            .iter()
            .filter_map(ProfileEvidence::from_object)
            .collect::<Vec<_>>();
        let clusters = cluster_objects
            .iter()
            .filter_map(ClusterEvidence::from_object)
            .collect::<Vec<_>>();
        let mut profile_by_key = BTreeMap::<ProfileKey, Vec<&ProfileEvidence>>::new();
        for profile in &profiles {
            profile_by_key
                .entry(profile.key.clone())
                .or_default()
                .push(profile);
        }
        let mut deployments = Vec::new();

        for profile in &profiles {
            let matching = profile.matching_clusters.as_deref().unwrap_or_default();
            let failed = profile.failed_clusters.as_deref().unwrap_or_default();
            let updated = profile.updated_clusters.as_deref().unwrap_or_default();
            let updating = profile.updating_clusters.as_deref().unwrap_or_default();
            let targets = matching
                .iter()
                .chain(failed)
                .cloned()
                .collect::<BTreeSet<_>>();
            for cluster in targets {
                let is_failed = failed.contains(&cluster);
                let ambiguous_profile = profile_by_key
                    .get(&profile.key)
                    .is_some_and(|matches| matches.len() > 1);
                let unresolved_cluster = !cluster.is_well_formed();
                let mut deployment = DeploymentEvidence {
                    profile: (!ambiguous_profile).then(|| profile.key.clone()),
                    reported_owner: Some(profile.key.clone()),
                    owner_uid: profile.uid.clone(),
                    cluster: Some(cluster.clone()),
                    summary: None,
                    relation: if ambiguous_profile || unresolved_cluster {
                        TargetRelation::Unresolved
                    } else if is_failed {
                        TargetRelation::ReportedFailure
                    } else {
                        TargetRelation::Observed
                    },
                    bucket: if (ambiguous_profile || unresolved_cluster) && !is_failed {
                        ProvisioningBucket::Unknown
                    } else if is_failed {
                        ProvisioningBucket::Failed
                    } else {
                        ProvisioningBucket::MissingSummary
                    },
                    features: Vec::new(),
                    sync_mode: profile.sync_mode,
                    rollout_updated: membership(profile.updated_clusters.as_ref(), &cluster),
                    rollout_updating: membership(profile.updating_clusters.as_ref(), &cluster),
                    rollout_conflict: updated.contains(&cluster) && updating.contains(&cluster),
                    summary_failure_message: None,
                    helm_conflict: false,
                    diagnostic: None,
                };
                if is_failed {
                    deployment.diagnostic = Some(reported_failure(profile, &cluster));
                } else if ambiguous_profile {
                    deployment.diagnostic =
                        Some("Duplicate profile evidence for the same identity".to_owned());
                } else if unresolved_cluster {
                    deployment.diagnostic =
                        Some("Profile target has an unknown or missing cluster type".to_owned());
                }
                deployments.push(deployment);
            }
        }

        for summary in summary_objects {
            let target = summary_cluster_key(summary);
            let owner = summary_owner(summary);
            let owner_profile = owner.as_ref().map(|(key, _)| key.clone());
            let current_profile = owner_profile.as_ref().and_then(|key| {
                profile_by_key
                    .get(key)
                    .filter(|matches| matches.len() == 1)
                    .and_then(|matches| matches.first().copied())
            });
            let profile_reports_failure = current_profile.is_some_and(|profile| {
                profile.failed_clusters.as_ref().is_some_and(|clusters| {
                    target
                        .as_ref()
                        .is_some_and(|target| clusters.contains(target))
                })
            });
            let profile_reports_match = current_profile.is_some_and(|profile| {
                profile.matching_clusters.as_ref().is_some_and(|clusters| {
                    target
                        .as_ref()
                        .is_some_and(|target| clusters.contains(target))
                })
            });
            let uid_mismatch = owner.as_ref().is_some_and(|(_, uid)| {
                uid.as_ref().is_some_and(|owner_uid| {
                    current_profile
                        .and_then(|profile| profile.uid.as_ref())
                        .is_some_and(|profile_uid| profile_uid != owner_uid)
                })
            });
            let uid_unverifiable = owner.as_ref().is_some_and(|(_, uid)| {
                uid.is_some() && current_profile.is_some_and(|profile| profile.uid.is_none())
            });
            let resolved_profile = if uid_mismatch || uid_unverifiable {
                None
            } else {
                current_profile.map(|profile| profile.key.clone())
            };
            let existing = resolved_profile.as_ref().and_then(|profile| {
                deployments.iter().position(|deployment| {
                    deployment.profile.as_ref() == Some(profile)
                        && deployment.cluster == target
                        && deployment.summary.is_none()
                })
            });
            let duplicate_summary = resolved_profile.as_ref().and_then(|profile| {
                deployments.iter().position(|deployment| {
                    deployment.profile.as_ref() == Some(profile)
                        && deployment.cluster == target
                        && deployment.summary.is_some()
                })
            });
            let relation = if owner.is_none()
                || target.is_none()
                || target
                    .as_ref()
                    .is_some_and(|cluster| !cluster.is_well_formed())
                || uid_mismatch
                || uid_unverifiable
                || current_profile.is_none()
            {
                TargetRelation::Unresolved
            } else if summary_is_removing(summary) {
                TargetRelation::RemovingAfterMatch
            } else if profile_reports_failure {
                TargetRelation::ReportedFailure
            } else if profile_reports_match {
                TargetRelation::Observed
            } else {
                TargetRelation::SummaryWithoutObservedTarget
            };
            let sync_mode = current_profile.map_or(SyncMode::Unknown, |profile| profile.sync_mode);
            let verified_profile = current_profile.filter(|_| resolved_profile.is_some());
            let mut deployment =
                deployment_from_summary(summary, resolved_profile, target, relation, sync_mode);
            deployment.reported_owner = owner_profile;
            deployment.owner_uid = owner.as_ref().and_then(|(_, uid)| uid.clone());
            if let (Some(profile), Some(cluster)) = (verified_profile, deployment.cluster.as_ref())
            {
                deployment.rollout_updated = membership(profile.updated_clusters.as_ref(), cluster);
                deployment.rollout_updating =
                    membership(profile.updating_clusters.as_ref(), cluster);
                deployment.rollout_conflict = deployment.rollout_updated == Some(true)
                    && deployment.rollout_updating == Some(true);
            }
            if profile_reports_failure
                && !uid_mismatch
                && !uid_unverifiable
                && let (Some(profile), Some(cluster)) =
                    (current_profile, deployment.cluster.as_ref())
            {
                deployment.bucket = ProvisioningBucket::Failed;
                deployment.diagnostic = Some(reported_failure(profile, cluster));
            } else if uid_mismatch {
                deployment.diagnostic =
                    Some("Summary owner UID does not match current profile".to_owned());
            } else if uid_unverifiable {
                deployment.diagnostic =
                    Some("Summary owner UID cannot be verified against current profile".to_owned());
            } else if current_profile.is_none() {
                deployment.diagnostic =
                    Some("Summary profile owner is missing or unresolved".to_owned());
            } else if deployment.cluster.is_none()
                || deployment
                    .cluster
                    .as_ref()
                    .is_some_and(|cluster| !cluster.is_well_formed())
            {
                deployment.diagnostic =
                    Some("Summary cluster reference is missing or has unknown type".to_owned());
            }
            if let Some(index) = duplicate_summary {
                let message = "Duplicate summary evidence for profile and cluster identity";
                let previous = &mut deployments[index];
                previous.profile = None;
                previous.relation = TargetRelation::Unresolved;
                previous.bucket = ProvisioningBucket::Unknown;
                previous.diagnostic = Some(message.to_owned());
                deployment.profile = None;
                deployment.relation = TargetRelation::Unresolved;
                deployment.bucket = ProvisioningBucket::Unknown;
                deployment.diagnostic = Some(message.to_owned());
                deployments.push(deployment);
            } else if let Some(index) = existing {
                deployments[index] = deployment;
            } else {
                deployments.push(deployment);
            }
        }

        Self {
            profiles,
            clusters,
            deployments,
            source_reads: BTreeMap::new(),
        }
    }

    pub fn counts_for_profile(&self, profile: &ProfileKey) -> ProvisioningCounts {
        let mut counts = ProvisioningCounts::default();
        for deployment in self
            .deployments
            .iter()
            .filter(|deployment| deployment.profile.as_ref() == Some(profile))
        {
            counts.add(deployment.bucket);
        }
        counts
    }

    pub fn counts_for_cluster(&self, cluster: &ClusterKey) -> ProvisioningCounts {
        let mut counts = ProvisioningCounts::default();
        let mut seen = BTreeSet::new();
        for deployment in self
            .deployments
            .iter()
            .filter(|deployment| deployment.cluster.as_ref() == Some(cluster))
        {
            if seen.insert((
                deployment.reported_owner.clone(),
                deployment.cluster.clone(),
            )) {
                counts.add(deployment.bucket);
            }
        }
        counts
    }

    pub fn inventory_for_cluster<'a>(
        inventory: &'a [InventoryEvidence],
        cluster: &ClusterKey,
    ) -> Vec<&'a InventoryEvidence> {
        inventory
            .iter()
            .filter(|evidence| evidence.cluster.as_ref() == Some(cluster))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryEvidence {
    pub raw: Value,
    pub cluster: Option<ClusterKey>,
    pub sections: Vec<InventorySection>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventorySection {
    pub name: String,
    pub raw: Value,
    pub features: Vec<Value>,
}

pub fn inventory_evidence(objects: &[Value]) -> Vec<InventoryEvidence> {
    objects
        .iter()
        .map(|object| {
            let cluster = ClusterKey::from_configuration_labels(object);
            let mut sections = Vec::new();
            for field in ["clusterProfileResources", "profileResources"] {
                if let Some(entries) = object
                    .get("status")
                    .and_then(|status| status.get(field))
                    .and_then(Value::as_array)
                {
                    sections.extend(entries.iter().enumerate().map(|(index, raw)| {
                        let mut features = Vec::new();
                        collect_uppercase_features(raw, &mut features);
                        InventorySection {
                            name: format!("{field}[{index}]"),
                            raw: raw.clone(),
                            features,
                        }
                    }));
                }
            }
            let diagnostic = if cluster.is_none() {
                Some("ClusterConfiguration lacks usable cluster identity labels".to_owned())
            } else if cluster
                .as_ref()
                .is_some_and(|cluster| !cluster.is_well_formed())
            {
                Some("ClusterConfiguration has an unknown cluster type label".to_owned())
            } else {
                None
            };
            InventoryEvidence {
                raw: object.clone(),
                cluster,
                sections,
                diagnostic,
            }
        })
        .collect()
}

fn collect_uppercase_features(value: &Value, features: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            if let Some(value) = map.get("Features") {
                match value {
                    Value::Array(items) => features.extend(items.iter().cloned()),
                    Value::Null => {}
                    other => features.push(other.clone()),
                }
            }
            for (key, child) in map {
                if key != "Features" {
                    collect_uppercase_features(child, features);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_uppercase_features(item, features);
            }
        }
        _ => {}
    }
}

fn deployment_from_summary(
    summary: &Value,
    profile: Option<ProfileKey>,
    cluster: Option<ClusterKey>,
    relation: TargetRelation,
    sync_mode: SyncMode,
) -> DeploymentEvidence {
    let spec = summary.get("spec").unwrap_or(&Value::Null);
    let profile_spec = spec.get("clusterProfileSpec");
    let configured = profile_spec.map_or_else(Vec::new, configured_features);
    let has_configured_features = !configured.is_empty();
    let reports = summary
        .get("status")
        .and_then(|status| status.get("featureSummaries"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut features = Vec::new();
    for feature in &configured {
        let matching_reports = reports
            .iter()
            .filter(|report| string_at(report, &["featureID"]).as_deref() == Some(feature.as_str()))
            .collect::<Vec<_>>();
        if matching_reports.is_empty() {
            features.push(feature_evidence(feature, true, None));
        } else {
            features.extend(
                matching_reports
                    .into_iter()
                    .map(|report| feature_evidence(feature, true, Some(report))),
            );
        }
    }
    for report in &reports {
        let Some(id) = string_at(report, &["featureID"]) else {
            features.push(feature_evidence("<missing featureID>", false, Some(report)));
            continue;
        };
        if !configured.contains(&id) {
            features.push(feature_evidence(&id, false, Some(report)));
        }
    }
    let summary_failure_message =
        string_at(summary, &["status", "failureMessage"]).filter(|message| !message.is_empty());
    let helm_conflict = summary
        .pointer("/status/helmReleaseSummaries")
        .and_then(Value::as_array)
        .is_some_and(|releases| {
            releases.iter().any(|release| {
                string_at(release, &["status"]).as_deref() == Some("Conflict")
                    || string_at(release, &["conflictMessage"])
                        .is_some_and(|message| !message.is_empty())
            })
        });
    let removing = summary_is_removing(summary);
    let explicit_failure = summary_failure_message.is_some()
        || helm_conflict
        || features.iter().any(|feature| {
            matches!(
                feature.reported_state.as_deref(),
                Some("Failed" | "FailedNonRetriable")
            ) || feature
                .failure_reason
                .as_deref()
                .is_some_and(|reason| !reason.is_empty())
                || feature
                    .failure_message
                    .as_deref()
                    .is_some_and(|message| !message.is_empty())
        });
    let any_progressing = features
        .iter()
        .any(|feature| feature.reported_state.as_deref() == Some("Provisioning"));
    let any_blocked = features
        .iter()
        .any(|feature| feature.reported_state.as_deref() == Some("Blocked"));
    let all_provisioned = !features.is_empty()
        && features
            .iter()
            .filter(|feature| feature.configured)
            .all(|feature| feature.reported_state.as_deref() == Some("Provisioned"))
        && features
            .iter()
            .filter(|feature| !feature.configured)
            .all(|feature| feature.reported_state.as_deref() == Some("Provisioned"));
    let bucket = if explicit_failure {
        ProvisioningBucket::Failed
    } else if removing {
        ProvisioningBucket::Removing
    } else if any_blocked {
        ProvisioningBucket::Blocked
    } else if any_progressing {
        ProvisioningBucket::Progressing
    } else if relation == TargetRelation::Unresolved || profile_spec.is_none() {
        ProvisioningBucket::Unknown
    } else if !has_configured_features {
        ProvisioningBucket::NoFeaturesConfigured
    } else if all_provisioned {
        if sync_mode == SyncMode::DryRun {
            ProvisioningBucket::DryRun
        } else {
            ProvisioningBucket::Provisioned
        }
    } else {
        ProvisioningBucket::Unknown
    };
    DeploymentEvidence {
        profile,
        reported_owner: None,
        owner_uid: None,
        cluster,
        summary: Some(summary.clone()),
        relation,
        bucket,
        features,
        sync_mode,
        rollout_updated: None,
        rollout_updating: None,
        rollout_conflict: false,
        summary_failure_message,
        helm_conflict,
        diagnostic: None,
    }
}

fn feature_evidence(id: &str, configured: bool, report: Option<&Value>) -> FeatureEvidence {
    FeatureEvidence {
        id: id.to_owned(),
        configured,
        raw_report: report.cloned(),
        reported_state: report.and_then(|value| string_at(value, &["status"])),
        failure_reason: report.and_then(|value| string_at(value, &["failureReason"])),
        failure_message: report.and_then(|value| string_at(value, &["failureMessage"])),
        consecutive_failures: report
            .and_then(|value| value.get("consecutiveFailures"))
            .and_then(Value::as_u64),
        last_applied_time: report.and_then(|value| string_at(value, &["lastAppliedTime"])),
    }
}

fn configured_features(profile_spec: &Value) -> Vec<String> {
    [
        ("helmCharts", "Helm"),
        ("policyRefs", "Resources"),
        ("kustomizationRefs", "Kustomize"),
    ]
    .into_iter()
    .filter_map(|(field, feature)| {
        profile_spec
            .get(field)
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .map(|_| feature.to_owned())
    })
    .collect()
}

fn summary_cluster_key(summary: &Value) -> Option<ClusterKey> {
    let spec = summary.get("spec")?;
    Some(ClusterKey::from_sveltos_type(
        string_at(spec, &["clusterType"]).as_deref()?,
        string_at(spec, &["clusterNamespace"]).as_deref()?,
        string_at(spec, &["clusterName"]).as_deref()?,
    ))
}

fn summary_owner(summary: &Value) -> Option<(ProfileKey, Option<String>)> {
    let owners = summary
        .get("metadata")?
        .get("ownerReferences")?
        .as_array()?;
    let matches = owners
        .iter()
        .filter_map(|owner| {
            Some((
                ProfileKey::from_owner_reference(owner, summary)?,
                string_at(owner, &["uid"]),
            ))
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

fn summary_is_removing(summary: &Value) -> bool {
    summary
        .get("metadata")
        .and_then(|metadata| metadata.get("deletionTimestamp"))
        .is_some_and(|value| !value.is_null())
        || summary
            .get("status")
            .and_then(|status| status.get("featureSummaries"))
            .and_then(Value::as_array)
            .is_some_and(|features| {
                features.iter().any(|feature| {
                    matches!(
                        string_at(feature, &["status"]).as_deref(),
                        Some("Removing" | "AgentRemoving")
                    )
                })
            })
}

/// Profile status references are `corev1.ObjectReference`s; an unknown kind or
/// group yields a key that is not well formed instead of a guessed cluster type.
fn object_ref(entry: &Value) -> ClusterKey {
    let kind = string_at(entry, &["kind"]).unwrap_or_default();
    let namespace = string_at(entry, &["namespace"]).unwrap_or_default();
    let name = string_at(entry, &["name"]).unwrap_or_default();
    match (kind.as_str(), api_group(entry).as_deref()) {
        ("SveltosCluster", None | Some(SVELTOS_CLUSTER_GROUP)) => {
            ClusterKey::from_sveltos_type("Sveltos", &namespace, &name)
        }
        ("Cluster", None | Some(CAPI_CLUSTER_GROUP)) => {
            ClusterKey::from_sveltos_type("Capi", &namespace, &name)
        }
        _ => ClusterKey {
            api_group: String::new(),
            kind,
            namespace,
            name,
        },
    }
}

fn cluster_refs(value: Option<&Value>) -> Option<Vec<ClusterKey>> {
    Some(value?.as_array()?.iter().map(object_ref).collect())
}

/// `updatedClusters`/`updatingClusters` are `{hash, clusters}` structs whose
/// empty `clusters` list is omitted, so a present struct means a known set.
fn rollout_refs(value: Option<&Value>) -> Option<Vec<ClusterKey>> {
    value
        .filter(|value| value.is_object())
        .map(|value| cluster_refs(value.get("clusters")).unwrap_or_default())
}

fn failed_refs(value: Option<&Value>) -> Option<Vec<ClusterKey>> {
    Some(
        value?
            .as_array()?
            .iter()
            .map(|entry| object_ref(entry.get("clusterRef").unwrap_or(&Value::Null)))
            .collect(),
    )
}

fn reported_failure(profile: &ProfileEvidence, cluster: &ClusterKey) -> String {
    let message = profile
        .raw
        .pointer("/status/failedClusters")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|entry| {
                object_ref(entry.get("clusterRef").unwrap_or(&Value::Null)) == *cluster
            })
        })
        .and_then(|entry| string_at(entry, &["failureMessage"]))
        .filter(|message| !message.is_empty());
    match message {
        Some(message) => format!("Profile reports this target in failedClusters: {message}"),
        None => "Profile reports this target in failedClusters".to_owned(),
    }
}

fn membership(reports: Option<&Vec<ClusterKey>>, key: &ClusterKey) -> Option<bool> {
    reports.map(|reports| reports.contains(key))
}

fn sync_mode(value: Option<&str>) -> SyncMode {
    match value {
        Some("Continuous" | "ContinuousWithDriftDetection") => SyncMode::Continuous,
        Some("OneTime") => SyncMode::OneTime,
        Some("DryRun") => SyncMode::DryRun,
        _ => SyncMode::Unknown,
    }
}

fn api_group(object: &Value) -> Option<String> {
    object
        .get("apiVersion")?
        .as_str()?
        .split_once('/')
        .map(|(group, _)| group.to_owned())
}

fn string_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_owned)
}

fn scalar_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile(kind: &str, namespace: Option<&str>, uid: &str, sync_mode: &str) -> Value {
        let mut metadata = json!({"name": "base", "uid": uid});
        if let Some(namespace) = namespace {
            metadata["namespace"] = json!(namespace);
        }
        json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": kind,
            "metadata": metadata,
            "spec": {"syncMode": sync_mode, "helmCharts": [{"name": "app"}]}
        })
    }

    fn summary(cluster_type: &str, owner_uid: &str, feature_summaries: Value) -> Value {
        json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "ClusterSummary",
            "metadata": {
                "name": "generated-name-is-irrelevant",
                "namespace": "team",
                "ownerReferences": [{
                    "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
                    "kind": "Profile",
                    "name": "base",
                    "uid": owner_uid
                }]
            },
            "spec": {
                "clusterType": cluster_type,
                "clusterNamespace": "team",
                "clusterName": "west",
                "clusterProfileSpec": {"helmCharts": [{"name": "app"}]}
            },
            "status": {"featureSummaries": feature_summaries}
        })
    }

    fn feature(id: &str, state: &str) -> Value {
        json!({"featureID": id, "status": state})
    }

    fn west_ref() -> Value {
        json!({
            "apiVersion": "lib.projectsveltos.io/v1beta1",
            "kind": "SveltosCluster",
            "namespace": "team",
            "name": "west"
        })
    }

    #[test]
    fn cluster_keys_keep_same_name_across_types_distinct() {
        let sveltos = ClusterKey::from_sveltos_type("Sveltos", "team", "west");
        let capi = ClusterKey::from_sveltos_type("Capi", "team", "west");
        assert_ne!(sveltos, capi);
        assert_eq!(sveltos.kind, "SveltosCluster");
        assert_eq!(capi.kind, "Cluster");
        assert_eq!(
            ClusterKey::from_object(&json!({
                "apiVersion": "lib.projectsveltos.io/v1beta1",
                "kind": "SveltosCluster",
                "metadata": {"namespace": "team", "name": "west"}
            })),
            Some(sveltos.clone())
        );
        assert_eq!(
            ClusterKey::from_object(&json!({
                "apiVersion": "cluster.x-k8s.io/v1beta2",
                "kind": "Cluster",
                "metadata": {"namespace": "team", "name": "west"}
            })),
            Some(capi.clone())
        );

        let profile = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "Profile",
            "metadata": {"name": "base", "namespace": "team", "uid": "p-uid"},
            "spec": {"syncMode": "Continuous"},
            "status": {"matchingClusters": [
                west_ref(),
                {"apiVersion": "cluster.x-k8s.io/v1beta2", "kind": "Cluster", "namespace": "team", "name": "west"}
            ]}
        });
        let model = SveltosModel::build(&[profile], &[], &[]);
        assert_eq!(model.deployments.len(), 2);
        assert_ne!(model.deployments[0].cluster, model.deployments[1].cluster);
    }

    #[test]
    fn profile_keys_keep_cluster_and_namespaced_scopes_distinct() {
        let cluster_profile =
            ProfileKey::from_object(&profile("ClusterProfile", None, "cp-uid", "Continuous"))
                .unwrap();
        let profile =
            ProfileKey::from_object(&profile("Profile", Some("team"), "p-uid", "Continuous"))
                .unwrap();
        assert_ne!(cluster_profile, profile);
        assert_eq!(cluster_profile.namespace, None);
        assert_eq!(profile.namespace.as_deref(), Some("team"));
    }

    #[test]
    fn cluster_profile_summary_owner_correlates_without_namespace() {
        let profile = profile("ClusterProfile", None, "cp-uid", "Continuous");
        let mut summary = summary("Capi", "cp-uid", json!([feature("Helm", "Provisioned")]));
        summary["metadata"]["ownerReferences"][0]["kind"] = json!("ClusterProfile");
        summary["metadata"]["namespace"] = json!("controller-namespace");
        let model = SveltosModel::build(&[profile], &[summary], &[]);

        assert_eq!(model.deployments.len(), 1);
        assert_eq!(
            model.deployments[0].profile.as_ref().unwrap().namespace,
            None
        );
        assert_eq!(
            model.deployments[0].cluster,
            Some(ClusterKey::from_sveltos_type("Capi", "team", "west"))
        );
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Provisioned);
    }

    #[test]
    fn replaced_owner_uid_does_not_attribute_summary_to_new_profile() {
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "new-uid", "Continuous")],
            &[summary(
                "Sveltos",
                "old-uid",
                json!([feature("Helm", "Provisioned")]),
            )],
            &[],
        );
        assert_eq!(model.deployments.len(), 1);
        assert_eq!(model.deployments[0].profile, None);
        assert_eq!(model.deployments[0].relation, TargetRelation::Unresolved);
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Unknown);
        assert_eq!(
            model.deployments[0].diagnostic.as_deref(),
            Some("Summary owner UID does not match current profile")
        );
    }

    #[test]
    fn duplicate_summaries_remain_unresolved_without_double_success_count() {
        let profile = profile("Profile", Some("team"), "p-uid", "Continuous");
        let summary = summary("Sveltos", "p-uid", json!([feature("Helm", "Provisioned")]));
        let cluster = ClusterKey::from_sveltos_type("Sveltos", "team", "west");
        let model = SveltosModel::build(&[profile], &[summary.clone(), summary], &[]);

        assert_eq!(model.deployments.len(), 2);
        assert!(
            model
                .deployments
                .iter()
                .all(|deployment| deployment.relation == TargetRelation::Unresolved)
        );
        assert_eq!(model.counts_for_cluster(&cluster).unknown, 1);
        assert_eq!(
            model.counts_for_profile(&model.profiles[0].key).provisioned,
            0
        );
    }

    #[test]
    fn missing_summary_is_not_provisioned_and_missing_matching_status_is_unknown() {
        let profile = profile("Profile", Some("team"), "p-uid", "Continuous");
        let model = SveltosModel::build(&[profile], &[], &[]);
        let profile = &model.profiles[0];
        assert_eq!(profile.matching_clusters, None);
        assert_eq!(model.deployments.len(), 0);

        let profile = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "Profile",
            "metadata": {"name": "base", "namespace": "team"},
            "spec": {"syncMode": "Continuous"},
            "status": {"matchingClusters": [west_ref()]}
        });
        let model = SveltosModel::build(&[profile], &[], &[]);
        assert_eq!(
            model.deployments[0].bucket,
            ProvisioningBucket::MissingSummary
        );
        assert_eq!(model.deployments[0].rollout_updated, None);
    }

    #[test]
    fn missing_cluster_reference_fields_remain_unresolved() {
        let profile = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "Profile",
            "metadata": {"name": "base", "namespace": "team"},
            "spec": {"syncMode": "Continuous"},
            "status": {"matchingClusters": [
                {"namespace": "team", "name": "west"},
                {"apiVersion": "example.io/v1", "kind": "Cluster", "namespace": "team", "name": "west"}
            ]}
        });
        let model = SveltosModel::build(&[profile], &[], &[]);

        assert_eq!(model.deployments.len(), 2);
        for deployment in &model.deployments {
            assert_eq!(deployment.relation, TargetRelation::Unresolved);
            assert_eq!(deployment.bucket, ProvisioningBucket::Unknown);
            assert_eq!(
                deployment.diagnostic.as_deref(),
                Some("Profile target has an unknown or missing cluster type")
            );
        }
    }

    #[test]
    fn profile_reported_failure_without_summary_remains_a_failed_deployment() {
        let profile = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "Profile",
            "metadata": {"name": "base", "namespace": "team"},
            "spec": {"syncMode": "Continuous"},
            "status": {"failedClusters": [{
                "clusterRef": west_ref(),
                "failureMessage": "ClusterSummary create denied"
            }]}
        });
        let model = SveltosModel::build(&[profile], &[], &[]);

        assert_eq!(model.deployments.len(), 1);
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Failed);
        assert_eq!(
            model.deployments[0].relation,
            TargetRelation::ReportedFailure
        );
        assert_eq!(
            model.deployments[0].diagnostic.as_deref(),
            Some("Profile reports this target in failedClusters: ClusterSummary create denied")
        );
    }

    #[test]
    fn mixed_feature_states_count_one_failed_deployment() {
        let mut summary = summary(
            "Sveltos",
            "p-uid",
            json!([
                feature("Helm", "Provisioned"),
                feature("Resources", "Failed")
            ]),
        );
        summary["spec"]["clusterProfileSpec"]["policyRefs"] = json!([{"name": "policy"}]);
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary],
            &[],
        );
        let deployment = &model.deployments[0];
        assert_eq!(deployment.bucket, ProvisioningBucket::Failed);
        assert_eq!(model.counts_for_profile(&model.profiles[0].key).failed, 1);
        assert_eq!(
            model.counts_for_profile(&model.profiles[0].key).provisioned,
            0
        );
    }

    #[test]
    fn configured_feature_without_report_is_unknown_not_success() {
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary("Sveltos", "p-uid", json!([]))],
            &[],
        );
        let deployment = &model.deployments[0];
        assert_eq!(deployment.bucket, ProvisioningBucket::Unknown);
        assert!(deployment.features[0].configured);
        assert_eq!(deployment.features[0].id, "Helm");
        assert_eq!(deployment.features[0].reported_state, None);
    }

    #[test]
    fn unrecognized_feature_state_is_preserved_and_never_success() {
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "FutureState")]),
            )],
            &[],
        );
        let deployment = &model.deployments[0];
        assert_eq!(deployment.bucket, ProvisioningBucket::Unknown);
        assert_eq!(
            deployment.features[0].reported_state.as_deref(),
            Some("FutureState")
        );
    }

    #[test]
    fn provisioning_and_conflicting_rollout_evidence_remain_separate() {
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "Provisioning")]),
            )],
            &[],
        );
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Progressing);

        let mut profile = profile("Profile", Some("team"), "p-uid", "OneTime");
        profile["status"] = json!({
            "matchingClusters": [west_ref()],
            "updatedClusters": {"hash": "aGFzaA==", "clusters": [west_ref()]},
            "updatingClusters": {"clusters": [west_ref()]}
        });
        let model = SveltosModel::build(
            &[profile],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "Provisioned")]),
            )],
            &[],
        );
        let deployment = &model.deployments[0];
        assert_eq!(deployment.bucket, ProvisioningBucket::Provisioned);
        assert_eq!(deployment.rollout_updated, Some(true));
        assert_eq!(deployment.rollout_updating, Some(true));
        assert!(deployment.rollout_conflict);
    }

    #[test]
    fn provisioned_capi_control_plane_is_not_readiness_or_connectivity() {
        let cluster = json!({
            "apiVersion": "cluster.x-k8s.io/v1beta1",
            "kind": "Cluster",
            "metadata": {"name": "west", "namespace": "team"},
            "status": {"initialized": true}
        });
        let evidence = ClusterEvidence::from_object(&cluster).unwrap();
        assert_eq!(evidence.readiness, None);
        assert_eq!(evidence.control_plane_initialized, Some(true));
        assert_eq!(evidence.connectivity, None);
    }

    #[test]
    fn empty_feature_configuration_and_helm_conflicts_are_not_success() {
        let mut empty = summary("Sveltos", "p-uid", json!([]));
        empty["spec"]["clusterProfileSpec"]
            .as_object_mut()
            .unwrap()
            .remove("helmCharts");
        let empty_model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[empty],
            &[],
        );
        assert_eq!(
            empty_model.deployments[0].bucket,
            ProvisioningBucket::NoFeaturesConfigured
        );

        let mut conflicted = summary("Sveltos", "p-uid", json!([feature("Helm", "Provisioned")]));
        conflicted["status"]["helmReleaseSummaries"] = json!([{
            "releaseName": "app",
            "releaseNamespace": "apps",
            "status": "Conflict"
        }]);
        let conflict_model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[conflicted],
            &[],
        );
        assert!(conflict_model.deployments[0].helm_conflict);
        assert_eq!(
            conflict_model.deployments[0].bucket,
            ProvisioningBucket::Failed
        );
    }

    #[test]
    fn dry_run_never_counts_as_provisioned() {
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "DryRun")],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "Provisioned")]),
            )],
            &[],
        );
        let deployment = &model.deployments[0];
        assert_eq!(deployment.sync_mode, SyncMode::DryRun);
        assert_eq!(deployment.bucket, ProvisioningBucket::DryRun);
    }

    #[test]
    fn profile_status_distinguishes_absent_and_explicitly_empty_targets() {
        let absent = profile("Profile", Some("team"), "p-uid", "Continuous");
        let empty = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "Profile",
            "metadata": {"name": "empty", "namespace": "team"},
            "spec": {"syncMode": "Continuous"},
            "status": {"matchingClusters": []}
        });
        let model = SveltosModel::build(&[absent, empty], &[], &[]);

        assert_eq!(model.profiles[0].matching_clusters, None);
        assert_eq!(model.profiles[1].matching_clusters, Some(Vec::new()));
        assert!(model.deployments.is_empty());
    }

    #[test]
    fn matching_summary_is_observed_and_profile_failure_cannot_be_overridden() {
        let cluster = json!({
            "apiVersion": "lib.projectsveltos.io/v1beta1",
            "kind": "SveltosCluster",
            "metadata": {"name": "west", "namespace": "team", "uid": "cluster-uid"},
            "status": {"ready": true, "connectionStatus": "Connected"}
        });
        let mut profile = profile("Profile", Some("team"), "p-uid", "Continuous");
        profile["status"] = json!({
            "matchingClusters": [west_ref()],
            "failedClusters": [{"clusterRef": west_ref(), "failureMessage": "update denied"}],
            "updatedClusters": {},
            "updatingClusters": {}
        });
        let model = SveltosModel::build(
            &[profile],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "Provisioned")]),
            )],
            &[cluster],
        );

        let deployment = &model.deployments[0];
        assert_eq!(deployment.relation, TargetRelation::ReportedFailure);
        assert_eq!(deployment.bucket, ProvisioningBucket::Failed);
        assert_eq!(deployment.rollout_updated, Some(false));
        assert_eq!(deployment.rollout_updating, Some(false));
        assert_eq!(model.counts_for_profile(&model.profiles[0].key).failed, 1);
        assert_eq!(model.counts_for_cluster(&model.clusters[0].key).failed, 1);
        assert_eq!(model.clusters[0].readiness.as_deref(), Some("true"));
        assert_eq!(model.clusters[0].connectivity.as_deref(), Some("Connected"));
    }

    #[test]
    fn summary_after_matching_stops_remains_as_removal_evidence() {
        let summary = json!({
            "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
            "kind": "ClusterSummary",
            "metadata": {
                "name": "generated",
                "namespace": "team",
                "deletionTimestamp": "2026-10-05T12:00:00Z",
                "ownerReferences": [{
                    "apiVersion": format!("{CONFIG_GROUP}/v1beta1"),
                    "kind": "Profile",
                    "name": "base",
                    "uid": "p-uid"
                }]
            },
            "spec": {
                "clusterType": "Sveltos",
                "clusterNamespace": "team",
                "clusterName": "west",
                "clusterProfileSpec": {"helmCharts": [{"name": "app"}]}
            },
            "status": {"featureSummaries": [feature("Helm", "Removing")]}
        });
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary],
            &[],
        );

        assert_eq!(model.deployments.len(), 1);
        assert_eq!(
            model.deployments[0].relation,
            TargetRelation::RemovingAfterMatch
        );
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Removing);
        assert_eq!(model.counts_for_profile(&model.profiles[0].key).removing, 1);
        assert_eq!(
            model
                .counts_for_cluster(
                    model.deployments[0]
                        .cluster
                        .as_ref()
                        .expect("cluster reference")
                )
                .removing,
            1
        );
    }

    #[test]
    fn cluster_readiness_connectivity_and_capi_initialization_stay_separate() {
        let sveltos = json!({
            "apiVersion": "lib.projectsveltos.io/v1beta1",
            "kind": "SveltosCluster",
            "metadata": {
                "name": "west",
                "namespace": "team",
                "annotations": {"cluster.x-k8s.io/paused": "true"}
            },
            "status": {"ready": false, "connectionStatus": "Disconnected"}
        });
        let capi_v1beta1 = json!({
            "apiVersion": "cluster.x-k8s.io/v1beta1",
            "kind": "Cluster",
            "metadata": {"name": "west", "namespace": "team"},
            "status": {"initialized": true}
        });
        let capi_v1beta2 = json!({
            "apiVersion": "cluster.x-k8s.io/v1beta2",
            "kind": "Cluster",
            "metadata": {"name": "east", "namespace": "team"},
            "status": {"initialization": {"controlPlaneInitialized": true}}
        });

        let sveltos = ClusterEvidence::from_object(&sveltos).unwrap();
        assert_eq!(sveltos.readiness.as_deref(), Some("false"));
        assert_eq!(sveltos.connectivity.as_deref(), Some("Disconnected"));
        assert!(sveltos.paused);

        for capi in [&capi_v1beta1, &capi_v1beta2] {
            let evidence = ClusterEvidence::from_object(capi).unwrap();
            assert_eq!(evidence.readiness, None);
            assert_eq!(evidence.connectivity, None);
            assert_eq!(evidence.control_plane_initialized, Some(true));
        }

        let unknown = json!({
            "apiVersion": "cluster.x-k8s.io/v1beta2",
            "kind": "Cluster",
            "metadata": {"name": "unknown", "namespace": "team"},
            "status": {}
        });
        let evidence = ClusterEvidence::from_object(&unknown).unwrap();
        assert_eq!(evidence.readiness, None);
        assert_eq!(evidence.connectivity, None);
        assert_eq!(evidence.control_plane_initialized, None);
    }

    #[test]
    fn removal_blocked_and_non_retriable_failure_keep_single_bucket() {
        let mut removing = summary("Sveltos", "p-uid", json!([feature("Helm", "Blocked")]));
        removing["metadata"]["deletionTimestamp"] = json!("2026-10-05T12:00:00Z");
        let owner = profile("Profile", Some("team"), "p-uid", "OneTime");
        let model = SveltosModel::build(&[owner], &[removing], &[]);
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Removing);
        assert_eq!(
            model.deployments[0].features[0].reported_state.as_deref(),
            Some("Blocked")
        );

        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "OneTime")],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "FailedNonRetriable")]),
            )],
            &[],
        );
        assert_eq!(model.deployments[0].bucket, ProvisioningBucket::Failed);
        assert_eq!(model.deployments[0].sync_mode, SyncMode::OneTime);
    }

    #[test]
    fn provisioned_summary_does_not_imply_latest_rollout_and_onetime_is_apply_once() {
        let mut profile = profile("Profile", Some("team"), "p-uid", "OneTime");
        profile["status"] = json!({
            "matchingClusters": [west_ref()]
        });
        let model = SveltosModel::build(
            &[profile],
            &[summary(
                "Sveltos",
                "p-uid",
                json!([feature("Helm", "Provisioned")]),
            )],
            &[],
        );
        let deployment = &model.deployments[0];

        assert_eq!(deployment.bucket, ProvisioningBucket::Provisioned);
        assert_eq!(deployment.sync_mode, SyncMode::OneTime);
        assert_eq!(deployment.relation, TargetRelation::Observed);
        assert_eq!(deployment.rollout_updated, None);
        assert_eq!(deployment.rollout_updating, None);
    }

    #[test]
    fn errors_dependencies_charts_and_inventory_identity_remain_reported() {
        let mut summary = summary(
            "Sveltos",
            "p-uid",
            json!([{
                "featureID": "Helm",
                "status": "Provisioned",
                "failureReason": "prior failure",
                "failureMessage": "recovered",
                "consecutiveFailures": 2,
                "lastAppliedTime": "2026-10-05T12:00:00Z"
            }]),
        );
        summary["status"]["failureMessage"] = json!("summary warning");
        summary["status"]["helmReleaseSummaries"] = json!([{
            "releaseName": "app-release",
            "releaseNamespace": "apps",
            "status": "Managing",
            "conflictMessage": "release conflict"
        }]);
        let model = SveltosModel::build(
            &[profile("Profile", Some("team"), "p-uid", "Continuous")],
            &[summary],
            &[],
        );
        let deployment = &model.deployments[0];
        let feature = &deployment.features[0];
        assert_eq!(feature.failure_reason.as_deref(), Some("prior failure"));
        assert_eq!(feature.failure_message.as_deref(), Some("recovered"));
        assert_eq!(feature.consecutive_failures, Some(2));
        assert_eq!(
            feature.last_applied_time.as_deref(),
            Some("2026-10-05T12:00:00Z")
        );
        assert_eq!(
            deployment.summary_failure_message.as_deref(),
            Some("summary warning")
        );
        assert!(deployment.helm_conflict);

        let inventory = json!({
            "metadata": {
                "namespace": "team",
                "labels": {
                    "projectsveltos.io/cluster-name": "west",
                    "projectsveltos.io/cluster-type": "Sveltos"
                }
            },
            "status": {
                "clusterProfileResources": [{
                    "Features": [{
                        "group": "apps",
                        "version": "v1",
                        "kind": "Deployment",
                        "namespace": "app-ns",
                        "name": "local-app",
                        "deploymentType": "Local",
                        "lastAppliedTime": "2026-10-05T12:00:00Z"
                    }]
                }],
                "profileResources": [{
                    "Features": [{
                        "group": "batch",
                        "version": "v1",
                        "kind": "Job",
                        "namespace": "remote-ns",
                        "name": "remote-job",
                        "deploymentType": "Remote"
                    }]
                }]
            }
        });
        let inventory = inventory_evidence(&[inventory]);
        assert_eq!(inventory[0].sections.len(), 2);
        assert_eq!(
            inventory[0].sections[0].features[0]["deploymentType"],
            "Local"
        );
        assert_eq!(
            inventory[0].sections[1].features[0]["deploymentType"],
            "Remote"
        );
        assert_eq!(
            inventory[0].sections[0].features[0]["lastAppliedTime"],
            "2026-10-05T12:00:00Z"
        );
        assert_eq!(inventory[0].sections[0].features[0]["namespace"], "app-ns");
        assert_eq!(inventory[0].sections[0].features[0]["group"], "apps");
        assert_eq!(inventory[0].sections[0].features[0]["version"], "v1");
        assert_eq!(inventory[0].sections[0].features[0]["kind"], "Deployment");
        assert_eq!(inventory[0].sections[0].features[0]["name"], "local-app");
        let same_name_other_type = json!({
            "metadata": {
                "namespace": "team",
                "labels": {
                    "projectsveltos.io/cluster-name": "west",
                    "projectsveltos.io/cluster-type": "Capi"
                }
            },
            "status": {}
        });
        let mut inventory_objects = inventory;
        inventory_objects.extend(inventory_evidence(&[same_name_other_type]));
        assert_eq!(
            SveltosModel::inventory_for_cluster(
                &inventory_objects,
                &ClusterKey::from_sveltos_type("Sveltos", "team", "west")
            )
            .len(),
            1
        );
    }

    #[test]
    fn inventory_identity_and_source_availability_distinguish_empty_missing_and_unknown() {
        let missing_labels = json!({"metadata": {"namespace": "team"}, "status": {}});
        let unknown_type = json!({
            "metadata": {
                "namespace": "team",
                "labels": {
                    "projectsveltos.io/cluster-name": "west",
                    "projectsveltos.io/cluster-type": "FutureCluster"
                }
            },
            "status": {}
        });
        let inventory = inventory_evidence(&[missing_labels, unknown_type]);
        assert_eq!(inventory[0].cluster, None);
        assert!(inventory[0].diagnostic.is_some());
        assert_eq!(
            inventory[1]
                .cluster
                .as_ref()
                .map(|cluster| cluster.api_group.as_str()),
            Some("")
        );
        assert!(inventory[1].diagnostic.is_some());
        assert!(inventory_evidence(&[]).is_empty());

        assert_eq!(
            SourceRead::available("now").availability,
            ReadAvailability::Available
        );
        assert_eq!(SourceRead::absent().availability, ReadAvailability::Absent);
        assert_eq!(
            SourceRead::incomplete("partial").availability,
            ReadAvailability::Incomplete
        );
        assert_eq!(
            SourceRead::failed("forbidden").availability,
            ReadAvailability::Failed
        );
        let mut model = SveltosModel::default();
        model.set_source_read(
            SourceKind::ClusterConfiguration,
            SourceRead::available("now"),
        );
        assert!(inventory_evidence(&[]).is_empty());
        assert_eq!(
            model
                .source_read(SourceKind::ClusterConfiguration)
                .map(|read| read.availability),
            Some(ReadAvailability::Available)
        );
        model.set_source_read(
            SourceKind::ClusterConfiguration,
            SourceRead::failed("forbidden"),
        );
        assert_eq!(
            model
                .source_read(SourceKind::ClusterConfiguration)
                .map(|read| read.availability),
            Some(ReadAvailability::Failed)
        );
    }

    #[test]
    fn inventory_uses_serialized_uppercase_features_key_and_identity_labels() {
        let object = json!({
            "metadata": {
                "namespace": "team",
                "labels": {
                    "projectsveltos.io/cluster-name": "west",
                    "projectsveltos.io/cluster-type": "Sveltos"
                }
            },
            "status": {
                "clusterProfileResources": [{
                    "resources": {"Features": [{"kind": "Deployment", "name": "app"}]},
                    "features": [{"kind": "wrong-case"}]
                }]
            }
        });
        let inventory = inventory_evidence(&[object]);
        assert_eq!(
            inventory[0].cluster,
            Some(ClusterKey::from_sveltos_type("Sveltos", "team", "west"))
        );
        assert_eq!(
            inventory[0].sections[0].features,
            vec![json!({"kind": "Deployment", "name": "app"})]
        );
    }

    #[test]
    fn successful_empty_read_is_distinct_from_unavailable_or_failed() {
        assert_eq!(
            SourceRead::available("now").availability,
            ReadAvailability::Available
        );
        assert_eq!(SourceRead::absent().availability, ReadAvailability::Absent);
        assert_eq!(
            SourceRead::failed("forbidden").availability,
            ReadAvailability::Failed
        );
        assert_eq!(
            SourceRead::available("now")
                .stale("refresh denied")
                .freshness,
            Freshness::Stale
        );

        let mut model = SveltosModel::default();
        model.set_source_read(SourceKind::ClusterSummary, SourceRead::absent());
        assert_eq!(
            model
                .source_read(SourceKind::ClusterSummary)
                .map(|read| read.availability),
            Some(ReadAvailability::Absent)
        );
    }
}
