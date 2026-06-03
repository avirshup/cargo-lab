#!/usr/bin/env bash
# Directly test if autocompletion is set up correctly in various shells.
# Somewhat wonky. Currently DOES NOT TEST ZSH and BASH TESTS ARE HACKY.
# (Can also be tested manually by building the autocomplete environment
# container with `earth +autocomplete-env` then interactively running
# it)
#
# (I tried doing this "the right way" - testing autocomplete _in a PTY_ -
# using `completest-pty` but for reasons unknown could not get
# it to work in the CI container environment ...)
#
# WONKINESS WARNING: Testing tab-completion outside of a PTY
# isn't totally possible in bash or zsh (it works great in fish though!).
# Specifically:
# - for ZSH: zsh has multiple autocomplete systems with a lot of complexity.
#   There is no equivalent to `complete --do-complete`.
#   Without a PTY, there is no sane path to testing here.
#
# - for BASH: the script that clap's `CompleteEnv` emits uses `compopt`,
#   which is not working correctly as called here - it spits out a bunch
#   of warnings that I think indicate it needs to be running in a PTY,
#   (including "not currently executing completion function" and
#   "no job control in this shell")
set -eou pipefail

# really basic tests to ensure autocomplete is still working
# for both cargo _and_ -lab
function test_autocomplete_works() {
  assert_completion "cargo met" "metadata"
  assert_completion "cargo la" "lab"

  assert_not_completion "cargo inj" "inject"

  for cmd in "cargo lab" "cargo-lab"; do
    assert_completion "$cmd inj" "inject"

    # # ensure not returning wrong completions
    # # !DISABLED! for now, see bash wonkiness warning at top
    # assert_not_completion "$cmd ne" "inject"

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
