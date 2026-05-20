# Supporting autocompete is unpleasant

## Call forms

Autocomplete support is the cartesian product of all of _at least_ following
factors:

1. Shell (fish, bash, zsh)
2. Running as cargo subcommand or not
3. Name of the `cargo-playground` executable
4. Name of the `cargo` executable
5. Whether `cargo-playground` is in path or not
6. Whether cargo has an alias for our command or not.

The following _probably_ don't matter:

- whether `cargo` itself is on our path or not (not our problem,
  at least not for autocomplete)
- shell aliases (should be expanded well before anything related
  to autocomplete is invoked ... right?)

## Shell setup considerations

1. Lazily loaded (good) or loaded in startup script (slow, wasetful)
2. Does the shell have normal `cargo` completions?
    - Once installed, can it autocomplete `cargo playg[TAB]`?
    - Does activating `cargo playground` completions
      interfere with `cargo` completions?
3. Where do the normal cargo completions come from?
    - You can run `rustup completions $SHELL cargo` to
      generate the script for only bash, zsh (NOT fish)
    - Fish ships with its own built-in cargo completions ...
      so they might break. For homebrew they're in
      `/opt/homebrew/share/fish/completions/cargo.fish`

### Bash notes

- Some sources (including `rustup`) say to put your completions in
  `~/.local/share/bash-completion/completions`.
    - ~~BUT this does not do anything for me on recent bash (5.3.9)
      unless I `brew install bash_completion@2` and then
      source `/opt/homebrew/share/bash-completion/bash_completion`~~
      no, that command actually just installs a set of community
      maintained completions I think?

- Once cargo completions _are_ installed and working, just sourcing
  the `cargo playground` completions will break them.
    - this is also true for cargo-tauri's completion script tho

## Testing

It's easy enough to our program's EnvCompleter itself directly
via the CLI, but testing that it _actually works_ in a shell,
with the generated completion scripts? Much harder.

Unfortunately, the combinatorial explosion of possible ways this
can break means that you basically HAVE TO figure out a way
to test this automatically, because otherwise you're going
to be constantly breaking edge cases (and in a high
dimensional space, *everything* is an edge case).

Good news:
`@epage` maintains a library for this:
`[completest](https://docs.rs/completest-pty/latest/completest_pty/)`.

# Cooperative autocomplete in bash

Unlike fish, `bash` (and `zsh`) appear to only allow
a single completion provider per command. So activating
completions for `cargo playground` (and other tools like `cargo tauri` too)
will override the completions for `cargo` (and each other).

Fix is probably to make the scripts cooperate?
In bash, can use `complete -p` to show if there is a provider associated
with the command.

`bash` associates a completion function w/ a command via
`complete [options] -F (function) (cmd)`; this overwirtes any previous completion.

To see if there are previous completions registered:

```console session
bash-5.3$ complete -p cargo
complete -F _cargo cargo

bash-5.3$ complete -p does-not-exist
bash: complete: does-not-exist: no completion specification
bash-5.3$ echo $?
1
```

For our purposes, ~~it hopefully suffices to just create a new function
that executes both the old _and_ new bash functions?~~ LOL of course not.
Weirdly, even if I call the `_clap_complete_cooperatively` function _first_,
I still only get cargo autocomplete. Maybe there's a stream being consumed
or something, fuck I don't know.

(Interestingly the bash completions do somehow seem to include aliases from
`.cargo/config.toml`, not sure how that's happening)
