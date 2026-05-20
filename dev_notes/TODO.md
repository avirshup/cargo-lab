# TODO:

- [x] (sufficiently done) ~~Prime directive~~ Main usability goal:
  UX "as fast" / "as ergonomic" to use as going to https://play.rust-lang.org/
  in your browser ... qualitatively and within reason, subject to the limitations of:
    - user's chosen IDE, and
    - what a CLI tool can do

### Bugs

- [x] BUG: passing arguments to scripts via doesn't work
  (`cargo playground run $scriptname -- $args` should work)

- autocomplete:
    - [x] BUG: "global" arguments are suggested w/ base "cargo" cmd, that ain't right
        - WHY? because `cargo playground generate` generates completions for bare
          `cargo` itself I guess?
    - [x] `cargo play[tab]` does not autocomplete even though
      `cargo playground [args] [tab]` does
    - [ ] It should work with cargo aliases (if u set `[alias] pg = "playground"`
      then `cargo pg [tab]` should still work)
        - This _used to_ work! I tested it! Why doesn't it work anymore?
        - Oh - it's because we now check to ensure that we're really running
          the correct cargo subcmd before generating completions ... but that
          doesn't work if there's an alias.
        - To fix: currently can check `CARGO_ALIAS_$(to-upper subcmd)` and, if that's
          not set, check output of `cargo --list`. Unfortunately the latter is not
          _really_ designed to be machine-readable and there may be escaping problems.
          The latter will definitely be pretty slow unfortunately.
        - Fix when stable: can run
          `cargo +nightly -Z unstable-options config get alias.$subcmd`
    - [ ] in `bash` (and `zsh`)
      `source <(cargo playground completions bash)` overrides
      the effect of `source <(rustup completions bash cargo)`, and vice versa.

- [ ] `cargo playground [args] &| cat` should not contain
  ANSI color by default (because output not a TTY)
    - [x] `cargo playground &| cat`,
      `cargo playground help &| cat` (clap automatically
      determines whether to use color)
    - [ ] `cargo playground list &| cat` (and all other subccommands really)
        - related but not relevant: [see the TIL here](https://stackoverflow.com/a/79614957/1958900).

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

## Misc task

- [ ] passing arguments to script via `cargo playground run $scriptname $args`
  should work _even if_ `$args` contains flags.
  i.e., user can omit "--" and everything after `scriptname`
  will be treated as arguments to the playground (same way as `docker run` works)
    - [x] working when there are no conflicts with
      `cargo playground` options. Fixed by:
      ```
      #[arg(allow_hyphen_values = true, num_args = 0..)]
      pub args: Vec<String>
      ```       
    - [ ] working even when there _are_ conflicts with cargo plaground options
      (e.g., trying to call the script with `--help` or `--version` or
      `--manifest-path`)
        - (it works right now as long as the _first_ argument does not conflict
          with ours)
        - Problem: we basically have to tell clap to collect _everything_ after
          the script name, otherwise `(script-name) --help` is a valid
          command, so it has no reason to consider `--help` as an opaque value
          and start parsing everything as a list of values. It requires
          something to start this off.
        - Collapsing the script name + all its arguments into a single
          `Vec<String>` arg with
          `allow_hyphen_values` + `num_args=1..` +
          `trailing_var_arg` should work ... but the generated help would not
          be right (can you define the struct for the help string differently than
          the actual help string?). Also autocomplete would need to know
          to autocomplete the script name on the very first arg only and then
          return nothing.
            - for autocompletion, you'd implement `ValueCompleter::complete_at`.
              EXCEPT there appears to be an off-by-one bug somewhere?
              It calls complete_at w/ `arg_index=0` for the first TWO arguments
            - for help, can you "override" `render_help` / `render_long_help`?
              probably not.
        - Alternatively, can we customize the parsing itself - instead of
          using `Parser::parse()`, using maybe
          `Command::get_arg_matches` or something?
        - Could also use `Arg::overrides_with_all` (I think?) to manually list all
          of the flags that this thing overrides? Except it does not seem to work.
          Neither does `Arg::exclusive` (in that it prevents all other arguments
          from being parsed)
        - Also having users pass "--" is probably not the worst thing in the world
          ... BUT I keep screwing it up.


- [x] finish splitting this into its own repo
- [x] lifecycle test
    - init a repo, add templates, create scripts, add dependencies, run scripts
- [x] more fun w/ autocompletion:
    - [x] nah ~~don't suggest flags unless current arg starts with a '-' maybe?
      (this is what clap's stable (static) completion implementation does)~~
    - [x] what is right way to install this in fish that doesn't conflict w/
      cargo itself? (answer: autocomplete `cargo` cmd but don't invoke
      clap's autocomplete method unless we're sure that our subcommand
      has been invoked)

- [ ] autocomplete - playing nice w/ others
    - [ ] in bash: sourcing our autocompletions overrides the native
      `cargo` completions
    - [ ] in fish:
      *sourcing* our completions does not override the native completions.
      But adding a `cargo.fish` to our completions directory DOES override the native
      completions (presumably because it stops looking for completions to lazy-load
      after it finds the first `cargo.fish`?)

## Functionality

- [ ] Alfred workflow W/ same autocomplete support as terminal (for me anyway)

### Cargo script support?

- [ ] `cargo script` support -
  manage an ["embedded manifest"](https://rust-lang.github.io/rfcs/3502-cargo-script.html) and keep it in sync w/ our
  `Cargo.toml`
- [x] hide experiments behind a feature flag
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

- [x] require enabling "metadata.playground" flag in Cargo.toml
  before modifying it so you don't accidentally a non-playground.
- [x] local template dir creation / management

## usage as `xtask`

- [x] configurable location for the main project (implemented: use
  `CARGO_PLAYGROUND_MANIFEST_PATH` or `--manifest-path`)
- [ ] get autocomplete working w/ `cargo xtask`?
    - somehow screen it so that it only invokes our autocomplete for "cargo xtask"
      when running in the correct workspace? In the shell script, filters
      on the CWD or something?

### A proper output handling system

- [ ] Centralized color-aware, verbosity-aware output management.
    - Use a logging crate? Is it really "logging" tho?
    - `cargo` uses its own custom output system, is it vendorable?
- [ ] global, configurable w/ term-dependent defaults disabling of **ANSI color**
    - probably https://docs.rs/colorchoice-clap/1.0.8/colorchoice_clap/
    - Note that even with cargo itself, I'm not sure that help's output colors are
      _configurable_ (just auto TTY detection)
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
- [ ] add (optionalyl) "extern crate" statements and/or comments to the
  top of each script when dependencies and/or features are added?
- [ ] use styles from `cli_style` for output everywhere
- [x]  `cargo playground init` -
  to create a whole-ass new project or enable metadata on existing one
    - [x] ~~optionally initialize as xtask? (i.e., add this code to the
      workspace?)~~ nah
- [x] `cargo playground rename`
- [ ] `cargo playground copy`
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
- [ ] Don't say "Updated Cargo.toml" if nothing changed
- [x] Don't run cargo add if no deps will change

#### Pedantic correctness

These are about correctness, including cases even if manifest is manually edited.
These are "pedantic" if they're low likelihood or just Not A Big Deal if they happen.

- [x] validate script names and filenames in `cpg new`
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
