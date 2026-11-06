use super::*;
use crate::app::sveltos::SourceResult;
use crate::sveltos::{SourceKind, SourceRead};
use std::convert::Infallible;

const CONFIG: &str = "config.projectsveltos.io/v1beta1";
const LIB: &str = "lib.projectsveltos.io/v1beta1";
const CAPI: &str = "cluster.x-k8s.io/v1beta2";

fn sveltos_ref(namespace: &str, name: &str) -> Value {
    json!({"apiVersion": LIB, "kind": "SveltosCluster", "namespace": namespace, "name": name})
}

fn sveltos_cluster(namespace: &str, name: &str) -> Value {
    json!({
        "apiVersion": LIB,
        "kind": "SveltosCluster",
        "metadata": {"name": name, "namespace": namespace, "uid": format!("sc-{name}")},
        "status": {"ready": true, "connectionStatus": "Healthy"}
    })
}

fn summary(name: &str, owner: (&str, &str), cluster: (&str, &str), features: Value) -> Value {
    json!({
        "apiVersion": CONFIG,
        "kind": "ClusterSummary",
        "metadata": {
            "name": name,
            "namespace": "team",
            "uid": format!("cs-{name}"),
            "ownerReferences": [{"apiVersion": CONFIG, "kind": owner.0, "name": "base", "uid": owner.1}]
        },
        "spec": {
            "clusterType": cluster.0,
            "clusterNamespace": "team",
            "clusterName": cluster.1,
            "clusterProfileSpec": {"helmCharts": [{}]}
        },
        "status": {"featureSummaries": features}
    })
}

