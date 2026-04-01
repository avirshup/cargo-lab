_# How to get dynamic autocomplete working

`clap_complete` has 2 different, possibly incompatible ways
to generate the autocomplete config for a given shell.
The _supported, stable_ version uses `clap_complete::generate`
but _only supports static (compile-time) completions_, you
cannot generate them dynamically.

So how do you this for _dynamic_ completions?

TLDR, lesson learned: should have been looking at `jj` as an example.

## `unstable_dynamic` and `clap_complete::CompleteEnv::complete`

This uses the `clap_complete/unstable_dynamic` feature,
which is for example in use by such tools such as
e.g. for instance `jj` and `cargo`
(in nightly). This is mostly disjoint path from `clap_complete`'s stable
path (`clap_complete::generate`). Here
you running `COMPLETE=$SHELLNAME myprogram`.
It generates a much smaller script that looks generic -
it delegates all the completions to your binary
calling into the program with some special env vars set.

To get this to work, your program needs to call [
`CompleteEnv::complete`](https://docs.rs/clap_complete/latest/clap_complete/env/struct.CompleteEnv.html) as
early as possible so that the autocomplete engine
can take over execution if it's being requested.

In fish it looks like this
`COMPLETE=$SHELLNAME cargo-playground` (options slightly reordered for readability)
(linebreaks inserted for clarity, the real version does not contain any linebreaks):

```bash
complete --command cargo-playground \
    --keep-order --exclusive \
    --arguments \
    "(
        COMPLETE=fish cargo-playground --
        (commandline --current-process --tokenize --cut-at-cursor)
        (commandline --current-token)
    )"
```

This is, I believe, a fully generic shim (except for the name of the command) that
just generates all autocomplete options at runtime in the binary, even the stuff
that could be theoretically static. Seems like a great model to me,
it means you get to write the autocomplete logic once in rust instead of N different shells.
(Actually seems like you're nearing something LSP-like, but for shells, here?)

## Is it usable?

I think so, for instance it's used [by
`cargo` (nightly)](https://github.com/rust-lang/cargo/blob/3185f58b/src/bin/cargo/main.rs#L31-L38)
and [stable/prod
`jj`](https://github.com/jj-vcs/jj/blob/fdf9f55/cli/src/cli_util.rs#L3997).
`jj`.

JJ actually provides both the static and the dynamic completion scripts as options, since dynamic is unstable. [The docs say this](https://docs.jj-vcs.dev/latest/install-and-setup/#dynamic-completions):

> Generally, dynamic completions provide a much better completion experience.
> Although the underlying engine is deemed unstable, there have not been many
> issues in practice. Dynamic completions are the preferred option for many
> contributors and users.
>
> We recommend using the dynamic completion script, and falling back to the
> standard completion script if there are any issues.

It also says that
`No configuration is required with fish >= 4.0.2 which loads dynamic completions by default.`
Q: Wait how the hell does that work?<br>
A: Oh, fish ships with this [built-in for
`jj`](https://github.com/fish-shell/fish-shell/blob/4.0.2/share/completions/jj.fish), it's not, like, a general autocomplete discovery
mechanism or anything.

## On usage in prod

At least in the case of
`jj`, they are doing [a
_lot_ of preprocessing and other work](https://github.com/jj-vcs/jj/blob/fdf9f55/cli/src/cli_util.rs#L3935-L3995) before invoking
`CompleteEnv::complete`. Although most / all of that seems to be about user-defined aliases I think,
so it might not be relevant here.

# Parsing toml w/ serde?

The manual parsing (and error handling!) via `toml_edit`'s interface is getting old.
It has a serde feature, can I use that?
But weirdly it seems to deserialize documents and values BUT NOT tables?
So either I need to parse the whole thing or parse values, but can't parse tables?
Maybe this is because a given table's subtables might exist somewhere else in the toml?

## How cargo does it

It looks like `cargo` uses `toml`, not `toml_edit`, for reading the file.
When _editing_ the manifest with
`toml_edit`, it looks like they do it [imperatively without
a schema](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml_mut/manifest.rs).

- [Here is the serde schema](https://github.com/rust-lang/cargo/blob/710cce58b/crates/cargo-util-schemas/src/manifest/mod.rs#L37).
- Here is how they [read](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L155-L165), [parse](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L168-L182), and finally [deserialize](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L185-L197)
  a manifest file.

## How we can do it here

- steal (a subset) of the official cargo

# How to _install_ autocompletions automatically

TLDR: don't.

Slightly longer: this is a cursed topic, even by the cursed
standards of shell scripting. In fish it's _slightly_ less fucked
but it's still a huge fucking pain there with like 10 different
places you might want to install them, and then it turns out
this is really NOT something your program should even attempt
to do anyway, either a package manager should do it or the user
needs to set up their env theemselves
(i.e., put `COMPLETE=bash myprogram | source` in their profile,
or run `COMPLETE=fish myprogram > ~/.config/fish/completions`,
or etc etc.)

## Fish

https://fishshell.com/docs/current/completions.html#where-to-put-completions
the docs say to use `pkg-config --variable completionsdir fish` to discover
the proper place to put completions. We'll do this ... but no other tools I use
have actually put their completions there on my machine, they have all used
`~/.config/fish/completions.d`

ACTUALLY I think this is simply not correct (or at least not working right
for homebrew). On my machine, it returns something in the homebrew "cellar"
`/opt/homebrew/Cellar/fish/[...]`, which nothing else uses and is almost
certainly the wrong place to install anything.
(The right place is /opt/homebrew/share/fish/vendor_completions.d)

ACTUALLY NO for several reasons - `vendor-completions.d` or whatever
is for completions that are included w/ packages and shipped. These
are _generated_, so they go in the _user's_ completions file. I think.
ACTUALLY ACTUALLY ACTUALLY shouldn't it go in the same prefix as cargo?
oh my god fuck it.

### note on debugging completions

If you install the completions into any of the completion directories, it appears
that they will only be loaded when you run the executable when it's in your `$PATH`,
not if you refere to it via a path. So it's probably easiest to `source` it when
trying it out in dev.

### Note on how well it actually works

So, `clap_complete` is not quite as, well, feature-complete as I thought
it would be?

- does not retain order? (`-k` option)
- stable has no support for custom completions - not even compile-time static ones
- You can enable the dauntingly named `unstable-dynamic` crate feature, which
  does provide more support for customization, [e.g.](https://docs.rs/clap_complete/latest/clap_complete/engine/struct.ArgValueCompleter.html).

## Not fish

Aside from fish (which has apparently best-in-class documentation), I cannot
find straightforward documentation of how to do this for any other shell.
From MSDN blog posts (sigh) it looks like in powershell you'd _run commands_
to do this, not write files.

The best I can find:

- [`shellcomp`](https://github.com/lvillis/shellcomp-rs)
  a very recent libray for
  *installing* completions, i.e., exactly this. Seems a little
  complex, but even if the shells themselves were simple to work with, we're
  messing with user (and in some cases system-wide) shell config, of course
  it's complex.
    - however this is from someone I don't know, it does not have any stars,
      it originates during the AI slop era, and it's complex enough that I don't
      want to audit it.
- ZSH: stack overflow answer (https://stackoverflow.com/a/67161186/1958900)
- autocompletion scripts for "starship" on my machine were installed automatically along w/ homebrew

# In-memory manifest lifecycle

So, our commands both:

- use the manifest as a source of configuration, but also
- edit the manifest, popentially including the configuration embedded in it.

While this almost feels like an architecture issue,
this is not that unusual, plenty of commands (including cargo) can
edit their own configuration.

## The "right" way

The cleanest architecture, I suppose, would be separate this into
2 layers, with a lower-level config concern below the business
concerns. And this would not need to actually be exposed through
the CLI, you'd just route the user to the appropriate layer
for their request.

On first glance, the biggest advantage here would be error
handling - since config may not even exist, or it may be
invalid even if it does exist, you need config operations
to be their own set of things.

I'm not sure I want to bother with that or even research how
other tools do it here though.

## Architecture -> lifecycle -> data model

Well I guess the lifecycles will reflect the architecture:

1. First we try to load as much configuration as we can, including
   a _snapshot_ of the manifest as it originally exists, if it exists.
   This needs to happen quite early of course, and it A) must not panic,
   B) must not produce any stdout, and C) must preserve any errors so the
   business layer can deal with them.
    - after loading, that all becomes immutable
2. Then we pass the results of all that to the business layer to deal with
   as it sees fit.

This overlaps with a couple of cases I already have in the code where
it returns a `Result<Option<Thing>>` or `Option<Result<Thing>>`.
Makes me thing these should either be turned into their own `ThingOutcome`
enums OR collapse whatever the `Option` variants represent into the `Result`
part of the enum. Idiomatically the latter probably makes more sense?

Except that having a manifest not exist is not actually necessarily an "error"
in certain cases, so maybe idiomatically having it be a custom enum
and only turning it into an error in the cases where it constitutes one is better?

## Config layers and fallibility

We can consider each of these a layer that may or may not fail; if it
fails, we can't load the subsequent layers.

1. Core environment config (just panic if we can't even figure these out)
    - logging verbosity
    - working dir
    - cargo exe path (must either be `$CARGO` env var or just `cargo`)
        - this does not really need to be here, but it seems unlikely
          to be a problem in practice
2. Manifest discovery:
    - manifest file/dir path
    - template dir path
3. Manifest parsed at TOML level: `ManifestToml`,
   then in-memory TOML deserialized into structs: `ManifestData`
   (parsing and deserialization could be separate layers but in pratice
   I don't _think_ we will ever need one without the other)
4. `PlaygroundConfig` extracted from `ManifestToml`.

Which operations need which layers?

- All commands that actually enter our code require at least layer 1 (clap can handle
  `--help` and `--version` without even layer 1, I guess).
- Initializing a new playground project needs only layer 1.
- Upgrading an existing project into a playground needs layer 3.
- All other operations basically need layer 4.
- Autocompletions can *use* layer 4 but must handle every possible
  fallibility scenario gracefully enough.

## Brainstorming: What is the data model?

So this actually would seem to trigger my DTO annoyances,
where we have a bunch of types with subsets of each others'
fields.

(Composable record types? Which are kind
of like inheritance?)

Leaning towards: use the `ConfigLoader` / DI provider style for this.

### Whole enchilada-style, with different sauces for different commands

Basically different types of config - as necessary for each command?
e.g., for an L4 command:

```rust
pub struct FullConfig {
    // L1
    pub verbosity: Verbosity,
    pub cwd: PathBuf,
    pub cargo_exe: PathBuf,

    // L2
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,
    pub cargo_exe: PathBuf,

    // L3
    pub toml: ManifestToml,
    pub data: ManifestData,

    // L4
    pub pgconfig: PlaygroundConfig
}
```

### By layer

```rust
pub struct Config {
    env: EnvConfig,
    paths: Result<PathConfig>,
    manifest: Option<Result<Manifest>>, // not even a `Result` if we can't get paths
    pgconfig: Option<PlaygroundConfig> // from config.toml metadata
}

pub struct EnvConfig {
    pub verbosity: Verbosity,
    pub cwd: PathBuf,
    pub cargo_exe: PathBuf
}

struct PathConfig {
    pub manifest_dir: PathBuf,
    pub template_dir: PathBuf,
    pub cargo_dot_toml: PathBuf,
    pub cargo_exe: PathBuf,
}

struct Manifest {
    pub toml: ManifestToml,
    pub data: ManifestData,
}
```

### Au naturel, no config data model

Q: Why do we even, like, categorize our config into `struct`s
Isn't that just want the object-oriented MAN wants us
to do? Why not just load everything up in `main` as needed and use dependency
injection to inject it all into the command themselves?

A: Because that sounds like an imperative untestable mess of
copy-and-pasted config code.

Anyway, I _like_ modules. And config loading is complicated, it should
be done intentionally and with a firm model.

### ConfigLoader - cached/on-demand configs

```rust
pub struct ConfigLoader {
    /* ...private fields, with interior mutability for caching... */
}

impl ConfigLoader {
    pub fn manifest_paths(&self) -> Paths { /*...*/ }
    pub fn playground_config(&self) -> Result<PlaygroundConfig> { /*...*/ }
    // etc
}
```

This starts to look like a simple DI provider ... I _like_ DI providers.
They can decouple runtime depenendency DAGs from the usage of
any specific dependency.

- **Problem**: always need to unwrap or handle errors every time you ask the
  loader for something even if you know it already has it.
    - **Solution**: ConfigLoader needs to live at the top level and inject early;
      this also helps separate config loading errors from the rest of the possible
      runtime errors.
- **Problem**: there's still a lot of config to pass around, gonna
  have some long-ass function signatures, and the actual passing
  will still need a lot of unwrapping and map_errs.
    - **Solution 1**: we can combine this with the "by-layer" config structs
      to group things logically - this does result in somewhat higher
      coupling between layers tho.
    - **Solution 2**: some form of DI _framework_ ... sigh.

### "Services"?

Instantiate "service" objects that have the appriate bits of config baked into them.
This probably works well with a `ConfigLoader`. So the lifecycle is, for a given
command, the configbuilder builds a service, then we run the appropriate
command on that service. Many commands can likely be provided by a single "service".

This feels like an almost tautological decoupling? And very enterprise-architecture-ish
architecture (i.e., more than it needs).

## How does cargo do it?

Sigh, I guess I'm figuring this out. So, it has a
[
`GlobalContext`](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/util/context/mod.rs#L211) struct, which "is not specific to a build, it is information
relating to cargo itself" - e.g., it does not have any information related to a specific
cargo.toml.

When _initialized_, it only checks to lazy-loads information as needed,
and can only fail if `cwd` or `$HOME` can't be determined. The context
struct is therefore filled with private `OnceLock<T>` (thread-safe
`OnceCell`s) fields,
and the implementation has a bunch of getters. So that's the configloader pattern.

The actual manifest-related stuff is handled in a [
`Workspace`](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/core/workspace.rs#L66) that retains a (shared) borrow of the global context.
Unlike the gctx, [this loads somewhat eagerly](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/core/workspace.rs#L235).
