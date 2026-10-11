---
description: Code clarity — the Stepdown Rule for every software entity. A reader at any level sees the highest concept first, and each next paragraph drops exactly one level. Show the module map first, each item's behavior as small named functions, one abstraction level per body. No meddled util code.
---

# Code Clarity

Apply the Stepdown Rule from Robert C. Martin's *Clean Code* to every
software entity: the crate, the module, the file, the item, the type, the
function. A reader should read the source top-down. Each level states the
concept in terms of the level below. The reader can stop reading at any
level and still hold the whole picture.

## The rule, one sentence

Write so that reading proceeds top-down: each entity first speaks the
vocabulary of the level above it, then the vocabulary of the level below
it.

## Levels

The scale runs: crate -> module -> file -> item (tool, agent, command) ->
type or function -> statement. The next sections state the duty of each
level. All sections describe one rule at different depths.

### Crate level: show the offer

- The crate root (`lib.rs`) opens with the complete list of things the
  crate offers: module declarations plus the re-export block. The list is
  the map. A reader scans it and knows what lives here.
- The binary (`main.rs`) opens with the numbered run story: the sequence
  of steps the program performs, as a doc comment. The story is the map.

### Module level: show the list

- A module that provides things opens with the complete list of those
  things. The list is the module documentation or the re-export block at
  the top of the file.
- Utility code does not sit next to the list. It lives in its own
  submodule below the items that use it.

### File level: one item, one file

- Keep one item per file when the item is more than a few lines. The file
  name is the item name. The file reads as one stepdown: item top, detail
  below.
- Utility code lives in its own source file under `src/util/`, one topic
  per file. See `layout.md`.

### Item level: show what it does

- Each item starts with a short high-level description of what it does, as
  a doc comment. No dubious wording. State the effect.
- The body of one item reads as small named functions. Each function does
  one step and its name says which. A reader scans the function names and
  gets the behavior without reading bodies.

### Function level: one body, one level

- Inside a function, the body stays high level. Write a sequence of small
  named calls. Then write `return`.
- One function lives on one abstraction level. A function that orchestrates
  steps does not also parse strings.
- To read a function top-down is to read its steps in order. The next
  function name down is the next paragraph down.

## Order: the stepdown as physical layout

The stepdown is also a layout rule. Entities lower in the abstraction
hierarchy sit lower in the file. A reader who stops at line N holds only
concepts at or above the level of line N.

### Block order within one file

- The item that the user of the module meets first comes first. Write the
  tool, agent, or command first. Then write its impl block. Then write its
  main entry function. The item's type declaration comes first.
- Immediately after that function, declare the functions it calls. Order
  them in the order the body calls them. This is depth-first: a caller
  comes before each callee. Each next function sits one level lower.
- Example: `run()` calls `resolve()` and then `read()`. Declare `resolve`
  first. The helpers of `resolve` follow it. Then declare `read`.
- Balance: when a function calls many helpers, group the helpers logically
  and accept a small jump. The goal is clean and idiomatic code. Do not
  apply the depth-first stencil with clutter.

#### Declaration before implementation

- A type declaration precedes the impl block that gives it behavior. The
  reader meets the shape before the behavior. `pub struct Foo {...}` comes
  before `impl Foo {...}`.
- An `impl Trait for X` block comes between `X`'s declaration and the
  types the impl signature names.
- Within one item, order is: primary type declaration, its impl blocks,
  secondary types the impl needs, then helpers in call order. Tool
  example: `pub struct WriteFile {...}` -> `impl ::rig::tool::Tool for
  WriteFile {...}` -> `pub struct WriteFileArgs {...}` -> helper fns.
- Secondary types (request and response artifacts, args) sit below the
  entry functions: the reader meets the behavior before the data shapes
  that serve it.

## Description purity

- A doc comment states what the item does. A model-facing tool
  `description()` string states what the item does. Nothing else.
- Do not embed a parameter list, a JSON example, or an argument schema in
  these texts. The `parameters()` method carries the schema. The compiler
  carries the types.

## Cross-entity stepdown

The stepdown holds between entities, not only within one file:

- The crate re-exports its items. The item exposes its entry function.
  The entry function names its steps. The steps use primitives and
  helpers. Each boundary narrows the vocabulary and grows the detail.
- Library code holds logic. The binary holds wiring (config load, logging
  setup, process exit codes). Wiring never hides in library functions.
- Error handling sits at the level that owns the choice: a tool reports
  its failure, the caller decides what it means.
- An entity owned exclusively by one other entity lives in that entity's
  file, below its owner's entry points. A shared entity lives in `util/`.

## Line cap

- One source file holds at most 300 lines. The linter lists longer files
  as advisory findings.
- A tool file is exempt. The file is the tool, and a complex domain
  (protocol handling, selector parsing) grows the helpers. The tool
  struct, its impl, and the args stay at the top. The cap stays for every
  other file.

## Violations

- A file that buries the tool list under helper functions.
- A 200-line function that mixes policy, parsing, and IO.
- A `util.rs` dumping ground.
- A function whose body mixes two levels: orchestration plus string
  parsing.
- An impl block above its type declaration.
- A callee declared above its caller with no grouping reason.