/// One management cluster covering every deployment shape the dashboard
/// distinguishes: failed, reported-failure placeholder, DryRun, removing,
/// unresolved owner, and joined inventory.
fn fleet(at: &str) -> Vec<SourceResult> {
    let helm = || json!([{"featureID": "Helm", "status": "Provisioned"}]);
    let mut west = summary(
        "base-sveltos-west",
        ("ClusterProfile", "p-base"),
        ("Sveltos", "west"),
        json!([
            {"featureID": "Helm", "status": "Provisioned", "lastAppliedTime": "2025-01-01T00:00:00Z"},
            {"featureID": "Resources", "status": "Failed", "consecutiveFailures": 3, "failureMessage": "denied by webhook"}
        ]),
    );
    west["spec"]["clusterProfileSpec"]["policyRefs"] = json!([{}]);
    west["status"]["helmReleaseSummaries"] = json!([{
        "releaseName": "app",
        "releaseNamespace": "apps",
        "status": "Conflict",
        "conflictMessage": "owned by Profile team/other"
    }]);
    west["status"]["dependencies"] = json!("ClusterProfile prereq");
    west["status"]["nextReconcileTime"] = json!("2025-01-01T00:10:00Z");
    let mut old = summary(
        "base-sveltos-old",
        ("ClusterProfile", "p-base"),
        ("Sveltos", "old"),
        json!([{"featureID": "Resources", "status": "Blocked"}]),
    );
    old["metadata"]["deletionTimestamp"] = json!("2025-01-01T00:05:00Z");
    old["spec"]["clusterProfileSpec"] = json!({"policyRefs": [{}]});
    let capi_west =
        json!({"apiVersion": CAPI, "kind": "Cluster", "namespace": "team", "name": "west"});
    let sources = [
        (
            SourceKind::SveltosCluster,
            vec![
                sveltos_cluster("team", "west"),
                sveltos_cluster("ops", "east"),
            ],
        ),
        (
            SourceKind::CapiCluster,
            vec![json!({
                "apiVersion": CAPI,
                "kind": "Cluster",
                "metadata": {
                    "name": "west",
                    "namespace": "team",
                    "uid": "capi-west",
                    "annotations": {"cluster.x-k8s.io/paused": "true"}
                },
                "status": {"initialization": {"controlPlaneInitialized": true}}
            })],
        ),
        (
            SourceKind::ClusterProfile,
            vec![
                json!({
                    "apiVersion": CONFIG,
                    "kind": "ClusterProfile",
                    "metadata": {"name": "base", "uid": "p-base"},
                    "spec": {"syncMode": "Continuous"},
                    "status": {
                        "matchingClusters": [sveltos_ref("team", "west"), capi_west, sveltos_ref("ops", "east")],
                        "updatedClusters": {"clusters": [sveltos_ref("team", "west")]},
                        "updatingClusters": {},
                        "failedClusters": [{
                            "clusterRef": sveltos_ref("ops", "east"),
                            "failureMessage": "summary creation failed: quota"
                        }]
                    }
                }),
                json!({
                    "apiVersion": CONFIG,
                    "kind": "ClusterProfile",
                    "metadata": {"name": "empty", "uid": "p-empty"},
                    "status": {"matchingClusters": []}
                }),
                json!({
                    "apiVersion": CONFIG,
                    "kind": "ClusterProfile",
                    "metadata": {"name": "selector", "uid": "p-selector"},
                    "spec": {"clusterSelector": {"matchLabels": {"env": "prod"}}}
                }),
            ],
        ),
        (
            SourceKind::Profile,
            vec![json!({
                "apiVersion": CONFIG,
                "kind": "Profile",
                "metadata": {"name": "base", "namespace": "team", "uid": "p-team-base"},
                "spec": {"syncMode": "DryRun"},
                "status": {
                    "matchingClusters": [sveltos_ref("team", "west")],
                    "updatedClusters": {},
                    "updatingClusters": {}
                }
            })],
        ),
        (
            SourceKind::ClusterSummary,
            vec![
                west,
                summary(
                    "base-capi-west",
                    ("ClusterProfile", "p-base"),
                    ("Capi", "west"),
                    helm(),
                ),
                summary(
                    "team-base-west",
                    ("Profile", "p-team-base"),
                    ("Sveltos", "west"),
                    helm(),
                ),
                old,
                summary(
                    "base-sveltos-ghost",
                    ("ClusterProfile", "old-uid"),
                    ("Sveltos", "west"),
                    helm(),
                ),
            ],
        ),
        (
            SourceKind::ClusterConfiguration,
            vec![json!({
                "apiVersion": CONFIG,
                "kind": "ClusterConfiguration",
                "metadata": {
                    "name": "sveltos--west",
                    "namespace": "team",
                    "uid": "cc-west",
                    "labels": {
                        "projectsveltos.io/cluster-name": "west",
                        "projectsveltos.io/cluster-type": "Sveltos"
                    }
                },
                "status": {"clusterProfileResources": [{
                    "clusterProfileName": "base",
                    "Features": [
                        {"featureID": "Resources", "resources": [
                            {"group": "", "version": "v1", "kind": "ConfigMap", "namespace": "apps", "name": "settings", "deploymentType": "Local", "lastAppliedTime": "2025-01-01T00:00:00Z"},
                            {"group": "apps", "version": "v1", "kind": "Deployment", "namespace": "apps", "name": "web", "deploymentType": "Remote"}
                        ]},
                        {"featureID": "Helm", "charts": [
                            {"releaseName": "app", "namespace": "apps", "chartVersion": "1.2.3", "repoURL": "https://charts.example"}
                        ]}
                    ]
                }]}
            })],
        ),
    ];
    sources
        .into_iter()
        .map(|(kind, objects)| SourceResult {
            kind,
            objects,
            read: SourceRead::available(at),
        })
        .collect()
}

fn source(reads: &mut [SourceResult], kind: SourceKind) -> &mut SourceResult {
    reads.iter_mut().find(|read| read.kind == kind).unwrap()
}

fn remove(reads: &mut [SourceResult], kind: SourceKind, name: &str) {
    source(reads, kind)
        .objects
        .retain(|object| object["metadata"]["name"] != name);
}

fn fail(reads: &mut [SourceResult], kind: SourceKind, read: SourceRead) {
    let result = source(reads, kind);
    result.objects.clear();
    result.read = read;
}

/// Deliver a gather result for the read currently in flight.
fn feed(app: &mut App, reads: Vec<SourceResult>) {
    app.handle_msg(Msg::SveltosData {
        generation: app.generation,
        request: app.sveltos_request,
        reads,
    });
}

