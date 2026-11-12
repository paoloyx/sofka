//! Karpenter dashboard: turns raw cluster objects into table rows per tab.
//!
//! Pure: takes JSON objects and returns [`Report`], so it is tested without a
//! cluster. Usage is the sum of NodeClaim `status.capacity` per pool, the same
//! basis Karpenter uses for `spec.limits`.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::columns::{fmt_cpu, fmt_mem, parse_cpu_milli, parse_mem_bytes};
use crate::explain::{Level, Target};

pub struct Inputs<'a> {
    pub pools: &'a [Value],
    pub claims: &'a [Value],
    pub nodes: &'a [Value],
    pub pods: &'a [Value],
    pub events: &'a [Value],
    pub events_ok: bool,
    /// Provider NodeClasses (EC2NodeClass, AKSNodeClass, …), any kind.
    pub classes: &'a [Value],
}

/// Tab titles and column headers, in tab order.
/// History sentinel for the dashboard.
pub const VIEW: &str = "karpenter";

/// Seconds between background refreshes of the dashboard.
pub const REFRESH_SECS: u64 = 5;

pub const TABS: [(&str, &[&str]); 5] = [
    (
        "NodePools",
        &[
            "NAME",
            "CLAIMS",
            "CPU",
            "MEMORY",
            "NODES",
            "BUDGET",
            "MODE",
            "NODECLASS",
        ],
    ),
    (
        "NodeClaims",
        &[
            "NAME", "POOL", "TYPE", "INSTANCE", "CPU", "MEMORY", "NODE", "AGE", "STATE",
        ],
    ),
    ("Pending pods", &["POD", "AGE", "WHY"]),
    ("Disruption", &["OBJECT", "KIND", "STATE", "AGE", "MESSAGE"]),
    (
        "NodeClasses",
        &["NAME", "KIND", "POOLS", "CLAIMS", "STATE", "AGE"],
    ),
];

/// NodeClasses tab, whose rows serve several pools (listed in POOLS).
pub const CLASSES: usize = 4;

#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<String>,
    pub level: Level,
    pub target: Option<Target>,
    /// Lines shown in the detail pane when the row is selected.
    pub detail: Vec<String>,
    /// Pool without NodeClaims; hidden by default.
    pub idle: bool,
    /// Node the row runs on or names, the target of `o`.
    pub node: Option<String>,
    /// NodePool the row belongs to, the key of the pool focus.
    pub pool: Option<String>,
    /// Seconds since creation or last event, the sort key of AGE columns.
    pub age: Option<i64>,
    /// The object the row targets, for `y` and `d`.
    pub object: Option<Value>,
}

impl Row {
    /// Case-insensitive substring match over every cell.
    pub fn matches(&self, q: &str) -> bool {
        let q = q.to_lowercase();
        self.cells.iter().any(|c| c.to_lowercase().contains(&q))
    }

    /// Order by column `col`: AGE by seconds, cells starting with a number
    /// (after any usage bar) numerically, anything else as text.
    pub fn cmp_col(&self, other: &Row, col: usize, header: &str) -> std::cmp::Ordering {
        if header == "AGE" {
            return self.age.cmp(&other.age);
        }
        let num = |r: &Row| {
            let t = r.cells[col].trim_start_matches(['█', '░', ' ']);
            let n: String = t
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            n.parse::<f64>().ok()
        };
        match (num(self), num(other)) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ => self.cells[col].cmp(&other.cells[col]),
        }
    }
}

fn age_of(v: &Value, paths: &[&str], now: i64) -> Option<i64> {
    paths.iter().find_map(|p| {
        let ts = s(v, p).parse::<jiff::Timestamp>().ok()?;
        Some((now - ts.as_second()).max(0))
    })
}

fn age_cell(age: Option<i64>) -> String {
    age.map_or_else(|| "-".into(), crate::columns::humanize)
}

const CREATED: &[&str] = &["/metadata/creationTimestamp"];
const EVENT_TS: &[&str] = &[
    "/series/lastObservedTime",
    "/lastTimestamp",
    "/eventTime",
    "/deprecatedLastTimestamp",
];

#[derive(Debug, Clone, Default)]
pub struct Report {
    pub tabs: [Vec<Row>; 5],
    pub not_ready: usize,
    pub events_ok: bool,
}

const DO_NOT_DISRUPT: &str = "karpenter.sh/do-not-disrupt";
const POOL_LABEL: &str = "karpenter.sh/nodepool";

fn s<'a>(v: &'a Value, p: &str) -> &'a str {
    v.pointer(p).and_then(Value::as_str).unwrap_or("")
}

fn name(v: &Value) -> &str {
    s(v, "/metadata/name")
}

