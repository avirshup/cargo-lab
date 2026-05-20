#!/usr/bin/env bash

set -eou pipefail

# really basic tests to ensure autcomplete is still working
# for both cargo _and_ -playground
function test_autocomplete_works() {
  assert_completion "cargo met" "metadata"
  assert_completion "cargo play" "playground"

  assert_not_completion "cargo inj" "inject"

  for cmd in "cargo playground" "cargo-playground"; do
    assert_completion "$cmd inj" "inject"

    # # ensure not returning wrong completions
    # assert_not_completion "$cmd ne" "inject"
    # # NOTE: disabled for now, this test fails for bash
    # # because (I think) the script that clap generates
    # # calls the bash built-in "compopt", which doesn't
    # # like our mocked completion environment

    # ensure not (somehow) returning cargo's completions
    assert_not_completion "$cmd met" "metadata"
  done
}

# ───── Assertion helpers ──────────────────────────────────────── #
function assert_completion() {
  local line=$1
  local expected=$2

  bash_result=$(bash_completions "$line")
  _assert_result_contains "$bash_result" "$expected" BASH

  fish_result=$(fish_completions "$line")
  _assert_result_contains "$fish_result" "$expected" FISH

  # # zsh, i am defeated, i do not wish to continue
  #  zsh_result=$(zsh_completions "$line")
  #  _assert_result_contains "$zsh_result" "$expected" ZSH
}

function assert_not_completion() {
  local line=$1
  local expected=$2

  bash_result=$(bash_completions "$line")
  _assert_result_does_not_contain "$bash_result" "$expected" BASH

  fish_result=$(fish_completions "$line")
  _assert_result_does_not_contain "$fish_result" "$expected" FISH

  # # holy hell, zsh, somehow 10x more complicated than bash
  #  zsh_result=$(zsh_completions "$line")
  #  _assert_result_does_not_contain "$zsh_result" "$expected" ZSH
}

_assert_result_contains() {
  local actual=$1
  local expected=$2
  local shellname=$3

  if ! (echo "$actual" | grep --fixed-strings --silent "$expected" &>/dev/null); then
    echo "Failed: $shellname completions for '$line' did NOT contain '$expected'"
    echo "Returned completions: '$actual'"
    return 1
  fi
}

_assert_result_does_not_contain() {
  local actual=$1
  local expected=$2
  local shellname=$3

  if (echo "$actual" | grep --fixed-strings --silent "$expected" &>/dev/null); then
    echo "Failed: $shellname completions for '$line' DID contain '$expected', but should not have"
    echo: "Returned completions: '$actual'"
    return 1
  fi
}

# ───── Runners ────────────────────────────────────────────────── #
# these all run commands _in subshells_ so that all of the environment is set up correctly.
function bash_completions() {
  # !! Requires that the "show-completions" function be defined in the shell's startup scripts !!
  bash -ic "show-completions $1"
}

function fish_completions() {
  fish -c "complete --do-complete '$1'"
}

# # GAVE UP: somehow even more complex and convoluted than bash?
#function zsh_completions() {
#  zsh -ic "show-completions $1"
#}

# ───── if __name__ == '__main__' ──────────────────────────────── #
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  test_autocomplete_works
fi