fn dashboard() -> (App, Receiver<Msg>) {
    let (mut app, rx) = test_app();
    app.cluster.register_kind(
        "config.projectsveltos.io",
        "ClusterProfile",
        "clusterprofiles",
        false,
    );
    palette(&mut app, "sveltos");
    assert_eq!(app.mode, Mode::Sveltos);
    (app, rx)
}

fn loaded() -> (App, Receiver<Msg>) {
    let (mut app, rx) = dashboard();
    feed(&mut app, fleet("t1"));
    (app, rx)
}

fn select(app: &mut App, needle: &str) {
    let rows = app.sveltos.display_rows();
    let index = rows
        .iter()
        .position(|row| row.contains(needle))
        .unwrap_or_else(|| panic!("no row contains {needle:?}: {rows:#?}"));
    app.handle_key(press(KeyCode::Char('g'))).unwrap();
    for _ in 0..index {
        app.handle_key(press(KeyCode::Char('j'))).unwrap();
    }
    assert_eq!(app.sveltos.selected(), index);
}

fn open(app: &mut App, needle: &str) {
    select(app, needle);
    app.handle_key(press(KeyCode::Enter)).unwrap();
}

/// Clusters → SveltosCluster team/west → its ClusterProfile base deployment.
fn open_d2(app: &mut App) {
    open(app, "SveltosCluster team west");
    open(app, "ClusterProfile base SveltosCluster");
}

fn filter(app: &mut App, text: &str) {
    app.handle_key(press(KeyCode::Char('/'))).unwrap();
    app.handle_key(ctrl(KeyCode::Char('u'))).unwrap();
    for character in text.chars() {
        app.handle_key(press(KeyCode::Char(character))).unwrap();
    }
    app.handle_key(press(KeyCode::Enter)).unwrap();
}

fn page_text(app: &App) -> String {
    [
        app.sveltos.context_lines(),
        app.sveltos.display_rows(),
        app.sveltos.detail_lines(),
    ]
    .concat()
    .join("\n")
}

fn list_client() -> (kube::Client, mpsc::UnboundedReceiver<String>) {
    let (sent, requests) = mpsc::unbounded_channel();
    let client = kube::Client::new(
        tower::service_fn(move |req: http::Request<kube::client::Body>| {
            let sent = sent.clone();
            async move {
                sent.send(req.uri().to_string()).unwrap();
                let body = json!({"apiVersion": "v1", "kind": "List", "metadata": {"continue": ""}, "items": []});
                Ok::<_, Infallible>(
                    http::Response::builder()
                        .status(200)
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(
                            body.to_string(),
                        )))
                        .unwrap(),
                )
            }
        }),
        "default",
    );
    (client, requests)
}

// The `:` palette also lists namespaces; keep only API-group requests.
fn group_requests(requests: &mut mpsc::UnboundedReceiver<String>) -> Vec<String> {
    std::iter::from_fn(|| requests.try_recv().ok())
        .filter(|path| path.starts_with("/apis/"))
        .collect()
}

async fn gathered(app: &mut App, rx: &mut Receiver<Msg>) {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("gather timed out")
            .expect("channel closed");
        if matches!(message, Msg::SveltosData { .. }) {
            app.handle_msg(message);
            return;
        }
    }
}

#[tokio::test]
async fn dashboard_reads_all_namespaces_under_partial_rbac_and_q_restores_the_table() {
    let (mut app, _rx) = test_app();
    app.cluster.register_kind(
        "config.projectsveltos.io",
        "ClusterProfile",
        "clusterprofiles",
        false,
    );
    app.namespace = "team-a".into();
    type_resource_query(&mut app, "pods");
    for name in ["db", "web-1", "web-2"] {
        apply(
            &mut app,
            json!({"apiVersion": "v1", "kind": "Pod", "metadata": {"name": name, "namespace": "team-a"}}),
        );
    }
    app.handle_msg(Msg::Synced {
        generation: app.generation,
    });
    type_filter(&mut app, "web");
    app.handle_key(press(KeyCode::Char('j'))).unwrap();
    let selected = || Some("web-2".to_owned());
    assert_eq!(app.selected().and_then(|pod| pod.metadata.name), selected());

    palette(&mut app, "sveltos");
    let mut reads = fleet("t1");
    fail(
        &mut reads,
        SourceKind::CapiCluster,
        SourceRead::failed("403 forbidden"),
    );
    feed(&mut app, reads);
    assert!(app.sveltos.status_lines()[0].contains("all namespaces"));
    assert!(
        app.sveltos.display_rows()[0].starts_with("SveltosCluster team west ready true Healthy")
    );
    let warning = app.sveltos.warning.clone().unwrap_or_default();
    assert!(
        warning.contains("CAPI Cluster unreadable: 403 forbidden"),
        "{warning}"
    );

    app.handle_key(press(KeyCode::Char('q'))).unwrap();
    assert_eq!(app.mode, Mode::Table);
    assert_eq!(
        (
            app.namespace.as_str(),
            app.kind_plural.as_str(),
            app.filter.as_str()
        ),
        ("team-a", "pods", "web")
    );
    assert_eq!(app.selected().and_then(|pod| pod.metadata.name), selected());
}

