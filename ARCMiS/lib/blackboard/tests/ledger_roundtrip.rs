//! Ledger roundtrip: append events, read them back in order, and exercise
//! the state/plan/tasks/graph files.

use blackboard::Decision;
use blackboard::Failure;
use blackboard::Graph;
use blackboard::GraphEdge;
use blackboard::GraphEdgeKind;
use blackboard::GraphNode;
use blackboard::GraphNodeKind;
use blackboard::Ledger;
use blackboard::Observation;
use blackboard::Phase;
use blackboard::PlanFile;
use blackboard::State;
use blackboard::TaskList;
use blackboard::TaskStatus;

#[test]
fn ledger_roundtrip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let ledger = Ledger::new(dir.path().to_path_buf());

    // Empty ledgers read as empty vectors.
    assert!(ledger.read_decisions().expect("empty decisions").is_empty());
    assert!(ledger.read_failures().expect("empty failures").is_empty());
    assert!(ledger.read_observations().expect("empty obs").is_empty());

    ledger
        .append_decision(&Decision {
            at: "2026-09-24T00:00:00Z".into(),
            phase: "DISCOVERY".into(),
            action: "delegate".into(),
            detail: serde_json::json!({"role": "analyst", "task": "t1"}),
            reasoning: "start with the entry points".into(),
        })
        .expect("append decision");
    ledger
        .append_decision(&Decision {
            at: "2026-09-24T00:01:00Z".into(),
            phase: "PLANNING".into(),
            action: "replan".into(),
            detail: serde_json::json!({"reason": "cycle"}),
            reasoning: "split the cycle".into(),
        })
        .expect("append decision 2");
    ledger
        .append_failure(&Failure {
            at: "2026-09-24T00:02:00Z".into(),
            phase: "MIGRATION".into(),
            category: "toolchain".into(),
            root_cause: "missing feature in Cargo.toml".into(),
            suggested_action: "add the feature and retry".into(),
        })
        .expect("append failure");
    ledger
        .append_observation(&Observation {
            at: "2026-09-24T00:03:00Z".into(),
            kind: "token_budget".into(),
            detail: serde_json::json!({"used": 1000, "cap": 5000}),
        })
        .expect("append observation");

    let decisions = ledger.read_decisions().expect("decisions");
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0].action, "delegate");
    assert_eq!(decisions[1].action, "replan");

    let failures = ledger.read_failures().expect("failures");
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].category, "toolchain");

    let observations = ledger.read_observations().expect("observations");
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].kind, "token_budget");
}

#[test]
fn state_and_plan_files() {
    let dir = tempfile::tempdir().expect("tempdir");

    // State: absent before first write, round-trips after.
    assert!(blackboard::state::read(dir.path()).expect("no state yet").is_none());
    blackboard::state::write(
        dir.path(),
        &State {
            phase_delegations: 0,
            phase_delegation_watermark: 0,
            phase: Phase::Migration,
            current_task: Some("t3".into()),
            current_batch: Some("batch-2".into()),
            current_model: "qwen3:8b".into(),
            updated_at: "2026-09-24T00:00:00Z".into(),
            last_transition: "pilot passed".into(),
        },
    )
    .expect("write state");
    let state = blackboard::state::read(dir.path()).expect("read state").expect("state present");
    assert_eq!(state.phase, Phase::Migration);
    assert_eq!(state.current_batch.as_deref(), Some("batch-2"));

    // Plan: capped write.
    let plan = PlanFile::with_cap(dir.path(), 40);
    let long = "x".repeat(100);
    plan.write_capped(&long).expect("write plan");
    let read_back = plan.read().expect("read plan");
    assert!(read_back.len() < 120, "cap must truncate, got {}", read_back.len());
    assert!(read_back.contains("plan truncated"));
}

#[test]
fn tasks_lifecycle_and_dedup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let tasks = TaskList::new(dir.path());

    let id = tasks.add("translate src/lib.rs", &[]).expect("add t1");
    assert_eq!(id, "t1");
    let id2 = tasks.add("translate src/main.rs", &["t1".into()]).expect("add t2");
    assert_eq!(id2, "t2");

    tasks.set_status("t1", TaskStatus::InProgress).expect("set status");
    let list = tasks.read().expect("read");
    assert_eq!(list[0].status, TaskStatus::InProgress);
    assert_eq!(list[1].depends_on, vec!["t1".to_owned()]);

    // Merge with an exact duplicate plus one fresh task.
    let added = tasks
        .merge_dedup(vec![
            blackboard::Task {
                id: String::new(),
                description: "translate src/lib.rs".into(),
                status: TaskStatus::Pending,
                depends_on: vec![],
                assigned_to: None,
            },
            blackboard::Task {
                id: String::new(),
                description: "write regression test".into(),
                status: TaskStatus::Pending,
                depends_on: vec!["t2".into()],
                assigned_to: None,
            },
        ])
        .expect("merge");
    assert_eq!(added.len(), 1, "duplicate must be dropped");
    let list = tasks.read().expect("read");
    assert_eq!(list.len(), 3);
    assert_eq!(list[2].id, "t3");
}

#[test]
fn graph_batches_and_scc() {
    let mut graph = Graph::new();
    // Module graph: m1 -> m2 -> m3, plus m4 isolated, plus a function node.
    for (id, file) in [("module:m1", "m1.rs"), ("module:m2", "m2.rs"), ("module:m3", "m3.rs"), ("module:m4", "m4.rs")] {
        graph.add_node(GraphNode {
            id: id.into(),
            kind: GraphNodeKind::Module,
            label: file.into(),
            file: Some(file.into()),
            line: None,
            attrs: Default::default(),
        });
    }
    graph.add_node(GraphNode {
        id: "fn:main".into(),
        kind: GraphNodeKind::Function,
        label: "main".into(),
        file: Some("m1.rs".into()),
        line: Some(1),
        attrs: Default::default(),
    });
    let edge = |from: &str, to: &str| GraphEdge {
        from: from.into(),
        to: to.into(),
        kind: GraphEdgeKind::Imports,
        attrs: Default::default(),
    };
    graph.add_edge(edge("module:m1", "module:m2"));
    graph.add_edge(edge("module:m2", "module:m3"));
    graph.add_edge(edge("fn:main", "module:m3"));

    let batches = graph.topological_batches();
    assert!(!batches.is_empty());
    // m1 must land in an earlier batch than m2, m2 before m3.
    let position =
        |id: &str| batches.iter().position(|batch| batch.iter().any(|member| member == id)).expect("node in batches");
    assert!(position("module:m1") < position("module:m2"));
    assert!(position("module:m2") < position("module:m3"));
    assert!(batches.iter().any(|batch| batch.contains(&"module:m4".to_owned())));

    // Cycle detection: m1 -> m2 -> m1 forms one SCC.
    graph.add_edge(edge("module:m2", "module:m1"));
    let sccs = graph.strongly_connected_components();
    let cycle = sccs
        .iter()
        .find(|scc| scc.contains(&"module:m1".to_owned()) && scc.contains(&"module:m2".to_owned()))
        .expect("cycle found");
    assert!(cycle.contains(&"module:m1".to_owned()));

    // Persistence roundtrip.
    let dir = tempfile::tempdir().expect("tempdir");
    graph.write(dir.path(), "discovery").expect("write graph");
    let reloaded = Graph::read(dir.path(), "discovery").expect("read graph");
    assert_eq!(reloaded.nodes().count(), graph.nodes().count());
    assert_eq!(reloaded.edges().len(), graph.edges().len());
}
