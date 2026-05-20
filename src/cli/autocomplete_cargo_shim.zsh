{#
    NOTE: this is a jinja template, rendered in completion_script.rs
#}
# ───── Setup ──────────────────────────────────────────────────── #
_registered_completion_fn() {
  # Get the completion function registered for a given command, if it exists
  # Prints nothing if it doesn't exist.

  local cmd="$1"
  local registered
  registered=$(echo "$_comps[$cmd]" 2>/dev/null) ||
    return 0
  test -n "$registered" && echo "$registered"
}

# ───── The actual completion function ─────────────────────────────────── #
_CARGO_PG_PARENT_COMPLETER=$(_registered_completion_fn "{{cmd}}")

_complete_cargo_pg_combined() {
  if test -n "$_CARGO_PG_PARENT_COMPLETER"; then
    "$_CARGO_PG_PARENT_COMPLETER" "$@"
  fi

  _clap_dynamic_completer_{{name}} "$@"
}

# ───── Registration ───────────────────────────────────────────── #
# clap complete script
{{clap_completion_script}}

# *override* clap script's registration with our "combined" provider
compdef _complete_cargo_pg_combined {{cmd}}