#[tokio::test]
async fn gather_lists_only_resolved_apis_and_reports_discovery_gaps() {
    let (mut app, mut rx) = test_app();
    let (client, mut requests) = list_client();
    app.cluster.client = client;
    app.cluster.register_kind(
        "config.projectsveltos.io",
        "ClusterProfile",
        "clusterprofiles",
        false,
    );
    app.cluster
        .discovery_warnings
        .push("lib.projectsveltos.io/v1beta1: discovery failed".into());
    palette(&mut app, "sveltos");
    gathered(&mut app, &mut rx).await;
    let paths = group_requests(&mut requests);
    assert!(
        matches!(paths.as_slice(), [path]
            if path.starts_with("/apis/config.projectsveltos.io/v1/clusterprofiles?")
                && path.contains("limit=500")),
        "{paths:?}"
    );
    let sources = app.sveltos.status_lines()[2].clone();
    for expected in [
        "SveltosCluster: unreadable",
        "CAPI Cluster: unavailable (not served)",
        "ClusterProfile: available",
        "Profile: unavailable (not served)",
    ] {
        assert!(sources.contains(expected), "{sources}");
    }

    let (mut app, mut rx) = test_app();
    let (client, mut requests) = list_client();
    app.cluster.client = client;
    app.cluster
        .discovery_warnings
        .push("config.projectsveltos.io/v1beta1: discovery failed".into());
    palette(&mut app, "sveltos");
    assert_eq!(app.mode, Mode::Sveltos);
    gathered(&mut app, &mut rx).await;
    assert_eq!(group_requests(&mut requests), Vec::<String>::new());
    assert!(app.sveltos.status_lines()[2].contains("ClusterProfile: unreadable"));
}

#[tokio::test]
async fn overview_tabs_show_evidence_and_keep_their_own_filter_and_selection() {
    let (mut app, _rx) = loaded();
    assert_eq!(app.sveltos.display_title(), "Sveltos · Clusters");
    assert_eq!(
        app.sveltos.display_rows(),
        [
            "SveltosCluster team west ready true Healthy  1 failed · 1 DryRun (not applied)",
            "SveltosCluster ops east ready true Healthy  1 failed",
            "CAPI Cluster team west unknown · control plane initialized not reported paused 1 provisioned",
            "SveltosCluster team old unknown not reported not observed 1 removing",
            "Unresolved - 1 deployments",
        ]
    );
    let status = app.sveltos.status_lines();
    assert_eq!(
        status[0],
        "management context · all namespaces · read-only · Sveltos v1.16.0 baseline"
    );
    assert_eq!(status[1], "Last successful read: t1 · idle");
    assert!(screen_text(&mut app, 160, 30).contains("Last successful read: t1"));

    select(&mut app, "CAPI Cluster team west");
    app.handle_key(press(KeyCode::Tab)).unwrap();
    assert_eq!(app.sveltos.display_title(), "Sveltos · Profiles");
    assert_eq!(app.sveltos.selected(), 0);
    let headings = app.sveltos.table_data().0;
    assert!(headings.contains(&"Rollout (reported)") && headings.contains(&"Provisioning"));
    let rows = app.sveltos.display_rows();
    assert!(
        rows[0].starts_with(
            "ClusterProfile <cluster> base Continuous 3 updated 1 · updating 0 1 provisioned · 2 failed · 1 removing"
        ) && rows[0].ends_with("1 reported failures"),
        "{rows:#?}"
    );
    assert!(
        rows[1].contains("not set (default Continuous) 0 · no matching clusters"),
        "{rows:#?}"
    );
    assert!(
        rows[3].starts_with("Profile team base DryRun (not applied) 1 updated 0 · updating 0"),
        "{rows:#?}"
    );
    filter(&mut app, "dryrun");
    assert_eq!(app.sveltos.display_rows().len(), 1);

    for overlay in ['?', ':'] {
        app.handle_key(press(KeyCode::Char(overlay))).unwrap();
        app.handle_key(press(KeyCode::Esc)).unwrap();
        assert_eq!(app.mode, Mode::Sveltos);
        assert_eq!(app.sveltos.filter(), "dryrun");
    }
    app.handle_key(press(KeyCode::BackTab)).unwrap();
    assert_eq!(app.sveltos.display_title(), "Sveltos · Clusters");
    assert_eq!((app.sveltos.filter(), app.sveltos.selected()), ("", 2));
    app.handle_key(press(KeyCode::Tab)).unwrap();
    assert_eq!(app.sveltos.filter(), "dryrun");

    let mut reads = fleet("t2");
    fail(&mut reads, SourceKind::CapiCluster, SourceRead::absent());
    feed(&mut app, reads);
    assert!(app.sveltos.status_lines()[2].contains("CAPI Cluster: unavailable (not served)"));
    assert_eq!(app.sveltos.warning, None);
}

