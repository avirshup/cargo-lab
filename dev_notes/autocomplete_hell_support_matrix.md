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
