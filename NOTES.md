# How to _install_ autocompletions

Spoiler: in general this is one of the most cursed topics in all
of modern shell programming, with the *slight* exception of fish
and it's still a huge fucking pain there with like 10 different
places you might want to install them.

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
      it originates from the AI slop era, and it's complex enough that I don't
      want to audit it.
- ZSH: stack overflow answer (https://stackoverflow.com/a/67161186/1958900)
- autocompletion scripts for "starship" on my machine were installed automatically along w/ homebrew