fn label<'a>(v: &'a Value, k: &str) -> &'a str {
    v.pointer("/metadata/labels")
        .and_then(|l| l.get(k))
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn target(plural: &str, ns: Option<&str>, n: &str) -> Target {
    Target {
        plural: plural.into(),
        namespace: ns.filter(|n| !n.is_empty()).map(str::to_string),
        name: n.into(),
    }
}

/// Conditions whose status is not `True`, as `Type: reason - message`.
fn bad_conditions(v: &Value, unknown_ok: bool) -> Vec<String> {
    let conds = v.pointer("/status/conditions").and_then(Value::as_array);
    conds
        .into_iter()
        .flatten()
        .filter(|c| match s(c, "/status") {
            "True" => false,
            "Unknown" => !unknown_ok,
            _ => true,
        })
        .map(|c| {
            let msg = s(c, "/message");
            let sep = if msg.is_empty() { "" } else { " - " };
            format!("{}: {}{sep}{msg}", s(c, "/type"), s(c, "/reason"))
        })
        .collect()
}

fn claim_ready(c: &Value) -> bool {
    bad_conditions(c, false).is_empty() && !s(c, "/status/nodeName").is_empty()
}

fn is_deleting(c: &Value) -> bool {
    c.pointer("/metadata/deletionTimestamp").is_some()
}

#[derive(Default)]
struct Usage {
    cpu: i64,
    mem: i64,
    claims: usize,
    deleting: usize,
    modes: BTreeMap<String, usize>,
}

fn usage_of(claims: &[&Value]) -> Usage {
    let mut u = Usage::default();
    for c in claims {
        u.claims += 1;
        u.deleting += usize::from(is_deleting(c));
        u.cpu += parse_cpu_milli(s(c, "/status/capacity/cpu"));
        u.mem += parse_mem_bytes(s(c, "/status/capacity/memory"));
        let mode = label(c, "karpenter.sh/capacity-type");
        *u.modes.entry(mode.to_string()).or_default() += 1;
    }
    u
}

fn level_for(pct: i64) -> Level {
    match pct {
        100.. => Level::Critical,
        80.. => Level::Warn,
        _ => Level::Good,
    }
}

pub(crate) fn rank(l: Level) -> u8 {
    match l {
        Level::Critical => 2,
        Level::Warn => 1,
        _ => 0,
    }
}

fn pct(used: i64, limit: i64) -> i64 {
    if limit <= 0 { 0 } else { used * 100 / limit }
}

/// `█████░ 83% 20/24`: usage bar, percentage, then used/limit.
fn bar(p: i64, used: &str, limit: &str) -> String {
    let filled = (p.clamp(0, 100) * 6 / 100) as usize;
    format!(
        "{}{} {p}% {used}/{limit}",
        "█".repeat(filled),
        "░".repeat(6 - filled)
    )
}

struct Limit {
    kind: &'static str,
    pct: i64,
    cell: String,
}

fn limits_of(pool: &Value, u: &Usage) -> Vec<Limit> {
    let Some(limits) = pool.pointer("/spec/limits").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (k, v) in limits {
        let raw = match v {
            Value::String(s) => s.clone(),
            o => o.to_string(),
        };
        let (kind, p, used, limit) = match k.as_str() {
            "cpu" => {
                let l = parse_cpu_milli(&raw);
                ("cpu", pct(u.cpu, l), fmt_cpu(u.cpu), fmt_cpu(l))
            }
            "memory" => {
                let l = parse_mem_bytes(&raw);
                ("memory", pct(u.mem, l), fmt_mem(u.mem), fmt_mem(l))
            }
            "nodes" => {
                let l = raw.parse::<i64>().unwrap_or(0);
                let n = u.claims as i64;
                ("nodes", pct(n, l), n.to_string(), raw)
            }
            _ => continue,
        };
        out.push(Limit {
            kind,
            pct: p,
            cell: bar(p, &used, &limit),
        });
    }
    out
}

fn budgets(pool: &Value, claims: usize) -> Vec<String> {
    let list = pool
        .pointer("/spec/disruption/budgets")
        .and_then(Value::as_array);
    let Some(list) = list.filter(|l| !l.is_empty()) else {
        return vec![format!("10% (allows {}, default)", claims.div_ceil(10))];
    };
    list.iter()
        .map(|b| {
            let n = s(b, "/nodes");
            let allowed = match n.strip_suffix('%') {
                Some(p) => (p.parse::<usize>().unwrap_or(0) * claims).div_ceil(100),
                None => n.parse().unwrap_or(0),
            };
            format!("{n} (allows {allowed})")
        })
        .collect()
}