#[tokio::test]
async fn cluster_and_profile_drills_converge_on_one_deployment_detail() {
    let (mut app, _rx) = loaded();
    open(&mut app, "SveltosCluster team west");
    assert_eq!(
        app.sveltos.display_title(),
        "SveltosCluster team/west · deployments"
    );
    assert_eq!(app.sveltos.display_rows().len(), 3);
    open(&mut app, "ClusterProfile base SveltosCluster");
    assert_eq!(
        app.sveltos.display_title(),
        "ClusterProfile base → SveltosCluster team/west"
    );
    let via_cluster = app.sveltos.detail_lines();
    assert_eq!(
        via_cluster,
        [
            "Profile: ClusterProfile base",
            "Cluster: SveltosCluster team/west",
            "Relation: observed in profile matchingClusters",
            "Provisioning: failed",
            "Rollout (controller-reported): updated",
            "Sync mode: Continuous",
            "Dependencies: ClusterProfile prereq",
            "Next reconcile: 2025-01-01T00:10:00Z",
            "Features (raw controller states):",
            "  Kustomize: not configured",
            "  Helm: Provisioned",
            "    last applied: 2025-01-01T00:00:00Z",
            "  Resources: Failed",
            "    consecutive failures: 3",
            "    failure message: denied by webhook",
            "Helm release apps/app: Conflict",
            "    conflict: owned by Profile team/other",
            "Reported inventory: 3 reported entries (listed below)",
            "",
            "Downstream workload runtime health is not verified.",
        ]
    );

    app.handle_key(press(KeyCode::Esc)).unwrap();
    app.handle_key(press(KeyCode::Esc)).unwrap();
    app.handle_key(press(KeyCode::Tab)).unwrap();
    open(&mut app, "ClusterProfile <cluster> base");
    assert_eq!(app.sveltos.display_title(), "ClusterProfile base · targets");
    open(&mut app, "SveltosCluster team/west");
    assert_eq!(app.sveltos.detail_lines(), via_cluster);

    app.handle_key(press(KeyCode::Esc)).unwrap();
    open(&mut app, "CAPI Cluster team/west");
    assert!(app.sveltos.detail_lines().iter().any(|line| line
        == "Features report Provisioned, but the profile does not report this target updated; the latest configuration is not confirmed applied."));
}

