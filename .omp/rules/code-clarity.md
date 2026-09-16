---
description: Code clarity — at module level show the list of things, per item show what it does in small named functions. Separate abstraction levels. No meddled util code.
---

# Code Clarity

Structure every module so a reader sees the map first and the detail second.

## Module level: show the list

- A module that provides things (tools, agents, commands) opens with the complete list of those things. The list is the module documentation or the re-export block at the top of the file.
- Utility code does not sit next to the list. It lives in its own submodule below the items that use it.

## Item level: show what it does

- Each item (one tool, one agent, one command) starts with a short high-level description of what it does, as a doc comment. No dubious wording. State the effect.
- The body of one item reads as small named functions. Each function does one step and its name says which. A reader scans the function names and gets the behavior without reading bodies.
- Keep one item per file when the item is more than a few lines. The file name is the item name.

### Block order

- In one file, the item that the user of the module meets first comes first. Write the tool, agent, or command first. Then write its impl block. Then write its main entry function.
- Inside a function, the body stays high level. Write a sequence of small named calls. Then write `return`.
- Immediately after that function, declare the functions it calls. Order them in the order the body calls them. This is depth-first: a caller comes before each callee. Each next function sits one level lower.
- Example: `run()` calls `resolve()` and then `read()`. Declare `resolve` first. The helpers of `resolve` follow it. Then declare `read`.
- Balance: when a function calls many helpers, group the helpers logically and accept a small jump. The goal is clean and idiomatic code. Do not apply the depth-first stencil with clutter.

## Description purity

- A doc comment states what the item does. A model-facing tool `description()` string states what the item does. Nothing else.
- Do not embed a parameter list, a JSON example, or an argument schema in these texts. The `parameters()` method carries the schema. The compiler carries the types.

## Abstraction levels: do not mix

- One function lives on one abstraction level. A function that orchestrates steps does not also parse strings.
- Library code holds logic. The binary holds wiring (config load, logging setup, process exit codes). Wiring never hides in library functions.
- Error handling sits at the level that owns the choice: a tool reports its failure, the caller decides what it means.

## Violations

- A file that buries the tool list under helper functions.
- A 200-line function that mixes policy, parsing, and IO.
- A `util.rs` dumping ground.