fn mix(u: &Usage) -> String {
    u.modes
        .iter()
        .map(|(m, n)| format!("{} {n}", if m.is_empty() { "?" } else { m }))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn report(i: &Inputs) -> Report {
    let now = crate::columns::now_secs();
    let mut by_pool: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for c in i.claims {
        by_pool.entry(label(c, POOL_LABEL)).or_default().push(c);
    }
    let usages: BTreeMap<&str, Usage> = by_pool.iter().map(|(k, v)| (*k, usage_of(v))).collect();
    let none = Usage::default();

    let mut pools: Vec<(Level, usize, Row)> = i
        .pools
        .iter()
        .map(|p| {
            let n = name(p);
            let u = usages.get(n).unwrap_or(&none);
            let limits = limits_of(p, u);
            let mut bad = bad_conditions(p, true);
            let class = s(p, "/spec/template/spec/nodeClassRef/name");
            match i.classes.iter().find(|c| name(c) == class) {
                Some(c) => bad.extend(
                    bad_conditions(c, false)
                        .first()
                        .map(|b| format!("nodeclass {class}: {b}")),
                ),
                None if !i.classes.is_empty() && !class.is_empty() => {
                    bad.push(format!("nodeclass {class} not found"));
                }
                None => {}
            }
            let level = if bad.is_empty() {
                limits
                    .iter()
                    .map(|l| level_for(l.pct))
                    .max_by_key(|l| rank(*l))
                    .unwrap_or(Level::Good)
            } else {
                Level::Critical
            };
            let cell = |k: &str, fallback: String| {
                limits
                    .iter()
                    .find(|l| l.kind == k)
                    .map_or(fallback, |l| l.cell.clone())
            };
            let budget = budgets(p, u.claims - u.deleting);
            let mut detail = vec![format!(
                "policy: {}, after {}",
                s(p, "/spec/disruption/consolidationPolicy"),
                s(p, "/spec/disruption/consolidateAfter")
            )];
            detail.extend(budget.iter().map(|b| format!("budget: {b}")));
            if limits.is_empty() {
                detail.push("limits: none set".into());
            }
            detail.push(format!("capacity mix: {}", mix(u)));
            if !class.is_empty() {
                let kind = s(p, "/spec/template/spec/nodeClassRef/kind");
                detail.push(format!("nodeclass: {kind} {class}"));
            }
            detail.extend(bad.iter().map(|b| format!("{PROBLEM}{b}")));
            for c in by_pool.get(n).into_iter().flatten() {
                let bad = bad_conditions(c, false);
                if !bad.is_empty() {
                    detail.push(format!("{PROBLEM}claim {}: {}", name(c), bad.join("; ")));
                }
            }
            let row = Row {
                cells: vec![
                    n.into(),
                    u.claims.to_string(),
                    cell("cpu", format!("{} · no limit", fmt_cpu(u.cpu))),
                    cell("memory", format!("{} · no limit", fmt_mem(u.mem))),
                    cell("nodes", u.claims.to_string()),
                    budget.join(", "),
                    mix(u),
                    class.into(),
                ],
                level,
                target: Some(target("nodepools", None, n)),
                detail,
                idle: u.claims == 0,
                node: None,
                pool: Some(n.to_string()),
                age: None,
                object: None,
            };
            (level, u.claims, row)
        })
        .collect();
    pools.sort_by_key(|(l, n, _)| (std::cmp::Reverse(rank(*l)), std::cmp::Reverse(*n)));

    let mut claims: Vec<Row> = i.claims.iter().map(|c| claim_row(c, now)).collect();
    claims.sort_by_key(|r| std::cmp::Reverse(rank(r.level)));

    let full: Vec<&str> = i
        .pools
        .iter()
        .filter(|p| {
            limits_of(p, usages.get(name(p)).unwrap_or(&none))
                .iter()
                .any(|l| l.pct >= 100)
        })
        .map(name)
        .collect();

    let mut tabs = [
        pools.into_iter().map(|p| p.2).collect(),
        claims,
        pending_pods(i, &full, now),
        disruption(i, now),
        classes(i, now),
    ];
    attach_objects(i, &mut tabs);
    // Events outlive the nodes and claims they name; drop rows for deleted
    // objects. An empty list may be a failed one, so it never hides rows.
    tabs[3].retain(|r| match r.target.as_ref().map(|t| t.plural.as_str()) {
        Some("nodes") if !i.nodes.is_empty() => r.object.is_some(),
        Some("nodeclaims") if !i.claims.is_empty() => r.object.is_some(),
        _ => true,
    });
    Report {
        not_ready: i.claims.iter().filter(|c| !claim_ready(c)).count(),
        events_ok: i.events_ok,
        tabs,
    }
}

/// One row per NodeClass: the pools and claims using it and whether the
/// provider could resolve it (AMIs, subnets, security groups, role).
fn classes(i: &Inputs, now: i64) -> Vec<Row> {
    let mut out: Vec<Row> = i
        .classes
        .iter()
        .map(|c| {
            let n = name(c);
            let pools: Vec<&str> = i
                .pools
                .iter()
                .filter(|p| s(p, "/spec/template/spec/nodeClassRef/name") == n)
                .map(name)
                .collect();
            let claims = i
                .claims
                .iter()
                .filter(|cl| s(cl, "/spec/nodeClassRef/name") == n)
                .count();
            let bad = bad_conditions(c, false);
            let (level, state) = if is_deleting(c) {
                (Level::Info, "Deleting".to_string())
            } else if let Some(b) = bad.first() {
                (Level::Critical, format!("NotReady: {b}"))
            } else {
                (Level::Good, "Ready".into())
            };
            let resolved = ["amis", "subnets", "securityGroups"]
                .iter()
                .filter_map(|k| {
                    let n = c.pointer(&format!("/status/{k}"))?.as_array()?.len();
                    Some(format!("{n} {k}"))
                })
                .collect::<Vec<_>>();
            let mut detail: Vec<String> = [
                ("role", s(c, "/spec/role")),
                ("instance profile", s(c, "/status/instanceProfile")),
            ]
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| format!("{k}: {v}"))
            .collect();
            if !resolved.is_empty() {
                detail.push(format!("resolved: {}", resolved.join(", ")));
            }
            detail.extend(bad.iter().map(|b| format!("{PROBLEM}{b}")));
            let kind = s(c, "/kind");
            let age = age_of(c, CREATED, now);
            Row {
                cells: vec![
                    n.into(),
                    kind.into(),
                    pools.join(", "),
                    claims.to_string(),
                    state,
                    age_cell(age),
                ],
                level,
                target: Some(target(&kind.to_lowercase(), None, n)),
                detail,
                idle: false,
                node: None,
                pool: None,
                age,
                object: None,
            }
        })
        .collect();
    out.sort_by_key(|r| std::cmp::Reverse(rank(r.level)));
    out
}

