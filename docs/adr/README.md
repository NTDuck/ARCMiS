# Decision Records

Numbered, immutable Architecture Decision Records. Format and policy: [`.omp/rules/decisions.md`](../../.omp/rules/decisions.md).

- One file per decision: `NNNN-kebab-case-title.md`.
- Once accepted, never edited — superseded by a new record that links back.
- Every decision not directly inferrable from code lands here, with a WHY comment at the affected site.

| # | Title | Status |
| - | ----- | ------ |
| [0001](0001-litmus-derived-rust-scaffold.md) | Litmus-derived Rust scaffold | accepted |
| [0002](0002-stable-rustfmt-drop-required-version.md) | Stable rustfmt: drop required_version | accepted |
| [0003](0003-rig-distilled-into-arex-skill-graph.md) | Distill rig into AREX-style skill graph | accepted |
| [0004](0004-mise-tasks-instead-of-cargo-aliases.md) | Mise tasks instead of cargo aliases | accepted |
| [0005](0005-simplified-technical-english-for-rules.md) | Simplified Technical English for rules and agent text | accepted |
| [0006](0006-workspace-layout-and-cucumber-tests.md) | Workspace layout and cucumber tests | accepted |
| [0007](0007-ollama-migration-agent.md) | Ollama-driven migration agent over GildedRose | accepted |
| [0008](0008-clarity-logging-no-tuning-nested-layout.md) | Code clarity, minimal code, logging, no tuning, nested layout | accepted |
| [0009](0009-crate-naming-manifest-structure-module-layout.md) | Crate naming, manifest structure, and module layout | accepted |
| [0010](0010-user-facing-code-order-and-macro-qualification.md) | User-facing code order, macro qualification, and module flattening | accepted |
| [0011](0011-util-directory-layout.md) | Util directory layout | accepted |
| [0012](0012-seventeen-tool-surface.md) | Seventeen-tool surface after oh-my-pi | accepted |
| [0013](0013-react-default-agent.md) | ReAct default agent | accepted |

## References

- [OpenAI: Harness engineering](https://openai.com/index/harness-engineering/) — repository as system of record. A short AGENTS.md as map, `docs/` as encyclopedia.
- [Martin Fowler: ArchitectureDecisionRecord](https://martinfowler.com/bliki/ArchitectureDecisionRecord.html) — ADR form: short, numbered, immutable, supersede-don't-edit.
