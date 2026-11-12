use super::*;

impl App {
    /// Open the pulse / cluster-health dashboard (k9s `:pulse`).
    pub fn open_pulse(&mut self) {
        self.bump_generation();
        self.pulse = Pulse::default();
        self.mode = Mode::Pulse;
        self.spawn_pulse();
    }

    pub(super) fn spawn_pulse(&mut self) {
        let claim = self.claim_status("pulse — cluster health…");
        let resolve = |n: &str| self.cluster.resolve(n).map(|k| (k.ar, k.namespaced));
        let nodes = resolve("nodes");
        let pods = resolve("pods");
        let deploys = resolve("deployments");
        let sts = resolve("statefulsets");
        let ds = resolve("daemonsets");
        let jobs = resolve("jobs");
        let pvc = resolve("persistentvolumeclaims");

        let client = self.cluster.client.clone();
        let tx = self.tx.clone();
        let genr = self.generation;
        let flag = self.gen_flag.clone();
        // Cluster-health snapshot: always spans every namespace, regardless
        // of whatever namespace filter was active in the table view.
        let ns = String::new();

        let handle = tokio::spawn(async move {
            loop {
                if flag.load(Ordering::SeqCst) != genr {
                    break;
                }
                let mut p = Pulse::default();
                // A denied/failed list must not render as "0 healthy" tiles —
                // record it so the view can say the numbers are incomplete.
                let mut warn = None;

                if let Some((ar, _)) = &nodes {
                    let items = list_or_warn(&client, ar, false, "", &mut warn).await;
                    p.nodes_total = items.len();
                    p.nodes_ready = items.iter().filter(|o| node_ready(o)).count();
                }
                if let Some((ar, nsd)) = &pods {
                    let items = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await;
                    p.pods_total = items.len();
                    for o in &items {
                        match phase(o).as_str() {
                            "Running" => p.pods_running += 1,
                            "Pending" => p.pods_pending += 1,
                            "Failed" => p.pods_failed += 1,
                            "Succeeded" => p.pods_succeeded += 1,
                            _ => {}
                        }
                    }
                }
                if let Some((ar, nsd)) = &deploys {
                    let items = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await;
                    p.deploys_total = items.len();
                    p.deploys_ready = items
                        .iter()
                        .filter(|o| ready_eq(o, "/status/readyReplicas", "/spec/replicas"))
                        .count();
                }
                if let Some((ar, nsd)) = &sts {
                    let items = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await;
                    p.sts_total = items.len();
                    p.sts_ready = items
                        .iter()
                        .filter(|o| ready_eq(o, "/status/readyReplicas", "/spec/replicas"))
                        .count();
                }
                if let Some((ar, nsd)) = &ds {
                    let items = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await;
                    p.ds_total = items.len();
                    p.ds_ready = items
                        .iter()
                        .filter(|o| {
                            ready_eq(o, "/status/numberReady", "/status/desiredNumberScheduled")
                        })
                        .count();
                }
                if let Some((ar, nsd)) = &jobs {
                    p.jobs_total = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await.len();
                }
                if let Some((ar, nsd)) = &pvc {
                    let items = list_or_warn(&client, ar, *nsd, &ns, &mut warn).await;
                    p.pvc_total = items.len();
                    p.pvc_bound = items.iter().filter(|o| phase(o) == "Bound").count();
                }
                p.warn = warn;

                if tx
                    .send(Msg::PulseData {
                        generation: genr,
                        claim,
                        data: p,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
        self.tasks.push(handle);
    }

    /// Open the Karpenter dashboard (`:karpenter`).
    pub fn open_karpenter(&mut self) {
        let resolve = |n: &str| self.cluster.resolve(n).map(|k| (k.ar, k.namespaced));
        if resolve("nodepools").is_none() || resolve("nodeclaims").is_none() {
            self.flash_warn("Karpenter not installed");
            return;
        }
        self.bump_generation();
        self.karpenter = Default::default();
        self.karpenter_tab = 0;
        self.karpenter_filter.clear();
        self.karpenter_filtering = false;
        self.karpenter_pool = None;
        self.karpenter_return = None;
        self.karpenter_state.select(None);
        self.karpenter_updated = None;
        self.mode = Mode::Karpenter;
        self.record_view(crate::karpenter::VIEW);
        self.spawn_karpenter();
    }

    /// Back to the dashboard as it was left, reloading its data.
    pub(super) fn resume_karpenter(&mut self) {
        self.karpenter_return = None;
        self.mode = Mode::Karpenter;
        self.bump_generation();
        self.spawn_karpenter();
    }

    pub(super) fn spawn_karpenter(&mut self) {
        self.karpenter_loading = true;
        let claim = self.claim_status("karpenter…");
        let resolve = |n: &str| self.cluster.resolve(n).map(|k| (k.ar, k.namespaced));
        let kinds = ["nodepools", "nodeclaims", "nodes", "pods", "events"].map(resolve);
        // Each provider has its own NodeClass kind (EC2NodeClass, AKSNodeClass).
        let classes: Vec<_> = self
            .cluster
            .kind_plurals()
            .into_keys()
            .filter(|(kind, group)| kind.ends_with("nodeclass") && group.contains("karpenter"))
            .filter_map(|(kind, group)| self.cluster.resolve_in_group(&kind, &group))
            .map(|k| (5, Some((k.ar, k.namespaced)), String::new()))
            .collect();
        let client = self.cluster.client.clone();
        let tx = self.tx.clone();
        let genr = self.generation;
        let flag = self.gen_flag.clone();

        let handle = tokio::spawn(async move {
            loop {
                if flag.load(Ordering::SeqCst) != genr {
                    break;
                }
                // Pods and events are big: list concurrently and narrow them
                // server-side to live pods and the event reasons in use.
                let jobs = (0..3)
                    .map(|i| (i, kinds[i].clone(), String::new()))
                    .chain([(
                        3,
                        kinds[3].clone(),
                        "status.phase!=Succeeded,status.phase!=Failed".to_string(),
                    )])
                    .chain(
                        crate::karpenter::EVENT_REASONS
                            .iter()
                            .map(|r| (4, kinds[4].clone(), format!("reason={r}"))),
                    )
                    .chain(classes.iter().cloned());
                let results = futures_util::future::join_all(jobs.map(|(i, kind, fields)| {
                    let client = &client;
                    async move {
                        let Some((ar, nsd)) = kind else {
                            return (i, Vec::new(), None);
                        };
                        match list_kind_fields(client, &ar, nsd, "", &fields).await {
                            // List items come without a type; `y`/`d` need it.
                            Ok(mut items) => {
                                for o in &mut items {
                                    o.types.get_or_insert(kube::core::TypeMeta {
                                        api_version: ar.api_version.clone(),
                                        kind: ar.kind.clone(),
                                    });
                                }
                                (i, items, None)
                            }
                            Err(e) => (i, Vec::new(), Some(e)),
                        }
                    }
                }))
                .await;
                let mut lists: [Vec<Value>; 6] = Default::default();
                let (mut warn, mut events_warn) = (None, None);
                for (i, items, w) in results {
                    lists[i].extend(items.iter().filter_map(|o| serde_json::to_value(o).ok()));
                    let slot = if i == 4 { &mut events_warn } else { &mut warn };
                    if slot.is_none() {
                        *slot = w;
                    }
                }
                let items = crate::karpenter::report(&crate::karpenter::Inputs {
                    pools: &lists[0],
                    claims: &lists[1],
                    nodes: &lists[2],
                    pods: &lists[3],
                    events: &lists[4],
                    events_ok: events_warn.is_none() && kinds[4].is_some(),
                    classes: &lists[5],
                });
                if tx
                    .send(Msg::KarpenterData {
                        generation: genr,
                        claim,
                        items,
                        warn,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(crate::karpenter::REFRESH_SECS)).await;
            }
        });
        self.tasks.push(handle);
    }

    /// Rows of the active tab that match the filter, in the chosen order.
    pub fn karpenter_rows(&self) -> Vec<&crate::karpenter::Row> {
        use crate::explain::Level;
        let searching = !self.karpenter_filter.is_empty();
        let mut rows: Vec<_> = self.karpenter.tabs[self.karpenter_tab]
            .iter()
            .filter(|r| match (&self.karpenter_pool, self.karpenter_tab) {
                (None, _) => true,
                // Pending pods have no pool, so the focus skips them.
                (_, 2) => true,
                (Some(p), crate::karpenter::CLASSES) => r.cells[2].split(", ").any(|c| c == p),
                (p, _) => r.pool.as_ref() == p.as_ref(),
            })
            .filter(|r| {
                if searching {
                    r.matches(&self.karpenter_filter)
                } else {
                    (self.karpenter_show_idle || !r.idle)
                        && (!self.karpenter_problems
                            || !matches!(r.level, Level::Good | Level::Info))
                }
            })
            .collect();
        let (col, desc) = self.karpenter_sort[self.karpenter_tab];
        if col > 0 {
            let header = crate::karpenter::TABS[self.karpenter_tab].1[col - 1];
            rows.sort_by(|a, b| a.cmp_col(b, col - 1, header));
        } else {
            // Worst first; stable, so report order breaks ties.
            rows.sort_by_key(|r| std::cmp::Reverse(crate::karpenter::rank(r.level)));
        }
        if desc {
            rows.reverse();
        }
        rows
    }

    /// Keep the cursor valid after the row set changed.
    pub(super) fn karpenter_reselect(&mut self) {
        let len = self.karpenter_rows().len();
        let sel = self
            .karpenter_state
            .selected()
            .unwrap_or(0)
            .min(len.saturating_sub(1));
        self.karpenter_state.select((len > 0).then_some(sel));
    }

    fn karpenter_switch(&mut self, next: bool) {
        let n = self.karpenter.tabs.len();
        self.karpenter_tab = (self.karpenter_tab + if next { 1 } else { n - 1 }) % n;
        self.karpenter_state.select(Some(0));
        self.karpenter_reselect();
    }

    /// NodePools tab with `pool` selected, leaving the pool focus and search.
    fn karpenter_show_pool(&mut self, pool: &str) {
        self.karpenter_tab = 0;
        self.karpenter_pool = None;
        self.karpenter_filter.clear();
        let at = self
            .karpenter_rows()
            .iter()
            .position(|r| r.cells[0] == pool);
        self.karpenter_state.select(Some(at.unwrap_or(0)));
        self.karpenter_reselect();
    }

    pub(super) fn key_karpenter(&mut self, key: KeyInput) {
        if self.karpenter_filtering {
            if edit_action(key.action, &mut self.karpenter_filter) {
            } else {
                match (key.action, key.code) {
                    (Some(Action::Back), _) => {
                        self.karpenter_filter.clear();
                        self.karpenter_filtering = false;
                    }
                    (Some(Action::Accept), _) => self.karpenter_filtering = false,
                    (Some(Action::Backspace), _) => {
                        self.karpenter_filter.pop();
                    }
                    (None, KeyCode::Char(c)) if key.is_text() => self.karpenter_filter.push(c),
                    _ => {}
                }
            }
            self.karpenter_state.select(Some(0));
            self.karpenter_reselect();
            return;
        }
        let len = self.karpenter_rows().len();
        let cur = self.karpenter_state.selected().unwrap_or(0);
        match (key.action, key.code) {
            (Some(Action::Back), _) if !self.karpenter_filter.is_empty() => {
                self.karpenter_filter.clear();
                self.karpenter_reselect();
            }
            (Some(Action::Back), _) if self.karpenter_pool.is_some() => {
                let pool = self.karpenter_pool.take().unwrap_or_default();
                self.karpenter_show_pool(&pool);
            }
            (Some(Action::Back), _) | (Some(Action::Close), _) => {
                self.mode = Mode::Table;
                self.record_history();
                self.start_watch();
            }
            (Some(Action::HistoryBack), _) => {
                self.mode = Mode::Table;
                self.history_back();
            }
            (Some(Action::HistoryForward), _) => {
                self.mode = Mode::Table;
                self.history_forward();
            }
            (Some(Action::Filter), _) => self.karpenter_filtering = true,
            (Some(Action::Sort), _) => {
                self.sort_picker_karpenter = true;
                self.sort_picker_filter.clear();
                self.sort_picker_state
                    .select(Some(self.karpenter_sort[self.karpenter_tab].0));
                self.mode = Mode::SortPicker;
            }
            (Some(Action::SortAge), _) => self.sort_by_age(),
            (Some(Action::InvertSort), _) => {
                let t = self.karpenter_tab;
                self.karpenter_sort[t].1 = !self.karpenter_sort[t].1;
                self.karpenter_reselect();
            }
            (Some(Action::Faults), _) => {
                self.karpenter_problems = !self.karpenter_problems;
                self.karpenter_reselect();
            }
            (Some(Action::Toggle), _) => {
                self.karpenter_show_idle = !self.karpenter_show_idle;
                self.karpenter_reselect();
            }
            (Some(Action::NextView), _) => self.karpenter_switch(true),
            (Some(Action::PreviousView), _) => self.karpenter_switch(false),
            (Some(Action::Down), _) if len > 0 => {
                self.karpenter_state.select(Some((cur + 1).min(len - 1)))
            }
            (Some(Action::Up), _) if len > 0 => {
                self.karpenter_state.select(Some(cur.saturating_sub(1)))
            }
            (Some(Action::First), _) if len > 0 => self.karpenter_state.select(Some(0)),
            (Some(Action::Last), _) if len > 0 => self.karpenter_state.select(Some(len - 1)),
            (Some(Action::Refresh), _) => {
                self.bump_generation();
                self.spawn_karpenter();
            }
            (Some(Action::Yaml | Action::Describe), _) => {
                let row = self.karpenter_rows().get(cur).copied().cloned();
                let obj = row
                    .as_ref()
                    .and_then(|r| r.object.clone())
                    .and_then(|v| serde_json::from_value::<DynamicObject>(v).ok());
                let Some(obj) = obj else {
                    // Disruption events outlive the nodes and claims they name.
                    let why = match row.and_then(|r| r.target) {
                        Some(t) => format!("{} {} no longer exists", t.plural, t.name),
                        None => "no object on this row".into(),
                    };
                    self.flash_warn(&why);
                    return;
                };
                self.set_return_mode();
                if key.action == Some(Action::Yaml) {
                    self.show_yaml(&obj);
                } else {
                    let resource = obj
                        .types
                        .as_ref()
                        .and_then(|t| {
                            let group = t.api_version.rsplit_once('/').map_or("", |g| g.0);
                            self.cluster.resolve_in_group(&t.kind.to_lowercase(), group)
                        })
                        .map(|k| match k.ar.group.as_str() {
                            "" => k.ar.plural,
                            g => format!("{}.{g}", k.ar.plural),
                        })
                        .unwrap_or_else(|| {
                            obj.types
                                .as_ref()
                                .map(|t| t.kind.to_lowercase())
                                .unwrap_or_default()
                        });
                    self.describe_object(resource, &obj);
                }
            }
            (Some(Action::Accept), _) if self.karpenter_tab == 0 => {
                if let Some(pool) = self.karpenter_rows().get(cur).map(|r| r.cells[0].clone()) {
                    self.karpenter_tab = 1;
                    self.karpenter_pool = Some(pool);
                    self.karpenter_filter.clear();
                    self.karpenter_state.select(Some(0));
                    self.karpenter_reselect();
                }
            }
            (Some(Action::Accept), _) => {
                let row = self.karpenter_rows().get(cur).copied().cloned();
                // A NodeClaim row descends to its node once it has one.
                let target = row.and_then(|r| match (r.target, r.node) {
                    (Some(t), Some(name)) if t.plural == "nodeclaims" => {
                        Some(crate::explain::Target {
                            plural: "nodes".into(),
                            namespace: None,
                            name,
                        })
                    }
                    (t, _) => t,
                });
                match target {
                    Some(t) => {
                        self.navigate_to_target(&t);
                        self.karpenter_return = Some(self.stack.len());
                    }
                    None => self.flash_warn("no resource to jump to on this row"),
                }
            }
            _ => {}
        }
    }

    pub(super) fn key_pulse(&mut self, key: KeyInput) {
        match (key.action, key.code) {
            (Some(Action::Back), _) | (Some(Action::Close), _) => {
                self.mode = Mode::Table;
                self.start_watch();
            }
            (Some(Action::Refresh), _) => {
                self.bump_generation();
                self.spawn_pulse();
            }
            _ => {}
        }
    }

    /// Open the xray tree for the current kind (owner → children → containers).
    pub fn open_xray(&mut self) {
        if self.kind.is_none() {
            self.flash_warn("select a resource first");
            return;
        }
        self.bump_generation();
        self.xray_items.clear();
        self.xray_state.select(Some(0));
        self.mode = Mode::Xray;
        self.spawn_xray();
    }

    pub(super) fn spawn_xray(&mut self) {
        let claim = self.claim_status(format!("xray: {}…", self.kind_plural));
        let Some((root_ar, root_nsd)) = self.kind.as_ref().map(|k| (k.ar.clone(), k.namespaced))
        else {
            return;
        };
        let root_kind = trim_s(&self.kind_plural).to_string();
        let pool_kinds: Vec<(String, ApiResource, bool)> = xray_pool_plurals(&root_kind)
            .iter()
            .filter_map(|plural| {
                self.cluster
                    .resolve(plural)
                    .map(|k| (trim_s(plural).to_string(), k.ar, k.namespaced))
            })
            .collect();

        let client = self.cluster.client.clone();
        let tx = self.tx.clone();
        let genr = self.generation;
        let flag = self.gen_flag.clone();
        let namespaces = self.watch_namespaces();

        let handle = tokio::spawn(async move {
            loop {
                if flag.load(Ordering::SeqCst) != genr {
                    break;
                }
                let mut warn = None;
                let mut roots = Vec::new();
                for ns in if root_nsd {
                    namespaces.clone()
                } else {
                    vec![String::new()]
                } {
                    roots.extend(list_or_warn(&client, &root_ar, root_nsd, &ns, &mut warn).await);
                }
                let mut pool: Vec<(String, DynamicObject)> = Vec::new();
                for (label, ar, namespaced) in &pool_kinds {
                    for ns in if *namespaced {
                        namespaces.clone()
                    } else {
                        vec![String::new()]
                    } {
                        for o in list_or_warn(&client, ar, *namespaced, &ns, &mut warn).await {
                            pool.push((label.clone(), o));
                        }
                    }
                }

                // Index children by owner uid.
                let mut children: HashMap<String, Vec<(String, DynamicObject)>> = HashMap::new();
                for (label, o) in &pool {
                    if let Some(owners) = &o.metadata.owner_references {
                        for owner in owners {
                            children
                                .entry(owner.uid.clone())
                                .or_default()
                                .push((label.clone(), o.clone()));
                        }
                    }
                }

                let mut items = Vec::new();
                for root in &roots {
                    emit_xray(&root_kind, root, 0, &children, &mut items);
                }

                if tx
                    .send(Msg::XrayData {
                        generation: genr,
                        claim,
                        items,
                        warn,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
        self.tasks.push(handle);
    }

    pub(super) fn key_xray(&mut self, key: KeyInput) {
        let len = self.xray_items.len();
        match (key.action, key.code) {
            (Some(Action::Back), _) | (Some(Action::Close), _) => {
                self.mode = Mode::Table;
                self.start_watch();
            }
            (Some(Action::Down), _) => list_step(&mut self.xray_state, len, true),
            (Some(Action::Up), _) => list_step(&mut self.xray_state, len, false),
            (Some(Action::First), _) => {
                if len > 0 {
                    self.xray_state.select(Some(0));
                }
            }
            (Some(Action::Last), _) => {
                if len > 0 {
                    self.xray_state.select(Some(len - 1));
                }
            }
            // Enter on a pod/container streams logs.
            (Some(Action::Logs), _) => {
                if let Some(i) = self.xray_state.selected()
                    && let Some(item) = self.xray_items.get(i).cloned()
                {
                    match item.kind.as_str() {
                        "container" => self.launch_logs(
                            LogSource::Single {
                                ns: item.ns,
                                pod: item.name.clone(),
                                container: item.container.clone(),
                                previous: false,
                            },
                            format!(
                                "{}:{} — logs",
                                item.name,
                                item.container.unwrap_or_default()
                            ),
                        ),
                        "pod" => {
                            if let Some(pod) = item.pod {
                                let (source, title) = logs::pod_log_source(&pod);
                                self.launch_logs(source, title);
                            }
                        }
                        _ => self.flash_warn("logs available on pods/containers"),
                    }
                }
            }
            (Some(Action::Refresh), _) => {
                self.bump_generation();
                self.spawn_xray();
            }
            _ => {}
        }
    }
}
