# Rules Index

Every rule file in this directory, with its enforcement. A script name means the rule is mechanical. "enforced in review" means a reviewer checks it.

| Rule | Description | Enforcement |
|---|---|---|
| [build-and-gates.md](build-and-gates.md) | Build, test, and commit gates. Nextest-only test runs. Never commit a red tree. | enforced in review |
| [code-clarity.md](code-clarity.md) | Show the module map first. Small named functions. User-facing code first, callees follow depth-first. Do not mix abstraction levels. Keep description strings pure. | `lint-rules.py` (fails `Args:` in description strings; advisory: files over 300 lines) + enforced in review |
| [commits.md](commits.md) | Conventional Commits 1.0.0. One commit per verified change. | enforced in review |
| [config.md](config.md) | No hardcoded configuration. Settings bubble up to the call site. | enforced in review |
| [decisions.md](decisions.md) | Log hidden decisions as a site comment plus a numbered ADR. | enforced in review |
| [layout.md](layout.md) | No `mod.rs`. No module-declaration glue files. One item per file. | `lint-rules.py` (fails `mod.rs` under `ARCMiS/`) + enforced in review |
| [logging.md](logging.md) | No printing. Log through tracing with structured fields. | `lint-rules.py` (fails `println!`, `eprintln!`, `print!`, `dbg!`) + enforced in review |
| [manifest.md](manifest.md) | Manifest structure: dependency order, `x.y.z` version pins, workspace inheritance. | `lint-rules.py` (advisory: two-component pins) + review for dep order |
| [minimal-code.md](minimal-code.md) | Write the least code. Prefer a maintained library over a re-implementation. | enforced in review |
| [naming.md](naming.md) | Workspace crate names are `ARCMiS-<name>`. Imports use the short alias. | `lint-rules.py` (fails lowercase package names and `arcmis_` imports) |
| [no-tuning.md](no-tuning.md) | No hand-tuning against the problem set. General solutions only. | enforced in review |
| [rust.md](rust.md) | Fully qualified paths, derives, and macros. Caller before callee. Tap chains. Functional style inside functions. | `lint-rules.py` (fails unqualified `use`, derives, and macro calls; advisory: turbofish, nesting) + enforced in review |
| [ste.md](ste.md) | Write all English per ASD-STE100. | `ste-lint.py` |

## Run Before Commit

```bash
cargo fmt --check
cargo clippy --workspace --all-targets
cargo nextest run --profile ci --workspace --all-targets --no-tests=pass
python3 .omp/scripts/lint-rules.py
python3 ~/.omp/agent/skills/asd-ste100/scripts/ste-lint.py <touched files>
```

Never commit a red tree. See [build-and-gates.md](build-and-gates.md) for the full gate list.
