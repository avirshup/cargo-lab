# TODO:

- [ ] Goal one: UX "as fast" / "as ergonomic" to use as going to https://play.rust-lang.org/
  in your browser. Qualitatively / within reason anyway. Possibly by integrating power-user
  tooling (alfred?).

## "Goal one" milestones

- [ ] **cli**: shell autocompletion for script and template names:
  cargo uses "clap_complete" (unstable?)- [ ] editor integration
- [ ] fast editing workflow: `cpg edit [name]` / `cpg new --edit [name]`
    - [ ] Q: Can you tell vscode and/or intellij "open
      _this_ file for editing WITHIN this project"
- [ ] Cargo.toml discovery
    - [ ] configurable **Cargo.toml location** via env var or CLI,
      so you can use it from anywhere (`CARGO_PLAYGROUND_MANIFEST_PATH`)
- [ ] "Naming support"
    - [ ] `cpg new`: generate a name creating a project if not specified (this
      breaks the current CLI tho)
    - [ ] `cpg rename`
- [ ] "play with a crate" workflow
    - [ ] `cpg play-with $depname`: creates new automatically named script
      with proper dependencies (and a `use` at the top? no, we don't actually
      know the crate name)
    - [ ] opens API docs in browser
- [ ] Alfred workflow W/ same autocomplete support as terminal (for me anyway)

## Functionality

## usage as `cargo playground`

- [ ] require enabling "metadata.playground" flag in Cargo.toml
  before modifying it so you don't accidentally a non-playground.
- [ ] local template dir creation / management

## usage as `xtask`

- [ ] configurable location for the main project

### Required for a "release"

- [ ] make it work on **windows** (spawn in `cpg run` instead of
  exec, format paths correctly, make color work)
- [ ] properer output system (is a logger appropriate for this?)
    - [ ] global, configurable / term-dependent disabling of **ANSI color**
    - [ ] use stderr for everything that you wouldn't want to pipe

#### Nice to have

- [ ] 
  `cargo script` support - manage an ["embedded manifest"](https://rust-lang.github.io/rfcs/3424-cargo-script.html) and keep it
  in sync w/ our `Cargo.toml`
- [ ] use styles from `cli_style` for output everywhere
- [ ]  `cargo playground init` -
  to create a whole-ass new project or enable metadata on existing one
    - [ ] optionally initialize as xtask? (i.e., add this code to the workspace?)
- [ ] `cargo playground rename`
- [ ] `cpg template`: list (modify/create?) available
  **templates**, + tell user where stored, how to modify
- [ ] **Version conflicts**: what happens if `cpg new script1 dep@0.1.2` then
  `cpg new script2 dep@0.5.0`?
    - right now, it will update the version for all scripts, which is _surprising_
    - most flexible: allow multiple versions via renaming
    - much simpler: warn before proceeding
    - even simpler than that: warn + do nothing + advise user to use
      `cargo add --optional` to manage versions?
- [ ] Don't exec out to `diff`, a dependency is fine for that

## QOL / usability

- [x] strip ".rs" from script names if they are passed
- [x] **ANSI color help styling** using the same style as cargo
- [x] `cpg new`: allow specifying deps at same time as creating new script
- [ ] `Arguments for cargo add` section has too much whitespace
- [ ] include `cargo add`'s changes to dependencies when we run `diff`
- [ ] Don't say "Updated Cargo.toml" updated if nothing changed
- [ ] Don't run cargo add if no deps will change

#### Pedantic correctness

These are about correctness, including cases even if manifest is manually edited.
These are "pedantic" if they're low likelihood or just Not A Big Deal if they happen.

- [ ] validate script names and filenames in `cpg new`
- [ ] Cargo.toml race conditions / TOCTOU / cargo locking
  (rn we are assuming user is not doing 10 things at once.
  "AI" agents might do this but I don't care about them.)
- [ ] ensure two scripts w/ different names don't point to same file
- [ ] ensure bin entries don't have duplicate names
- [ ] dependency consistency

Note:

1. Some of this could maybe checked via `cargo check`, although calling out
   to it every time seems a little silly
2. Some of these might already get good error messages when run via `cargo run`
   or whatever, which also solves the problem
