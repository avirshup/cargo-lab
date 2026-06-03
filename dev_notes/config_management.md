# In-memory manifest lifecycle

So, our commands both:

- use the manifest as a source of configuration, but also
- edit the manifest, popentially including the configuration embedded in it.

Is this an architecture issue (or skill) issue? No that unusual, plenty of CLI
commands (including cargo, jj, git, etc.) edit their own configuration, also
every GUI app with a preferences menu can do that too.

## The "right" way

The cleanest architecture, I suppose, would be separate this into 2 layers, with
a lower-level config concern below the business concerns. And this would not
need to actually be exposed through the CLI, you'd just route the user to the
appropriate layer for their request.

On first glance, the biggest advantage here would be error handling - since
config may not even exist, or it may be invalid even if it does exist, you need
config operations to be their own set of things.

I'm not sure I want to bother with that or even research how other tools do it
here though.

## Architecture -> lifecycle -> data model

Well I guess the lifecycles will reflect the architecture:

1. First we try to load as much configuration as we can, including a _snapshot_
   of the manifest as it originally exists, if it exists. This needs to happen
   quite early of course, and it A) must not panic, B) must not produce any
   stdout, and C) must preserve any errors so the business layer can deal with
   them.
   - after loading, that all becomes immutable
2. Then we pass the results of all that to the business layer to deal with as it
   sees fit.

This overlaps with a couple of cases I already have in the code where it returns
a `Result<Option<Thing>>` or `Option<Result<Thing>>`. Makes me thing these
should either be turned into their own `ThingOutcome` enums OR collapse whatever
the `Option` variants represent into the `Result` part of the enum.
Idiomatically the latter probably makes more sense?

Except that having a manifest not exist is not actually necessarily an "error"
in certain cases, so maybe idiomatically having it be a custom enum and only
turning it into an error in the cases where it constitutes one is better?

## Config layers and fallibility

We can consider each of these a layer that may or may not fail; if it fails, we
can't load the subsequent layers.

(Note from later: don't need explicit layers, a caching config loader makes it
easy to just define a DAG)

