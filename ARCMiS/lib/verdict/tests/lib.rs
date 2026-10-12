//! Integration tests for the verdict crate.

use mas_verdict::TaskEvidence;
use mas_verdict::Verdict;
use mas_verdict::VerdictComputer;

#[test]
fn empty_evidence_is_success() {
    let computer = VerdictComputer::new();
    assert_eq!(computer.compute(&[]), Verdict::Success);
}

#[test]
fn all_succeeded_is_success() {
    let computer = VerdictComputer::new();
    let evidence = vec![
        TaskEvidence {
            task_id: "t1".to_owned(),
            succeeded: true,
            failure_reason: None,
        },
        TaskEvidence {
            task_id: "t2".to_owned(),
            succeeded: true,
            failure_reason: None,
        },
    ];
    assert_eq!(computer.compute(&evidence), Verdict::Success);
}

#[test]
fn any_failure_marks_failed() {
    let computer = VerdictComputer::new();
    let evidence = vec![
        TaskEvidence {
            task_id: "t1".to_owned(),
            succeeded: true,
            failure_reason: None,
        },
        TaskEvidence {
            task_id: "t2".to_owned(),
            succeeded: false,
            failure_reason: Some("timeout".to_owned()),
        },
    ];
    let verdict = computer.compute(&evidence);
    assert!(matches!(verdict, Verdict::Failed { .. }));
    if let Verdict::Failed { reason } = verdict {
        assert_eq!(reason, "timeout");
    }
}

#[test]
fn failure_without_reason_uses_default() {
    let computer = VerdictComputer::new();
    let evidence = vec![TaskEvidence {
        task_id: "t1".to_owned(),
        succeeded: false,
        failure_reason: None,
    }];
    let verdict = computer.compute(&evidence);
    if let Verdict::Failed { reason } = verdict {
        assert_eq!(reason, "task failed");
    } else {
        panic!("expected Failed verdict");
    }
}
