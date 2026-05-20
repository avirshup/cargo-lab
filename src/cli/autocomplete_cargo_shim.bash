{#
    NOTE: this is a jinja template, rendered in completion_script.rs
#}
# ───── Setup ──────────────────────────────────────────────────── #

# Get the completion function registered for a given command, if it exists
# - If there is no function registered, prints nothing and returns 0
# - If there is an error, prints nothing and returns 1
# - Otherwise prints the name of the function and returns 0
_bash_registered_completion_fn() {
  local cmd="$1"

  # output is of form "complete [...] -F $function $cmd"
  # ... and I guess we're on our own for parsing it
  local compfn
  compfn=$(complete -p "$cmd" 2>/dev/null) ||
    return 0

  [[ "$compfn" =~ [[:space:]]-F[[:space:]]*([^[:space:]]+) ]] &&
    test -n "${BASH_REMATCH[1]}" &&
    echo "${BASH_REMATCH[1]}"
}

# ───── The actual completion function ─────────────────────────────────── #
_CARGO_PG_PARENT_COMPLETER=$(_bash_registered_completion_fn "{{cmd}}")

_complete_cargo_pg_combined() {
  local all_completions=()
  if test -n "$_CARGO_PG_PARENT_COMPLETER"; then
    "$_CARGO_PG_PARENT_COMPLETER" "$@"
    all_completions+=("${COMPREPLY[@]}")
  fi

  _clap_complete_{{name}} "$@"
  COMPREPLY+=("${all_completions[@]}")
}

# START: script emitted from clap::CompleteEnv
{{clap_completion_script}}
# END: script emitted from clap::CompleteEnv


# ───── Registration ───────────────────────────────────────────── #
if test -n "$_CARGO_PG_PARENT_COMPLETER"; then
    # overwrite the completion registration (again) with our function
    if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
      complete -o nospace -o bashdefault -o nosort -F _complete_cargo_pg_combined "{{cmd}}"
  else
      complete -o nospace -o bashdefault -F _complete_cargo_pg_combined "{{cmd}}"
  fi
fi
