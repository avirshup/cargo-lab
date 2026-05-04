# How to get dynamic autocomplete working

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
(linebreaks inserted for clarity):

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

### So what does that actually do?

Say you type (in fish): `cargo-playground run ab --glb=` then hit _tab_.

This causes fish to run the following command, in the background:

```bash
(
  COMPLETE=fish  # set the env var
  cargo-playground # run our command
  -- # very important!
  'cargo-playground' 'run' 'ab'  # result of the first `commandline` call
  '--glb='  # result of the `commandline --current-token` call
)
```

- This only includes only the arguments _before to your
  cursor_, everything after is ignored.
  I'd imagine you could write a `commandline` command to pass everything after
  as well, if your autocomplete system wanted it
    - (tbh would be cool to have autocomplete that took text _after_ the cursor
      into account too. not sure if other shells could do that tho. Also that
      seems like generating suggestions is a much harder problem there)
- If your cursor is not directly to the right of to a token (i.e., you're
  not currently typing an arg) it will send an empty string `""` as the
  last argument.

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