/// Give every row the object it targets, looked up among the inputs.
fn attach_objects(i: &Inputs, tabs: &mut [Vec<Row>]) {
    let mut by_key: std::collections::HashMap<(String, &str, &str), &Value> =
        std::collections::HashMap::new();
    let lists = [
        ("nodepools", i.pools),
        ("nodeclaims", i.claims),
        ("nodes", i.nodes),
        ("pods", i.pods),
    ];
    for (plural, list) in lists {
        for v in list {
            let ns = s(v, "/metadata/namespace");
            by_key.insert((plural.to_string(), ns, name(v)), v);
        }
    }
    for v in i.classes {
        by_key.insert((s(v, "/kind").to_lowercase(), "", name(v)), v);
    }
    for r in tabs.iter_mut().flatten() {
        if let Some(t) = &r.target {
            let ns = t.namespace.as_deref().unwrap_or("");
            r.object = by_key
                .get(&(t.plural.clone(), ns, t.name.as_str()))
                .map(|v| (*v).clone());
        }
    }
}

fn claim_row(c: &Value, now: i64) -> Row {
    let bad = bad_conditions(c, false);
    let (level, state) = if is_deleting(c) {
        (Level::Info, "Deleting".to_string())
    } else if !bad.is_empty() {
        (Level::Warn, format!("NotReady: {}", bad[0]))
    } else if s(c, "/status/nodeName").is_empty() {
        (Level::Warn, "Launching".into())
    } else {
        (Level::Good, "Ready".into())
    };
    let n = name(c);
    let age = age_of(c, CREATED, now);
    Row {
        cells: vec![
            n.into(),
            label(c, POOL_LABEL).into(),
            label(c, "karpenter.sh/capacity-type").into(),
            label(c, "node.kubernetes.io/instance-type").into(),
            fmt_cpu(parse_cpu_milli(s(c, "/status/capacity/cpu"))),
            fmt_mem(parse_mem_bytes(s(c, "/status/capacity/memory"))),
            s(c, "/status/nodeName").into(),
            age_cell(age),
            state,
        ],
        level,
        target: Some(target("nodeclaims", None, n)),
        detail: claim_detail(c, &bad),
        idle: false,
        node: Some(s(c, "/status/nodeName").to_string()).filter(|n| !n.is_empty()),
        pool: Some(label(c, POOL_LABEL).to_string()).filter(|p| !p.is_empty()),
        age,
        object: None,
    }
}

/// Prefix of detail lines that describe a problem; the UI colours them.
pub const PROBLEM: &str = "▲ ";

