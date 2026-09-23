use std::collections::VecDeque;

// ============================================================
// Brainstorming notes - Expression Evaluator Translation
// ============================================================
//
// Source: libmapper_expr.c (C, BSD licensed)
// Target: Rust
//
// Key Design Decisions:
//
// 1. State Machine Representation
//    The C code uses an enum-based state machine with ~20 states.
//    In Rust, we should use an enum for states (idiomatic Rust).
//    However, the parser's control flow is deeply intertwined
//    with stack operations. A hybrid approach makes sense:
//      - Enum for state machine states
//      - Separate structs for lexer, parser, evaluator
//      - Use Result types instead of die_unless
//
// 2. Signal Processing Semantics
//    The input_history/output_history arrays track which
//    past outputs an expression references via history_index
//    and vector_index. This is NOT just a feature - it's
//    part of the domain model. We should represent this as:
//      - A Vec<HistoryEntry> or similar
//      - HistoryEntry: { value: f64, history_index: usize, vector_index: usize }
//
// 3. Floating Point Handling
//    C uses IEEE 753 float (4 bytes). Rust has f32 and f64.
//    Since the C code uses float throughout, we should
//    either:
//      a) Use f64 to match C's float precision (may lose
//         exactness but is safer for math)
//      b) Use f32 to match C's float exactly
//    Recommendation: Use f64 for correctness and
//    compatibility with typical math operations. If
//    strict float matching is needed, add a conversion
//    layer.
//
// 4. Memory Management
//    C: malloc/free throughout
//    Rust: Box<T>, Vec<T>, Rc<RefCell<'_, T>>
//    Recommendation: Use owned types (Box/Vec) for
//    simplicity. If sharing is needed, use Rc/RefCell.
//
// 5. Error Handling
//    C: die_unless macros with assertions
//    Rust: Result<T, Error> or Option<T>
//    Recommendation: Use Result types throughout.
//    Define a custom Error enum for domain errors.
//
// 6. Architecture
//    The C code is a single file with everything mixed together.
//    In Rust, we should separate concerns:
//      - lexer.rs: Tokenization
//      - parser.rs: Parsing with state machine
//      - evaluator.rs: Stack-based evaluation
//      - constants.rs: Function table
//      - history.rs: Signal processing semantics
//    This makes testing easier and follows Rust idioms.
//
// 7. API Surface
//    C: Exported functions (expr_parse, expr_eval, etc.)
//    Rust: Public module with clear function signatures.
//    Consider using traits for the math functions.
//
// ============================================================
