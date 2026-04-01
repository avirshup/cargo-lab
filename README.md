# `cargo-playground`

A tool for quick, disposable, local, and IDE-friendly "playground" scripts.

#### Why?

You can navigate [https://play.rust-lang.org/](https://play.rust-lang.org/) and,
in about 10 seconds, try out some rust code or even play around with a few (curated)
crates. `cargo playground`'s goal is to make it just as easy to do the same
thing as the official rust playground, but:

1. on _your_ machine,
2. with _your_ IDE, and
3. with [any set of dependencies](#hygiene) and features that you care to try.

## Quick start

This is a CLI tool, install it with `cargo install cargo-playground`.

A playground is really just a normal crate with lots of "scripts"
(binary targets) in it, each with its own dependencies.
To create and run your first script:

```bash
# create or initialize a playground project
# # NOT ACTUALLY IMPLEMENTED YET
cargo playground init $path_to_playground

# Create a new script at "src/my_script.rs"
cargo playground new my-script dep1 -F dep/feature

# Add dependencies and activate features for a script
cargo playground inject script-name dep1 dep2 -F dep1/featurename -F dep2/featurename

# Run a script
cargo playground run script-name
```

## Don't install untrusted dependencies.

<a name="hygiene"></a>

A playground is _not_ a isolation sandbox.
A `cargo playground`-managed "playground" is
_a standard, local cargo project_ - the entire goal is to behave
composably will all of your rust dev tooling.

That means there is no isolation layer here.
Running these playgrounds is equivalent to running `cargo --bin $bin_name`
in any other local cargo project. Building and/or running a playground script
with a malicious dependency
is [exactly as dangerous](https://socket.dev/blog/malicious-rust-crate-evm-units-serves-cross-platform-payloads) here as it would be in any other cargo project.

## The basic idea

A "playground" here means a cargo project with lots of one-off scripts in it.
You can initialize one by running `cargo playground init $path`.

**NOT FULLY IMPLEMENTED YET: init, completions**

```console-session
$ cargo playground init ./my-new-playground
Success: created playground in ./my-new-playground

To make this your default playground, set
`CARGO_PLAYGROUND_MANIFEST_PATH=/path/to/my-new-playground`
In your shell (fish), you can set this persisently by running:
  set -Ug CARGO_PLAYGROUND_MANIFEST_PATH "/path/to/my-new-playground"
  
To enable autocomplete, run `cargo playground install-completions`.

$ cd my-new-playground
$ tree .
./
├── Cargo.toml
├── src
│   └── new_playground.rs
└── templates
    ├── bare.rs.template
    └── clap.rs.template
```

This will create a completely normal cargo project that the IDE of your
choice will happily work with. Everything `cargo playground` does is just
sugar on top of normal cargo commands. You can and should edit the generated
Cargo.toml if you want, it will not interfere with this command.

Each script managed by `cargo-playground` will have an entry like this
in `Cargo.toml`:

```toml
[[bin]]
name = "my-script-name"
path = "./src/my_script_name.rs"
required-features = ["dep1", "dep2", "dep3/feature"]
```

### Typical Workflow

For example, let's say I want to play around with proc macros. First we'll create the script:

**TODO: thinking of names every time is too much work, `cpg new` needs a way to
generate names, maybe by topic. Also these commands are kind of long to type**

```console session
$ cargo playground new proc-macro-experiment
success: Created script: templates/bare.rs.template -> src/proc_macro_experiment.rs
Updated Cargo.toml:
40a41,45
> [[bin]]
> name = "proc-macro-experiment"
> path = "src/proc_macro_experiment.rs"
> required-features = []
```

And then add the dependencies and features we want to use:
**TODO: having to type out the whole name again is like a drag, man.
Maybe "@latest" to hit the latest script created? Or just allow
these dependencies to be specified in the initial `new` command?**

**TODO: output formatting is still somewhat aspirational**

```console session
$ cargo playground inject proc-macro-experiment syn quote proc-macro2 --feature syn/parsing
running: "$ cargo add --optional syn quote proc-macro2"
    Updating crates.io index
      [...]

Updating features for "proc-macro-experiment" in Cargo.toml:
    < required-features = []
    ---
    > required-features = ["syn", "quote", "proc-macro2", "syn/parsing"]  
```

Then, you can play with it in the IDE of your choice and run it:

```console session
$ rustrover .  # start my IDE
$ # ... make changes ... 
$ cargo playground run proc-macro-experiment
my experiment's output goes here
```

## Configuration

### In `Cargo.toml`

**TODO Not actually implemented yet**

`cargo playground` stores configuration playground's `Cargo.toml` in the
`[package.metadata.playground]` table. `cargo playground` will
not perform any actions in a crate unless this table is present.

```toml
[package.metadata.playground]
# editable: optional, set to `true` on initialization, defaults to "false" if not present.
#    Must be "true" to enable updates to `Cargo.toml`
editable = true

# editor-cmd: optional, default=value of `$EDITOR` env var if set, otherwise `null`.
#   Specifies command to launch an editor when requested with
#   `--edit` flag or `edit` subcommand.
editor-cmd = "rustrover"
```

TODO: distinction between blocking and non-blocking editors?
Or just always finish everything up, then, (on POSIX anyway)
`exec` the editor command at the end?

## Why?

Why I use this:

1) for me, rust is a very IDE-dependent language, so even
   if I'm working with a one-off experiment, I'd really like to have
   all the autocompletion and pop-up docs etc. available.
2) It's a resaonable alternative to the REPL-/jupyter- driven exploration you can do
   in more dynamic languages.
3) It's nice to run experiments locally and maybe even keep them in a repo.
4) It seems excessive to create entire new cargo projects for each one-off
   experiment.

### Alternatives

- [The Official Rust Playground](https://play.rust-lang.org/) is wonderful and
  maybe the quickest way to write some rust code. But what if you want to try
  out a dependency that's not in top 100, or use your local IDE, or even just run locally?
    - There is also
      [Rust explorer](https://www.rustexplorer.com), also web based but supports the top
      _10k dependencies_, not just top 100.
    - [There is (was?) funding to improve web-based rust "playground" ecosystem](https://users.rust-lang.org/t/call-for-contributors-to-the-rust-playground-for-upcoming-features/87110?u=epage),
      which is great.


- single file [cargo scripts](https://rust-lang.github.io/rfcs/3424-cargo-script.html) are also a great idea, but don't yet have IDE
  support. For my playground experiments specifically, I also kind of like having
  a single `Cargo.toml` for everything.

- [
  `evcxr`](https://github.com/evcxr/evcxr) is an insanely impressive REPL for rust (which
  _should not be even possible_).

- A normal crate with lots of `[[bin]]` entries in
  `Cargo.toml` is kind of a pain to manage,
  especially if you want to have lots of different dependencies for all the different
  binaries.
    - This process is exactly what this crate automates.
