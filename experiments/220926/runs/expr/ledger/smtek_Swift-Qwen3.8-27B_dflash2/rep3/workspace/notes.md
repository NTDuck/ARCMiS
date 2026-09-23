## Brainstorm

C to Rust translation of mapper_expr expression evaluator.

Key challenges:
1. Lexer pointer-to-pointer string mutation
2. Union types (token, stack object, signal value)
3. Linked-list AST with in-place pointer mutation
4. Constant folding calls evaluate during parse
5. Function table with varying arities
6. History buffers for input/output

Crate layout: src/lib.rs with mapper_expr module, tests as #[test] functions.