fn claim_detail(c: &Value, bad: &[String]) -> Vec<String> {
    let mut d: Vec<String> = [
        ("zone", label(c, "topology.kubernetes.io/zone")),
        ("nodeclass", s(c, "/spec/nodeClassRef/name")),
        ("node", s(c, "/status/nodeName")),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .map(|(k, v)| format!("{k}: {v}"))
    .collect();
    if bad.is_empty() {
        d.push("conditions: all True".into());
    }
    d.extend(bad.iter().map(|b| format!("{PROBLEM}{b}")));
    d
}

/// STATE prefix of rows whose disruption is blocked; the rest names the cause.
pub const BLOCKED: &str = "blocked: ";

fn disruption_row(
    obj: &str,
    (kind, plural): (&str, &str),
    ns: Option<&str>,
    state: String,
    msg: String,
    age: Option<i64>,
    level: Level,
) -> Row {
    Row {
        cells: vec![obj.into(), kind.into(), state, age_cell(age), msg.clone()],
        level,
        target: Some(target(plural, ns, obj)),
        detail: vec![format!("message: {msg}")],
        idle: false,
        node: (plural == "nodes").then(|| obj.to_string()),
        pool: None,
        age,
        object: None,
    }
}

fn blocked_row(
    obj: &str,
    kp: (&str, &str),
    ns: Option<&str>,
    msg: String,
    age: Option<i64>,
) -> Row {
    let cause = cause(&msg);
    let level = if matches!(cause, "deleting" | "initializing") {
        Level::Info
    } else {
        Level::Warn
    };
    disruption_row(obj, kp, ns, format!("{BLOCKED}{cause}"), msg, age, level)
}

fn cause(msg: &str) -> &'static str {
    let m = msg.to_lowercase();
    if m.contains("pdb") || m.contains("disruption budget") {
        "PDB"
    } else if m.contains("nominated") {
        "nominated"
    } else if m.contains("do-not-disrupt") || m.contains("do not disrupt") {
        "do-not-disrupt"
    } else if m.contains("deleting") || m.contains("deletion") {
        "deleting"
    } else if m.contains("initialized") || m.contains("associated node") {
        "initializing"
    } else {
        "other"
    }
}

struct Ev {
    kind: String,
    reason: String,
    msg: String,
    ts: String,
    age: Option<i64>,
}

/// Event reasons the Disruption tab reads; the loader lists only these.
pub const EVENT_REASONS: &[&str] = &[
    "DisruptionBlocked",
    "Unconsolidatable",
    "DisruptionLaunching",
    "DisruptionWaitingReadiness",
    "DisruptionTerminating",
];

/// Latest event per object among those with one of `reasons`.
fn latest_events(i: &Inputs, reasons: &[&str], now: i64) -> BTreeMap<String, Ev> {
    let mut latest: BTreeMap<String, Ev> = BTreeMap::new();
    for e in i.events {
        let reason = s(e, "/reason");
        if !reasons.contains(&reason) {
            continue;
        }
        let kind = [s(e, "/involvedObject/kind"), s(e, "/regarding/kind")]
            .concat()
            .to_lowercase();
        let obj = [s(e, "/involvedObject/name"), s(e, "/regarding/name")].concat();
        let ts = EVENT_TS
            .iter()
            .map(|p| s(e, p))
            .find(|t| !t.is_empty())
            .unwrap_or("");
        if latest.get(&obj).is_some_and(|o| o.ts.as_str() > ts) {
            continue;
        }
        let ev = Ev {
            kind,
            reason: reason.into(),
            msg: [s(e, "/message"), s(e, "/note")].concat(),
            ts: ts.into(),
            age: age_of(e, EVENT_TS, now),
        };
        latest.insert(obj, ev);
    }
    latest
}

fn kind_plural(kind: &str) -> (&'static str, &'static str) {
    if kind.starts_with("nodeclaim") {
        ("nodeclaim", "nodeclaims")
    } else {
        ("node", "nodes")
    }
}

/// Node disruption in one place: what Karpenter is replacing or removing,
/// what it cannot consolidate, and what blocks it (`DisruptionBlocked` events
/// and `do-not-disrupt` holders). The latest event per object wins, so a row
/// shows its current state.
fn disruption(i: &Inputs, now: i64) -> Vec<Row> {
    let mut out: Vec<Row> = latest_events(i, EVENT_REASONS, now)
        .into_iter()
        .map(|(obj, e)| {
            let kp = kind_plural(&e.kind);
            match e.reason.as_str() {
                "DisruptionBlocked" => blocked_row(&obj, kp, None, e.msg, e.age),
                "Unconsolidatable" => {
                    disruption_row(&obj, kp, None, e.reason, e.msg, e.age, Level::Info)
                }
                _ => disruption_row(&obj, kp, None, e.reason, e.msg, e.age, Level::Warn),
            }
        })
        .collect();
    let ann = format!(
        "/metadata/annotations/{}",
        DO_NOT_DISRUPT.replace('/', "~1")
    );
    for n in i.nodes.iter().filter(|n| s(n, &ann) == "true") {
        if out.iter().any(|r| r.cells[0] == name(n)) {
            continue;
        }
        let msg = format!("node has {DO_NOT_DISRUPT}");
        let age = age_of(n, CREATED, now);
        out.push(blocked_row(name(n), ("node", "nodes"), None, msg, age));
    }
    for p in i.pods.iter().filter(|p| s(p, &ann) == "true") {
        let node = s(p, "/spec/nodeName");
        if node.is_empty() {
            continue;
        }
        let ns = s(p, "/metadata/namespace");
        let msg = format!("pod {ns}/{} on {node} has {DO_NOT_DISRUPT}", name(p));
        let age = age_of(p, CREATED, now);
        let mut row = blocked_row(name(p), ("pod", "pods"), Some(ns), msg, age);
        row.node = Some(node.to_string());
        out.push(row);
    }
    for r in &mut out {
        let obj = r.cells[0].as_str();
        if let Some(c) = i.claims.iter().find(|c| {
            let node = s(c, "/status/nodeName");
            name(c) == obj || node == obj || r.node.as_deref() == Some(node)
        }) {
            r.pool = Some(label(c, POOL_LABEL).to_string());
            let node = s(c, "/status/nodeName");
            if r.node.is_none() && !node.is_empty() {
                r.node = Some(node.to_string());
            }
            r.detail.extend([
                format!("pool: {}", label(c, POOL_LABEL)),
                format!("claim: {} on node {}", name(c), s(c, "/status/nodeName")),
                format!(
                    "instance: {} {}",
                    label(c, "karpenter.sh/capacity-type"),
                    label(c, "node.kubernetes.io/instance-type")
                ),
            ]);
        }
    }
    out.sort_by(|a, b| a.cells[2].cmp(&b.cells[2]));
    out
}

