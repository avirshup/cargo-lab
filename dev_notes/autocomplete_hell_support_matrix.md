# Supporting autocompete is unpleasant

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
