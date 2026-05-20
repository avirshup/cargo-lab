# Note: this needs to be sourced, it won't work as a script
# because it doesn't have the same completion environment

# same as in "src/cli/autocomplete_cargo_shim.bash"
_get_completion_fn() {
  local cmd="$1"
  local compfn
  compfn=$(complete -p "$cmd" 2>/dev/null) ||
    return 0

  [[ "$compfn" =~ [[:space:]]-F[[:space:]]*([^[:space:]]+) ]] &&
    test -n "${BASH_REMATCH[1]}" &&
    echo "${BASH_REMATCH[1]}"
}

# Does this work? It seems to work correctly for cargo
# and git, but for `cargo{ |-}playground` it always
# returns ALL of the completions (e.g., it suggests
# `cargo playground inject` for "cargo playground ne").
# HOWEVER the actual tab-completion does not have this issue,
# so maybe there is a filtering step somewhere in bash that
# we're not including here?
show-completions() {
  local compfn
  if ! compfn=$(_get_completion_fn "$1"); then
    echo "error in finding completion provider" >&2
    return 1
  fi

  if test -z "$compfn"; then
    echo "no completion provider found" >&2
    return 1
  fi

  if test -z "$compfn"; then
    echo "No completion function found for command '$1'" >&2
    return 1
  fi

  COMP_LINE="$*"
  COMP_WORDS=("$@")
  COMP_CWORD=$(("${#COMP_WORDS[@]}" - 1))
  COMP_POINT=${#COMP_LINE}

  # Docs: COMP_KEY is "key(s) used to invoke the current completion function"
  # That means its ascii keycode (9 for a tab) I guess
  COMP_KEY=9

  "$compfn" || (
    echo "completion function '$compfn' failed"
    return 1
  )

  for x in "${COMPREPLY[@]}"; do
    echo "$x"
  done
}

# dead code, but call this from a completion function
# to figure out wtf is going on if necessary
# from https://superuser.com/a/1576510 (w/ modifications)
function _debug_comp_vars {
  echo "COMP_CWORD:" "$COMP_CWORD"
  echo "COMP_POINT:" "$COMP_POINT"
  echo "COMP_LINE:" "$COMP_LINE"
  echo "COMP_KEY:" "$COMP_KEY"
  echo "COMP_WORDS:" "${COMP_WORDS[@]}"
  echo "COMPREPLY:" "${COMPREPLY[@]}"
  echo "LEN(COMP_WORDS):" "${#COMPREPLY[@]}"
  echo
}
