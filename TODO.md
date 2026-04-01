# TODO:

- [ ] ~~Prime directive~~ Main usability goal:
  UX "as fast" / "as ergonomic" to use as going to https://play.rust-lang.org/
  in your browser ... qualitatively and within reason, subject to the limitations of:
    - user's chosen IDE, and
    - what a CLI tool can do

## "Goal one" milestones

- [x] **cli**: autocomplete
    - [x] static for subcmds and flags
    - [x] dynamic, for script and template names?
        - Requires **unstable features** from clap! Very _very_ little documentation!
        - note this uses a completely different method for installing completions
          than the normal "static" `clap_complete::generate` functions.
    - [x] ~~Install completions for various
      shells?~~ NO. Just send the scripts to stdout
      and then user can figure out where to redirect it for their specific
      circumstance. Basically the only robust way to "install" these
      would be to use the system's package manager (and even then,
      what about oh-my-zsh?)
- [x] editor integration / fast editing workflow: `cpg edit [name]` /
  `cpg new --edit [name]`
    - Q: Can you tell vscode and/or intellij "open
      _this_ file for editing WITHIN this project".
        - [x] Rustrover: only if the project is already open in the IDE already.
            - if the project is already open, then yes, `rustrover [path]` does
              the right thing.
            - If it's not already open, but the project is already
              set up as a rustrover project, then it mostly works ... but it
              renames your project to the name of the script ... or something?
            - If it's not already set up with a
              `.idea` folder, then it opens just the file
              itself and complains.
            - Is there a different way to call rustrover that would always work?
              [Doesn't seem like it.](https://www.jetbrains.com/help/idea/working-with-the-ide-features-from-command-line.html#arguments)
        - [ ] vscode: todo
- [x] Cargo.toml discovery
    - [x] configurable **Cargo.toml location** via env var or CLI,
      so you can use it from anywhere (`CARGO_PLAYGROUND_MANIFEST_PATH`)
- [x] "Naming support"
    - [x] name generator: done (lifted naming scheme directly from docker).
    - [x] ~~`cpg new`: generate a name when creating a project if not
      specified?~~ no.
        - BUT: then there is no way to specify dependencies with an anonymous new
          project. Can we say that you have to specify deps via "--dep" in this case?
          alternative 1: always require `--dep` / `-d`, take a list like w/ features
          (makes us different than how cargo does it).
          alternative 2: optionally allow `--dep/-d`, require it for anonymous
          scripts (leads to confusing usage messages). **alternative 3**: use a
          different command (`cpg quick`?) to make it explicit.
    - [x] `cpg quick`
    - [x] `cpg rename`
- [x] "play with a crate" workflow
    - [x] ~~`cpg play-with $depname`~~
      `cpg quick [dupname]` creates new automatically named script
      with proper dependencies (and a `use` at the top? no, we don't actually
      know the crate name)
    - [x] ~~opens API docs in browser?~~ nah
- [ ] finish splitting this into its own repo
- [ ] lifecycle test
    - init a repo, add templates, create scripts, add dependencies, run scripts

## Functionality

- [ ] Alfred workflow W/ same autocomplete support as terminal (for me anyway)

### Bugs

- [x] passing arguments to script via
  `cargo playground run $scriptname -- $args`
- [ ] passing arguments to script via `cargo playground run $scriptname $args`?
    - is there equivalent of `parse_known_args()` or something?

### Cargo script support

- [ ] Parse it the same way as cargo's (unstable) implementation
    - important modules seem to be [
      `..::util::toml::embedded`](https://github.com/rust-lang/cargo/blob/0f14d9d2fa4c/src/cargo/util/toml/embedded.rs) and
      [
      `..::util::frontmatter`](https://github.com/rust-lang/cargo/blob/0f14d9d2fa4c/src/cargo/util/frontmatter.rs) ... it uses a whole-ass parser-combinator
      library called "winnow" (although the initial implementation just used regexes,
      which is probably not really ok for the usual reasons).
    - `winnow` is maintained by a maintainer of cargo itself so it's still high trust
      (also it's a fork of `nom` apparently?)

## usage as `cargo playground`

- [ ] require enabling "metadata.playground" flag in Cargo.toml
  before modifying it so you don't accidentally a non-playground.
- [ ] local template dir creation / management

## usage as `xtask`

- [x] configurable location for the main project (implemented: use
  `CARGO_PLAYGROUND_MANIFEST_PATH` or `--manifest-path`)
- [ ] get autocomplete working w/ `cargo xtask`?
    - somehow screen it so that it only invokes our autocomplete for "cargo xtask"
      when running in the correct workspace? In the shell script, filters
      on the CWD or something?

### Required for a "release"

- [ ] properer output system
    - [ ] Use a logging crate? But it's not really "logging" innit.
      `cargo` uses its own custom output system, maybe just vendor that?
    - [ ] global, configurable / term-dependent disabling of **ANSI color**
        - probably https://docs.rs/colorchoice-clap/1.0.8/colorchoice_clap/
    - [ ] use stderr for everything that you wouldn't want to pipe

### Tech debt

- [ ] Take more steps to manage printing to stdout (it will break autocomplete
  if we don't ...)
- ~~[ ] Migrate the passthrough args to a declarative derive
  macro~~: yeah no this is not worth it
    - Observation: a macro that reads like "build me a struct like this",
      (which won't look like idiomatic rust at all, at best it will
      look like a function that evaluates to a struct definition) is
      MUCH easier to write than a "standard" derive macro that needs to process
      attributes on fields in a type definition. Because you can make it only do
      exactly what it needs to do, by construction, rather than have it accept
      arbitrary type defs that might do literally anything and then have it
      be an error if it's something you don't support. I.e., it's much easier
      to create a limited DSL than it is to integrate with rust the language.
      Which I guess makes sense.
- [ ] make it work on **windows** (spawn in `cpg run` instead of
  exec, format paths correctly, make color work)

#### Nice to have

- [x] Parse `-F/--feature` the same way as cargo (i.e., allow for multiple
  space-comma separated features in a single argument)
- [ ] 
  `cargo script` support - manage an ["embedded manifest"](https://rust-lang.github.io/rfcs/3502-cargo-script.html) and keep it
  in sync w/ our `Cargo.toml`
- [ ] add (optionalyl) "extern crate" statements and/or comments to the
  top of each script when dependencies and/or features are added?
- [ ] use styles from `cli_style` for output everywhere
- [ ]  `cargo playground init` -
  to create a whole-ass new project or enable metadata on existing one
    - [ ] optionally initialize as xtask? (i.e., add this code to the workspace?)
- [ ] `cargo playground rename`
- [ ] `cargo playground copy'
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
- [x] Don't run cargo add if no deps will change

#### Pedantic correctness

These are about correctness, including cases even if manifest is manually edited.
These are "pedantic" if they're low likelihood or just Not A Big Deal if they happen.

- [ ] validate script names and filenames in `cpg new`
- [ ] Cargo.toml race conditions / TOCTOU / cargo locking
  (rn we are assuming user is not doing 10 things at once.
  "AI" agents might do this but I don't care about them.)
- [ ] ensure two scripts w/ different names don't point to same file?
- [x] ensure bin entries don't have duplicate names
- [ ] dependency consistency (what did I mean by this?)

Note:

1. Some of this could maybe checked via `cargo check`, although calling out
   to it every time seems a little silly
2. Some of these might already get good error messages when run via `cargo run`
   or whatever, which also solves the problem