1. Core environment config (just panic if we can't even figure these out)
   - logging verbosity
   - working dir
   - cargo exe path
     - When run from cargo itself, this will always be the $CARGO env var.
     - Is it worth making its own layer? In misconfigured environments at least
       a nice error message is better than not ...
2. Manifest discovery:
   - manifest file/dir path
   - template dir path
3. Manifest parsed at TOML level: `ManifestToml`, then in-memory TOML
   deserialized into structs: `ManifestData` (parsing and deserialization could
   be separate layers but in pratice I don't _think_ we will ever need one
   without the other)
4. `LabConfig` extracted from `ManifestToml`.

Which operations need which layers?

- All commands that actually enter our code require at least layer 1 (clap can
  handle `--help` and `--version` without even layer 1, I guess).
- Initializing a new lab project needs only layer 1.
- Upgrading an existing project into a lab needs layer 3.
- All other operations basically need layer 4.
- Autocompletions can _use_ layer 4 but must handle every possible fallibility
  scenario gracefully enough.

## Brainstorming: What is the data model?

So this actually would seem to trigger my DTO annoyances, where we have a bunch
of types with subsets of each others' fields.

(Composable record types? Which are kind of like inheritance?)

Leaning towards: use the `ConfigLoader` / DI provider style for this.

### Whole enchilada-style, with different sauces for different commands

Basically different types of config - as necessary for each command? e.g., for
an L4 command:

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

Q: Why do we even, like, categorize our config into `struct`s Isn't that just
want the object-oriented MAN wants us to do? Why not just load everything up in
`main` as needed and use dependency injection to inject it all into the command
themselves?

A: Because that sounds like an imperative untestable mess of copy-and-pasted
config code.

Anyway, I _like_ modules. And config loading is complicated, it should be done
intentionally and with a firm model.

### ConfigLoader - cached/on-demand configs

```rust
pub struct ConfigLoader {
    /* ...private fields, with interior mutability for caching... */
}

impl ConfigLoader {
    pub fn manifest_paths(&self) -> Paths { /*...*/ }
    pub fn lab_config(&self) -> Result<PlaygroundConfig> { /*...*/ }
    // etc
}
```

This starts to look like a simple DI provider ... I _like_ DI providers. They
can decouple runtime depenendency DAGs from the usage of any specific
dependency.

- **Problem**: always need to unwrap or handle errors every time you ask the
  loader for something even if you know it already has it.
  - **Solution**: ConfigLoader needs to live at the top level and inject early;
    this also helps separate config loading errors from the rest of the possible
    runtime errors.
- **Problem**: there's still a lot of config to pass around, gonna have some
  long-ass function signatures, and the actual passing will still need a lot of
  unwrapping and map_errs.
  - **Solution 1**: we can combine this with the "by-layer" config structs to
    group things logically - this does result in somewhat higher coupling
    between layers tho.
  - **Solution 2**: some form of DI _framework_ ... sigh.

### "Services"?

Instantiate "service" objects that have the appriate bits of config baked into
them. This probably works well with a `ConfigLoader`. So the lifecycle is, for a
given command, the configbuilder builds a service, then we run the appropriate
command on that service. Many commands can likely be provided by a single
"service".

This feels like an almost tautological decoupling? And very
enterprise-architecture-ish architecture (i.e., more than it needs).

## How does cargo do it?

Sigh, I guess I'm figuring this out. So, it has a
[ `GlobalContext`](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/util/context/mod.rs#L211)
struct, which "is not specific to a build, it is information relating to cargo
itself" - i.e., it does not have any information related to a specific
cargo.toml.

When _initialized_, it only checks to lazy-loads information as needed, and can
only fail if `cwd` or `$HOME` can't be determined. The context struct is
therefore filled with private `OnceLock<T>` (thread-safe `OnceCell`s) fields,
and the implementation has a bunch of getters. So that's the configloader
pattern.

The actual manifest-related stuff is handled in a
[ `Workspace`](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/core/workspace.rs#L66)
that retains a (shared) borrow of the global context. Unlike the gctx,
[this loads somewhat eagerly](https://github.com/rust-lang/cargo/blob/17b41ae86/src/cargo/core/workspace.rs#L235).

## Parsing toml w/ serde?

The manual parsing (and error handling!) via `toml_edit`'s interface is getting
old. It has a serde feature, can I use that? But weirdly it seems to deserialize
documents and values BUT NOT tables? So either I need to parse the whole thing
or parse values, but can't parse tables? Maybe this is because a given table's
subtables might exist somewhere else in the toml?

### How cargo does it

It looks like `cargo` uses `toml`, not `toml_edit`, for reading the file. When
_editing_ the manifest with `toml_edit`, it looks like they do it
[imperatively without a schema](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml_mut/manifest.rs).

- [Here is the serde schema](https://github.com/rust-lang/cargo/blob/710cce58b/crates/cargo-util-schemas/src/manifest/mod.rs#L37).
- Here is how they
  [read](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L155-L165),
  [parse](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L168-L182),
  and finally
  [deserialize](https://github.com/rust-lang/cargo/blob/3185f58bbb76877cb80a8d39225eb70bd50f590b/src/cargo/util/toml/mod.rs#L185-L197)
  a manifest file.

### How we can do it here

- steal (a subset) of the official cargo

### Deep thoughts

- Config management has certain ... simliarities ... to the problemss of cache
  invalidation and naming things. (Esp when your process, as a user-friendly
  tool, needs to be able to edit and update its own config ...)
- On code re-use and abstraction: probably don't abstract something you only
  need once. BUT "once" can be temporal, not just spatial. E.g., for a problem
  where you're trying multiple approaches, an abstraction is very helpful _even
  if_ any given snapshot of the code only actually uses it once?
  - It's basically scaffolding for prototying in this case ... Do you keep it
    around afterwards, as reified knowledge, or throw it away and remove the
    scaffolding once you're done? (theoretically you could always retrieve the
    abstraction later from the repo's history if you can find it again).
  - Probably comes down to how much YAGNI and premature generalization is
    present + and the cost of maintaining it vs. the semantic value of keeping
    it around.
  - If removing it, probably leave a comment somewhere pointing to a a commit it
    can be rescued from if you need it again? Generalizes to the usual problem
    of what to do with dead code that you may reasonably need later but not
    right now.