#[tokio::test]
async fn related_lists_show_failures_removals_unresolved_and_declared_targeting() {
    let (mut app, _rx) = loaded();
    app.handle_key(press(KeyCode::Tab)).unwrap();
    open(&mut app, "ClusterProfile <cluster> base");
    assert_eq!(app.sveltos.display_rows().len(), 4);
    select(&mut app, "SveltosCluster team/old");
    assert!(app.sveltos.display_rows()[app.sveltos.selected()].contains("summary removing"));
    open(&mut app, "SveltosCluster ops/east");
    let text = page_text(&app);
    assert!(text.contains("summary creation failed: quota"), "{text}");
    assert!(text.contains("no ClusterSummary reported"), "{text}");

    app.handle_key(press(KeyCode::Esc)).unwrap();
    app.handle_key(press(KeyCode::Esc)).unwrap();
    open(&mut app, "selector");
    let text = page_text(&app);
    assert!(text.contains("Declared: clusterSelector"), "{text}");
    app.handle_key(press(KeyCode::Esc)).unwrap();
    open(&mut app, "empty");
    assert!(page_text(&app).contains("0 · no matching clusters"));
    assert!(app.sveltos.display_rows().is_empty());

    app.handle_key(press(KeyCode::Esc)).unwrap();
    open(&mut app, "Unresolved");
    assert_eq!(app.sveltos.display_title(), "Sveltos · unresolved evidence");
    open(&mut app, "ClusterProfile base (unresolved)");
    let text = page_text(&app);
    assert!(
        text.contains("Summary owner UID does not match current profile"),
        "{text}"
    );
}

#[tokio::test]
async fn filter_is_local_case_insensitive_text_and_escape_cancels_the_draft() {
    let (mut app, _rx) = loaded();
    let ids = (app.generation, app.sveltos_request);
    app.handle_key(press(KeyCode::Tab)).unwrap();
    open(&mut app, "ClusterProfile <cluster> base");
    filter(&mut app, "FAILED");
    let rows = app.sveltos.display_rows();
    assert_eq!(rows.len(), 2, "{rows:#?}");
    assert!(
        rows[0].contains("SveltosCluster ops/east") && rows[1].contains("SveltosCluster team/west")
    );

    app.handle_key(press(KeyCode::Char('/'))).unwrap();
    for character in "zzz".chars() {
        app.handle_key(press(KeyCode::Char(character))).unwrap();
    }
    app.handle_key(press(KeyCode::Esc)).unwrap();
    assert_eq!(app.mode, Mode::Sveltos);
    assert_eq!(app.sveltos.filter(), "FAILED");
    assert_eq!(app.sveltos.display_rows(), rows);

    filter(&mut app, "");
    assert_eq!(app.sveltos.display_rows().len(), 4);
    filter(&mut app, "-l env=prod");
    assert!(app.sveltos.display_rows().is_empty());
    assert!(!app.flash_err);
    assert_eq!((app.generation, app.sveltos_request), ids);
}

#[tokio::test]
async fn source_rows_open_read_only_yaml_and_mutating_keys_do_nothing() {
    let (mut app, _rx) = loaded();
    app.readonly = false;
    for key in [
        press(KeyCode::Char('e')),
        ctrl(KeyCode::Char('d')),
        press(KeyCode::Char('a')),
        press(KeyCode::Char('C')),
        press(KeyCode::Char('D')),
        press(KeyCode::Char('s')),
        press(KeyCode::Char('f')),
    ] {
        app.handle_key(key).unwrap();
        assert_eq!(app.mode, Mode::Sveltos, "{key:?}");
        assert!(app.pending.is_none(), "{key:?}");
    }

    open_d2(&mut app);
    select(&mut app, "source · ClusterSummary");
    let selected = app.sveltos.selected();
    app.handle_key(press(KeyCode::Char('y'))).unwrap();
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.detail.title, "base-sveltos-west — YAML");
    assert!(
        app.detail
            .lines
            .iter()
            .any(|line| line == "kind: ClusterSummary")
    );
    assert!(app.document_source.is_none());
    app.handle_key(press(KeyCode::Char('e'))).unwrap();
    assert!(app.pending.is_none() && app.document_edit_task.is_none());
    app.handle_key(press(KeyCode::Esc)).unwrap();
    assert_eq!(app.mode, Mode::Sveltos);
    assert_eq!(app.sveltos.selected(), selected);

    open(&mut app, "source · cluster");
    assert_eq!(app.detail.title, "west — YAML");
    app.handle_key(press(KeyCode::Esc)).unwrap();

    select(&mut app, "reported: Helm");
    for key in [KeyCode::Enter, KeyCode::Char('y')] {
        app.flash.clear();
        app.handle_key(press(key)).unwrap();
        assert_eq!(app.mode, Mode::Sveltos);
        assert!(
            app.flash.contains("reported inventory only"),
            "{}",
            app.flash
        );
    }

    app.handle_key(press(KeyCode::Esc)).unwrap();
    app.handle_key(press(KeyCode::Esc)).unwrap();
    open(&mut app, "SveltosCluster ops east");
    app.handle_key(press(KeyCode::Char('y'))).unwrap();
    assert_eq!(app.mode, Mode::Sveltos);
    assert!(
        app.flash.contains("no ClusterSummary reported"),
        "{}",
        app.flash
    );
}

