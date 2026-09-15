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

## Abstraction levels: do not mix

- One function lives on one abstraction level. A function that orchestrates steps does not also parse strings.
- Library code holds logic. The binary holds wiring (config load, logging setup, process exit codes). Wiring never hides in library functions.
- Error handling sits at the level that owns the choice: a tool reports its failure, the caller decides what it means.

## Violations

- A file that buries the tool list under helper functions.
- A 200-line function that mixes policy, parsing, and IO.
- A `util.rs` dumping ground.
