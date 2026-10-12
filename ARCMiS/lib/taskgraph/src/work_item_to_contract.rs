//! Convert a legacy `WorkItem` into a typed `TaskContract`.

// WHY the allow: the legacy conversion still maps to the deprecated
// `Phase::Migration` for backward compatibility. The new walk uses
// `Phase::MigrateBatches` instead.
#![allow(deprecated)]

use domain::Phase;

use crate::kind::Kind;
use crate::risk::Risk;
use crate::strategy::Strategy;
use crate::task_contract::TaskContract;
use crate::task_id::TaskId;
use crate::work_item::WorkItem;
use crate::write_scope::WriteScope;

impl From<WorkItem> for TaskContract {
    fn from(item: WorkItem) -> Self {
        let strategy = match item.kind {
            Kind::Analyze | Kind::Plan | Kind::Review => Strategy::Uplift,
            Kind::Translate | Kind::Integrate => Strategy::Transform,
            Kind::Validate | Kind::Test | Kind::Repair => Strategy::Reimagine,
        };
        Self {
            task_id: TaskId(format!("legacy-{}-{}", item.role.name(), item.instruction.len())),
            phase: Phase::Migration,
            unit: item.instruction.clone(),
            strategy,
            risk: Risk::Medium,
            required_role: item.role,
            required_inputs: Vec::new(),
            write_scope: WriteScope::default(),
            max_attempts: Self::DEFAULT_MAX_ATTEMPTS,
            verification_required: true,
        }
    }
}