#[tokio::test]
async fn abandoned_reads_and_polls_are_ignored_and_polling_waits_for_completion() {
    for action in ["r", "q", "pods", "ctx west"] {
        let (mut app, _rx) = loaded();
        app.all_contexts = vec!["test".into(), "west".into()];
        let (generation, request) = (app.generation, app.sveltos_request);
        match action {
            "r" => app.handle_key(press(KeyCode::Char('r'))).unwrap(),
            "q" => app.handle_key(press(KeyCode::Char('q'))).unwrap(),
            command => palette(&mut app, command),
        }
        assert_ne!(
            (app.generation, app.sveltos_request),
            (generation, request),
            "{action}"
        );
        let (last_success, current) = (app.sveltos.last_success.clone(), app.sveltos_request);
        app.handle_msg(Msg::SveltosData {
            generation,
            request,
            reads: fleet("late"),
        });
        app.handle_msg(Msg::SveltosPoll {
            generation,
            request,
        });
        assert_eq!(app.sveltos.last_success, last_success, "{action}");
        assert_eq!(app.sveltos_request, current, "{action}");
    }

    let (mut app, _rx) = dashboard();
    let (generation, request) = (app.generation, app.sveltos_request);
    app.handle_msg(Msg::SveltosPoll {
        generation,
        request,
    });
    assert_eq!(
        app.sveltos_request, request,
        "a poll must wait for the read"
    );
    feed(&mut app, fleet("t1"));
    app.handle_msg(Msg::SveltosPoll {
        generation,
        request,
    });
    assert_eq!(app.sveltos_request, request + 1);
    assert!(app.sveltos.refreshing);

    feed(&mut app, fleet("t2"));
    open_d2(&mut app);
    select(&mut app, "source · ClusterSummary");
    app.handle_key(press(KeyCode::Char('y'))).unwrap();
    assert_eq!(app.mode, Mode::Detail);
    app.handle_msg(Msg::SveltosPoll {
        generation,
        request: request + 1,
    });
    assert_eq!(app.sveltos_request, request + 2, "YAML over the dashboard");
}

#[tokio::test]
async fn open_pages_follow_or_explain_evidence_that_disappears() {
    let (mut app, _rx) = loaded();
    open_d2(&mut app);
    let mut reads = fleet("t2");
    remove(&mut reads, SourceKind::ClusterSummary, "base-sveltos-west");
    feed(&mut app, reads);
    let text = page_text(&app);
    assert!(text.contains("Provisioning: missing summary"), "{text}");
    assert!(
        text.contains("Features: no ClusterSummary reported"),
        "{text}"
    );
    let mut reads = fleet("t3");
    remove(&mut reads, SourceKind::ClusterSummary, "base-sveltos-west");
    remove(&mut reads, SourceKind::ClusterProfile, "base");
    feed(&mut app, reads);
    assert_eq!(
        app.sveltos.display_title(),
        "ClusterProfile base → SveltosCluster team/west"
    );
    let text = page_text(&app);
    assert!(
        text.contains("Deployment no longer reported by the management cluster."),
        "{text}"
    );

    let (mut app, _rx) = loaded();
    app.handle_key(press(KeyCode::Tab)).unwrap();
    open(&mut app, "selector");
    let mut reads = fleet("t2");
    remove(&mut reads, SourceKind::ClusterProfile, "selector");
    feed(&mut app, reads);
    let text = page_text(&app);
    assert!(
        text.contains(
            "no longer reported by the management cluster; no other profile is shown in its place."
        ),
        "{text}"
    );

    let (mut app, _rx) = loaded();
    select(&mut app, "CAPI Cluster team west");
    let mut reads = fleet("t2");
    remove(&mut reads, SourceKind::SveltosCluster, "east");
    feed(&mut app, reads);
    let rows = app.sveltos.display_rows();
    assert!(
        rows[app.sveltos.selected()].starts_with("CAPI Cluster team west"),
        "{rows:#?}"
    );
    assert!(
        rows.iter()
            .any(|row| row == "SveltosCluster ops east unknown not reported not observed 1 failed"),
        "{rows:#?}"
    );
}

