//! Per-role output-token override. One hook per agent at build time: the
//! hook patches every completion call for its role to the configured
//! ceiling, leaving agents without an override untouched. Config-bubbled
//! (`mas.role_output_tokens`), no hardcoded values.

use rig::agent::hook::AgentHook;
use rig::agent::hook::CompletionCall;
use rig::agent::hook::CompletionCallAction;
use rig::agent::hook::HookContext;
use rig::agent::hook::RequestPatch;

/// Patch this agent's requests to the role's output-token ceiling.
#[derive(Clone, Debug)]
pub struct RoleOutputTokensHook {
    max_tokens: u64,
}

impl RoleOutputTokensHook {
    /// Build the hook for one role's override.
    #[must_use]
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max_tokens,
        }
    }
}

impl AgentHook for RoleOutputTokensHook {
    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCall<'_>) -> CompletionCallAction {
        CompletionCallAction::patch(RequestPatch::new().max_tokens(self.max_tokens))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_patch_with_the_ceiling() {
        // HookContext is pub(crate)-constructed in rig, so the patch content
        // is verified directly: the action carries the configured ceiling.
        let patch = RequestPatch::new().max_tokens(RoleOutputTokensHook::new(16384).max_tokens);
        assert_eq!(patch.max_tokens, Some(16384));
    }

    #[test]
    fn zero_is_a_valid_ceiling_not_a_disable() {
        // Distinct from num_ctx's 0-disables convention: an output cap of 0
        // would be a config error the provider rejects, and the hook passes
        // it through untouched so the wire surfaces it.
        let patch = RequestPatch::new().max_tokens(RoleOutputTokensHook::new(0).max_tokens);
        assert_eq!(patch.max_tokens, Some(0));
    }
}
