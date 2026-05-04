# Program branches and CLI signatures, and cargo

The program will follow one of 3 different branches. They
are dispatched in the following order:

1. **AUTOCOMPLETE**: Is `COMPLETE=$shell` set? In this case, invoke the clap
   autocomplete code (which will exit when done).
2. **CLAP**: Should the program even continue after parsing the CLI arguments?
   (i.e., are they valid? was `--help` passed? etc.) If not,
   clap will exit when it is done.
3. **OPS**: clap returns control to our program and our code actually runs.

The fun part is that these have different CLI signatures, depending on
how it was **INVOKED**
([see also the docs](https://doc.rust-lang.org/cargo/reference/external-tools.html#custom-subcommands))

1. **CARGO SUBCMD**: If you run `cargo playground ..args`,
   cargo will `exec` our executable with an argv of
   `["/path/to/cargo-playground", "playground", ..args]`.
   I.e., there's an extra "subcommand" argument there.
2. Standalone exe: invoked as `["/path/to/exe", ..args]`.
   subcommand (`cargo playground`).

**OPS** and **AUTOCOMPLETE**, should have the same behavior
regardless of whether it was invoked as
`cargo playground ..args` or `cargo-playground ..args`

Help and docs **need to change a little** depending on how it was invoked:

1. text of help and examples need to match how the user invoked our program
   (to the best of our ability, cargo necessarily hides some of that from us).
2. BUT the _structure_ of help should act as if "cargo playground" was a single
   command, not cmd + subcommand. Should not ever output help for
   the base "cargo" command.

## How does cargo discover subcommands?

AFAICT: any executable named `~/.cargo/bin/cargo-whatever` can be
invoked as `cargo whatever` - it just looks at filenames, there's
no registry or anything. So, generally, you can't assume you know
what the subcommand's name will be.

## How do we _detect_ when we've been invoked by cargo?

The only reason we need to know this is unfortunately a very important
one - we need to know how to process the CLI args.

TLDR: you can't know for certain due to edge cases. But I doubt
it's a problem in practice.

When cargo `exec`s our executable:

1. it set the CARGO and CARGO_HOME env vars (although
   we have no way of knowing whether these weren't already set)
2. `argv[0]` will _always_ be our exe (or a symlink pointing to it I guess),
3. `argv[1]` will to  _always_ be the name of the executable after `-playground`.
   (this is not affected if you use aliases, and any flags passed
   to cargo itself are discarded)

If any of these are _not_ true, then we're _not_ running as a cargo subcmd.

Unfortunately while these are all _necessary_ to detect whether we
were invoked via cargo, none is _sufficient_. E.g., what if our program
has a subcommand called `playground`? Or what if it was for some reason
installed as `cargo-quick` ... which collides with one of its subcommands?
These are all edge cases that are not super-important tbf.

You _could_ have a fallback where, if all the heuristics above match, we
try to parse as cargo subcmd, then fall back to parsing as direct
invocation.

## How do we get everything to work right in every situation?

- when generating **help**, we might need to trick it into using
  the correct invocation,
  but ensure that it does not start outputting help as if it were the base
  `cargo` command.
    - is this a problem in practice? This does not get invoked
- When clap is parsing our arguments, we need to basically discard the
  cargo subcmd ... but in a way that does not compromise error messages
  it generates?

#### Q: Can we simply pretend to be cargo when appropriate?

This is actually what the [clap example for this situation does in all situations](https://github.com/clap-rs/clap/blob/master/examples/cargo-example-derive.md).
I.e., it _must_ be called as
`cargo-example-derive example-derive`, and if you ask for
top-level help, it claims that it is cargo. I don't like that.

Very few other cargo subcommands actually do this.
The vast majority do try to explicitly detect whether running as a cargo subcommand or not
and work in either case. (`cargo-audit` is the exception that works like this).

#### Other tools

nb none of these adjust their help strings, they always say that their
usage is `cargo $name` even when called as `cargo-$name` (or vice versa for
`cargo-deny`).

- [How clippy does it without clap.](https://github.com/clap-rs/clap/blob/master/examples/cargo-example-derive.md)
- [How cargo-fmt does it, with clap](https://github.com/rust-lang/rustfmt/blob/ef22670a/src/cargo-fmt/main.rs#L100-L110). They always strip out the
  _first_ instance
  of an arg called "fmt" (harcoded string) from argv, not even necessarily `argv[1]`.
- cargo audit acts like the clap example - it
  _always_ pretends to be cargo so you have to run
  `cargo-audit audit`.
- Directly calling `cargo-bloat`, OTOH, makes it say "can only be run via
  `cargo bloat`. (when did I install this? was it a good idea?)

# Approach

Generally we have 2 options: either A) let clap access `env::argv()` directly
(and provide it with the appropriate parser) or B) just build one parser but
give it a custom argv. Tried A, it's a bit of a mess tbh, let's just do **B**.

We'll still need to set up the bin_name.

## Issues

So the clap autocomplete script is going to call cargo as
`COMPLETE=fish cargo -- cargo playground args`??

But cargo then thinks it's being called as `cargo cargo` and crashes
out? So wait how did this ever work in the first place? _Did it?_

In any case, the only way around this is to _not call cargo to generate
completions in the first place_; we need to call `cargo-playground` directly
when autocompleting. This does however mean it's gonna get called along
the lines of `/path/to/cargo-playground -- cargo playground arg1 arg2 [...]`,

Let's call the argument slice after `--` "`arg_autocomp`" to differentiate
it from the full argv.

So we still need to make the autocomplete parser understand that
there's an additional `arg_autocomp[1]` to ignore.

Options:

- is it still possible to strip `arg_autocomp[1]` here?
    - not sure if can do that in autocomplete API anymore
    - this invocation is NOT going through
      `cargo` (shell is directly invoking our exe here)
      so `$CARGO` will not be set.
    - also if you do this then "cargo" will get autocompletion options from cargo-playground
      which is not what we want.
- Run autocomplete with a special `Command` that basically ignores `arg_autocomp[1]`?
    - This is ok, we don't even have to worry about help strings or anything here.
    - BUT how do we know which `Command` to use?
        - Try both? Meh
        - Does clap allow optional positionals followed by required positionals?
          (hopefully not tbh).
        - Heuristic: check if the _actual_ `argv[0]` matches
          `arg_autocomp[0]`? ehh ... would
          this false positive on symlinks? Figuring this out is not our job.
        - Somehow pass information in the generated shell script to differentiate between
          direct call mode vs subcommand mode? E.g.
          **use a different env var** for the
          2 situations? ACTUALLY I like this! it's what env vars are for.

          