#[tokio::test]
async fn failed_rereads_keep_stale_evidence_until_a_fresh_read() {
    let (mut app, _rx) = loaded();
    let mut reads = fleet("t2");
    fail(
        &mut reads,
        SourceKind::SveltosCluster,
        SourceRead::failed("connection refused"),
    );
    feed(&mut app, reads);
    let status = app.sveltos.status_lines();
    assert_eq!(status[1], "Last successful read: t2 · idle");
    assert!(
        status[2].starts_with("SveltosCluster: stale since t1"),
        "{status:#?}"
    );
    assert_eq!(
        app.sveltos.warning.as_deref(),
        Some("SveltosCluster stale since t1: connection refused")
    );
    assert!(
        app.sveltos.display_rows()[0].starts_with("SveltosCluster team west ready true Healthy")
    );
    feed(&mut app, fleet("t3"));
    assert_eq!(app.sveltos.warning, None);
    assert!(app.sveltos.status_lines()[2].starts_with("SveltosCluster: available"));

    let (mut app, _rx) = dashboard();
    let mut reads = fleet("t1");
    source(&mut reads, SourceKind::ClusterConfiguration).read =
        SourceRead::incomplete("20,000 object limit reached");
    feed(&mut app, reads);
    assert_eq!(
        app.sveltos.warning.as_deref(),
        Some("ClusterConfiguration incomplete: 20,000 object limit reached")
    );
    open_d2(&mut app);
    let text = page_text(&app);
    assert!(text.contains("(incomplete)"), "{text}");
    assert!(
        text.contains("reported: Resources · Local · core/v1 ConfigMap apps/settings · applied 2025-01-01T00:00:00Z"),
        "{text}"
    );
}

#[tokio::test]
async fn denied_inventory_is_reported_without_hiding_provisioning() {
    let (mut app, _rx) = dashboard();
    let mut reads = fleet("t1");
    fail(
        &mut reads,
        SourceKind::ClusterConfiguration,
        SourceRead::failed("403 forbidden"),
    );
    feed(&mut app, reads);
    open_d2(&mut app);
    let rows = app.sveltos.display_rows();
    assert!(
        rows.iter().any(|row| row.contains("ClusterConfiguration")
            && row.contains("unavailable: 403 forbidden")),
        "{rows:#?}"
    );
    assert!(!rows.iter().any(|row| row.starts_with("reported:")));
    let detail = app.sveltos.detail_lines();
    assert!(detail.iter().any(|line| line == "Provisioning: failed"));
    assert!(
        detail
            .iter()
            .any(|line| line.starts_with("Reported inventory:")
                && line.contains("unavailable: 403 forbidden")),
        "{detail:#?}"
    );
}

#[tokio::test]
async fn deployment_page_renders_evidence_and_scrolls_detail() {
    let (mut app, _rx) = loaded();
    open_d2(&mut app);
    let wide = screen_text(&mut app, 160, 50);
    assert!(wide.contains(" Evidence "), "{wide}");
    assert!(wide.contains("reported: Helm"), "{wide}");
    assert!(screen_text(&mut app, 60, 24).contains(" Evidence "));
    app.handle_key(press(KeyCode::PageDown)).unwrap();
    assert_eq!(app.sveltos.detail_scroll(), 10);
    app.handle_key(press(KeyCode::PageUp)).unwrap();
    assert_eq!(app.sveltos.detail_scroll(), 0);
    filter(&mut app, "zzz");
    assert!(screen_text(&mut app, 160, 50).contains("no rows match the filter"));
}