fn pending_pods(i: &Inputs, full: &[&str], now: i64) -> Vec<Row> {
    let provisioning = i.claims.iter().any(|c| !claim_ready(c));
    i.pods
        .iter()
        .filter_map(|p| {
            if s(p, "/status/phase") != "Pending" {
                return None;
            }
            let cond = p
                .pointer("/status/conditions")
                .and_then(Value::as_array)?
                .iter()
                .find(|c| s(c, "/type") == "PodScheduled" && s(c, "/reason") == "Unschedulable")?;
            let why = if i.pools.is_empty() {
                "no NodePool exists".to_string()
            } else if provisioning {
                "likely waiting for capacity being provisioned".to_string()
            } else if !full.is_empty() {
                format!("likely blocked: pools at limit: {}", full.join(", "))
            } else {
                "likely no NodePool matches its requirements".to_string()
            };
            let ns = s(p, "/metadata/namespace");
            let age = age_of(p, CREATED, now);
            Some(Row {
                cells: vec![format!("{ns}/{}", name(p)), age_cell(age), why],
                level: Level::Warn,
                target: Some(target("pods", Some(ns), name(p))),
                detail: vec![format!("scheduler: {}", s(cond, "/message"))],
                idle: false,
                node: None,
                pool: None,
                age,
                object: None,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claim(pool: &str, n: &str, cpu: &str, ready: bool) -> Value {
        json!({
            "metadata": {"name": n, "labels": {POOL_LABEL: pool, "karpenter.sh/capacity-type": "spot"}},
            "status": {
                "nodeName": if ready { "node-a" } else { "" },
                "capacity": {"cpu": cpu, "memory": "8Gi"},
                "conditions": [{"type": "Ready", "status": if ready { "True" } else { "False" }, "reason": "NotReady"}]
            }
        })
    }

    fn pool(n: &str, cpu: &str) -> Value {
        json!({"metadata": {"name": n}, "spec": {
            "limits": {"cpu": cpu},
            "disruption": {"consolidationPolicy": "WhenEmpty", "budgets": [{"nodes": "10%"}]}
        }})
    }

    fn run(pools: &[Value], claims: &[Value], pods: &[Value], events: &[Value]) -> Report {
        report(&Inputs {
            pools,
            claims,
            nodes: &[],
            pods,
            events,
            events_ok: true,
            classes: &[],
        })
    }

    #[test]
    fn usage_against_limit_and_budget() {
        let r = run(
            &[pool("idle", "8"), pool("a", "8")],
            &[claim("a", "c1", "4", true), claim("a", "c2", "4", true)],
            &[],
            &[],
        );
        let row = &r.tabs[0][0];
        assert_eq!(row.cells[0], "a");
        assert!(row.cells[2].contains("100% 8/8"), "{:?}", row.cells);
        assert_eq!(row.cells[3], "16.0Gi · no limit");
        assert!(row.cells[5].contains("allows 1"), "{:?}", row.cells);
        assert_eq!(row.level, Level::Critical);
        assert_eq!(r.tabs[0][1].cells[0], "idle");
        assert_eq!(r.tabs[1].len(), 2);
        assert!(r.tabs[0][1].idle && !row.idle);
        assert!(row.matches("SPOT") && !row.matches("zzz"));
    }

    #[test]
    fn blocked_event_and_pending_pod_explained() {
        let ev = json!({"reason": "DisruptionBlocked", "involvedObject": {"kind": "Node", "name": "node-a"},
            "message": "Pdb prevents pod evictions", "lastTimestamp": "2026-01-01T00:00:00Z"});
        let pod = json!({"metadata": {"name": "p", "namespace": "ns"}, "status": {"phase": "Pending",
            "conditions": [{"type": "PodScheduled", "reason": "Unschedulable", "message": "0/3 nodes"}]}});
        let r = run(
            &[pool("a", "4")],
            &[claim("a", "c1", "4", true)],
            &[pod],
            &[ev],
        );
        let c = &r.tabs[3][0].cells;
        assert_eq!(
            (&c[0][..], &c[1][..], &c[2][..]),
            ("node-a", "node", "blocked: PDB")
        );
        assert!(
            r.tabs[3][0].detail.contains(&"pool: a".to_string()),
            "{:?}",
            r.tabs[3][0].detail
        );
        assert_eq!(c[4], "Pdb prevents pod evictions");
        assert_ne!(c[3], "-");
        assert!(r.tabs[2][0].cells[2].contains("pools at limit: a"));
        assert_eq!(r.tabs[2][0].detail, ["scheduler: 0/3 nodes"]);
    }

    #[test]
    fn nodeclass_state_reaches_pools_and_rows_carry_objects() {
        let mut a = pool("a", "8");
        a["spec"]["template"] = json!({"spec": {"nodeClassRef": {"name": "bad"}}});
        let mut b = pool("b", "8");
        b["spec"]["template"] = json!({"spec": {"nodeClassRef": {"name": "gone"}}});
        let mut c = claim("a", "c1", "4", true);
        c["spec"] = json!({"nodeClassRef": {"name": "bad"}});
        let class = |n: &str, ready: &str| {
            json!({"kind": "EC2NodeClass", "metadata": {"name": n},
                "spec": {"role": "r"}, "status": {"subnets": [{}, {}], "conditions": [
                    {"type": "SubnetsReady", "status": ready, "message": "no subnets"},
                    {"type": "Ready", "status": ready}]}})
        };
        let classes = [class("bad", "False"), class("ok", "True")];
        let r = report(&Inputs {
            pools: &[a, b],
            claims: &[c],
            nodes: &[],
            pods: &[],
            events: &[],
            events_ok: true,
            classes: &classes,
        });
        for p in &r.tabs[0] {
            assert_eq!(p.level, Level::Critical, "{}", p.cells[0]);
        }
        let a = r.tabs[0].iter().find(|p| p.cells[0] == "a").unwrap();
        assert_eq!(a.cells[7], "bad");
        assert!(
            a.detail.iter().any(|d| d.contains("nodeclass bad: ")),
            "{:?}",
            a.detail
        );
        let b = r.tabs[0].iter().find(|p| p.cells[0] == "b").unwrap();
        assert!(
            b.detail
                .iter()
                .any(|d| d.contains("nodeclass gone not found"))
        );
        let rows = &r.tabs[CLASSES];
        assert_eq!(
            rows[0].cells[..5],
            ["bad", "EC2NodeClass", "a", "1", rows[0].cells[4].as_str()]
        );
        assert!(
            rows[0].cells[4].starts_with("NotReady"),
            "{:?}",
            rows[0].cells
        );
        assert_eq!(
            (rows[0].level, rows[1].level),
            (Level::Critical, Level::Good)
        );
        assert!(rows[1].detail.contains(&"resolved: 2 subnets".to_string()));
        assert!(rows.iter().all(|r| r.object.is_some()));
        assert!(
            r.tabs[0]
                .iter()
                .chain(&r.tabs[1])
                .all(|r| r.object.is_some())
        );
    }

    fn ev(reason: &str, obj: &str, msg: &str, ts: &str) -> Value {
        json!({"reason": reason, "involvedObject": {"kind": "NodeClaim", "name": obj},
            "message": msg, "lastTimestamp": ts})
    }

    #[test]
    fn claim_states_and_age() {
        let mut deleting = claim("a", "gone", "4", true);
        deleting["metadata"]["deletionTimestamp"] = json!("2026-01-01T00:00:00Z");
        let mut old = claim("a", "ok", "4", true);
        old["metadata"]["creationTimestamp"] = json!("2020-01-01T00:00:00Z");
        let r = run(
            &[pool("a", "100")],
            &[
                old,
                claim("a", "new", "4", false),
                deleting,
                json!({"metadata": {"name": "boot", "labels": {POOL_LABEL: "a"}}, "status": {}}),
            ],
            &[],
            &[],
        );
        let by = |n: &str| r.tabs[1].iter().find(|x| x.cells[0] == n).unwrap();
        assert_eq!(by("ok").cells[8], "Ready");
        assert_eq!(by("ok").level, Level::Good);
        assert_eq!(by("ok").detail, ["node: node-a", "conditions: all True"]);
        assert!(by("new").detail.iter().any(|d| d.starts_with(PROBLEM)));
        assert_ne!(by("ok").cells[7], "-");
        assert_eq!(by("new").cells[7], "-");
        assert!(by("new").cells[8].starts_with("NotReady"));
        assert_eq!(by("gone").cells[8], "Deleting");
        assert_eq!(by("gone").level, Level::Info);
        assert_eq!(by("boot").cells[8], "Launching");
        assert_eq!(r.not_ready, 2);
    }

    #[test]
    fn blocked_causes_are_classified_and_grouped() {
        let r = run(
            &[],
            &[],
            &[],
            &[
                ev(
                    "DisruptionBlocked",
                    "z",
                    "Pod has \"karpenter.sh/do-not-disrupt\"",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "y",
                    "Nominated for a pending pod",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "x",
                    "something else",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "w",
                    "Pdb prevents pod evictions",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "v",
                    "Node is deleting or marked for deletion",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "u",
                    "Nodeclaim does not have an associated node",
                    "2026-01-01T00:00:00Z",
                ),
            ],
        );
        let causes: Vec<_> = r.tabs[3]
            .iter()
            .map(|x| (x.cells[0].as_str(), x.cells[2].as_str()))
            .collect();
        assert_eq!(
            causes,
            [
                ("w", "blocked: PDB"),
                ("v", "blocked: deleting"),
                ("z", "blocked: do-not-disrupt"),
                ("u", "blocked: initializing"),
                ("y", "blocked: nominated"),
                ("x", "blocked: other")
            ]
        );
        assert_eq!(r.tabs[3][1].level, Level::Info);
        assert_eq!(r.tabs[3][2].level, Level::Warn);
    }

    #[test]
    fn disruption_drops_rows_of_deleted_claims() {
        let events = [
            ev(
                "DisruptionTerminating",
                "c1",
                "terminating",
                "2026-01-01T00:00:00Z",
            ),
            ev(
                "DisruptionTerminating",
                "gone",
                "terminating",
                "2026-01-01T00:00:00Z",
            ),
        ];
        let r = run(
            &[pool("a", "8")],
            &[claim("a", "c1", "4", true)],
            &[],
            &events,
        );
        let names: Vec<_> = r.tabs[3].iter().map(|r| r.cells[0].as_str()).collect();
        assert_eq!(names, ["c1"]);
        // No claims listed (maybe denied): nothing is hidden.
        assert_eq!(run(&[], &[], &[], &events).tabs[3].len(), 2);
    }

    #[test]
    fn disruption_keeps_latest_event_per_object_and_levels() {
        let r = run(
            &[],
            &[],
            &[],
            &[
                ev(
                    "Unconsolidatable",
                    "a",
                    "no cheaper node",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionTerminating",
                    "a",
                    "terminating",
                    "2026-01-02T00:00:00Z",
                ),
                ev(
                    "Unconsolidatable",
                    "b",
                    "no cheaper node",
                    "2026-01-01T00:00:00Z",
                ),
                ev(
                    "DisruptionBlocked",
                    "b",
                    "Pdb prevents pod evictions",
                    "2026-01-02T00:00:00Z",
                ),
                ev(
                    "Unconsolidatable",
                    "c",
                    "no cheaper node",
                    "2026-01-01T00:00:00Z",
                ),
            ],
        );
        let rows = &r.tabs[3];
        assert_eq!(rows.len(), 3);
        let a = rows.iter().find(|x| x.cells[0] == "a").unwrap();
        let b = rows.iter().find(|x| x.cells[0] == "b").unwrap();
        let c = rows.iter().find(|x| x.cells[0] == "c").unwrap();
        assert_eq!(
            (a.cells[2].as_str(), a.level),
            ("DisruptionTerminating", Level::Warn)
        );
        assert_eq!(
            (b.cells[2].as_str(), b.level),
            ("blocked: PDB", Level::Warn)
        );
        assert_eq!(
            (c.cells[2].as_str(), c.level),
            ("Unconsolidatable", Level::Info)
        );
        assert_eq!(a.cells[1], "nodeclaim");
    }

    #[test]
    fn sort_columns_compare_numbers_ages_and_text() {
        let row = |cell: &str, age: Option<i64>| Row {
            cells: vec![cell.into()],
            level: Level::Good,
            target: None,
            detail: vec![],
            idle: false,
            node: None,
            pool: None,
            age,
            object: None,
        };
        use std::cmp::Ordering::{Greater, Less};
        assert_eq!(row("9", None).cmp_col(&row("10", None), 0, "CLAIMS"), Less);
        assert_eq!(
            row("██░░░░ 45% 36/80", None).cmp_col(&row("█░░░░░ 9% 1/8", None), 0, "CPU"),
            Greater
        );
        assert_eq!(
            row("x", Some(60)).cmp_col(&row("x", Some(3600)), 0, "AGE"),
            Less
        );
        assert_eq!(row("abc", None).cmp_col(&row("abd", None), 0, "NAME"), Less);
    }
}
