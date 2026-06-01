VERSION 0.8

FROM rust:1.96.0-slim

# pre-built rust commands
# https://github.com/earthly/lib/tree/main/rust
IMPORT github.com/earthly/lib/rust:2.2.11 AS rust

all:
    WAIT
        BUILD +tests
    END
    BUILD +crate
    BUILD +exe


# ──────────────────────────────────────────────────────────────────────── #
# ───── Environments                                                 ───── #
# ──────────────────────────────────────────────────────────────────────── #

# ───── Base environment setup ─────────────────────────────────── #
build-env:
    # project dir
    WORKDIR /src/project

    # bin tool setup
    RUN rustup component add \
        clippy \
        rustfmt
    RUN rustup component add \
        --toolchain=nightly rustfmt
    RUN --mount type=cache,target=/var/cache/apt/archives \
        --mount type=cache,target=/var/lib/apt/lists \
        apt-get update --quiet=2 \
     && apt-get install --no-install-recommends --quiet --yes \
        curl \
        git
    DO rust+INIT --keep_fingerprints=true
    ENV CARGO_TERM_COLOR=always


crate:
    FROM +build-env
    COPY +build-cargo-set-version/cargo-set-version $CARGO_HOME/bin/

    ARG build_tag=""
    # Checks out specific tag if requested,
    # sets version in Cargo.toml, then exports the minimal tree
    # needed to build and/or publish the crate.

    # get the requested code version and update version numbers
    IF test -z "$build_tag"
      COPY --dir --keep-ts . /src/project
    ELSE
      COPY --dir --keep-ts .git /src/project/.git
      RUN git reset --hard "$build_tag"
    END
    ENV CRATE_VERSION=$(git describe --dirty --tags)

    DO rust+CARGO --args="set-version \"$CRATE_VERSION\""
    DO rust+CARGO --args="package --allow-dirty" \
        --output="package/.*"

    RUN echo "$CRATE_VERSION" > VERSION

    # artifacts for other steps
    SAVE ARTIFACT "VERSION"
    SAVE ARTIFACT "./target/package/cargo-playground-${CRATE_VERSION}" crate-src
    SAVE ARTIFACT --keep-ts \
        "./target/package/cargo-playground-${CRATE_VERSION}.crate" \
        "cargo-playground.crate"

    # output the crate (when this target is built directly)
    SAVE ARTIFACT "./target/package/cargo-playground-${CRATE_VERSION}.crate" \
        AS LOCAL "./artifacts/package/"


SETUP_CRATE_TREE:
    FUNCTION
    # Copies the cleaned source tree into the current directory.
    # (This makes it easy to copy it in at an arbitrary layer
    # for caching purposes)

    COPY +crate/VERSION /tmp/VERSION
    ENV CRATE_VERSION=$(cat /tmp/VERSION)
    COPY --keep-ts +crate/crate-src ./

src:
    FROM +build-env
    DO +SETUP_CRATE_TREE


# ───── Test environments ──────────────────────────────────────── #
shell-testing-env:
    # Earthly's `rust+INIT` function somehow causes cargo's autocomplete
    # to emit things that look like escaped renderings
    # of ANSI control codes into the autocomplete results.
    # (e.g., `^[[1m^[[96m`)
    # So don't use any cargo commands that need the cache here ...
    FROM +base

    RUN --mount type=cache,target=/var/cache/apt/archives \
        --mount type=cache,target=/var/lib/apt/lists \
        apt-get update --quiet=2 \
     && apt-get install --no-install-recommends --quiet --yes \
        bash \
        bash-completion \
        fish \
        zsh

    RUN useradd \
        --uid 1002 \
        --create-home \
        --shell /bin/bash \
        testuser
    USER testuser
    WORKDIR /home/testuser

    # ZSH setup
    RUN echo 'fpath+=~/.zfunc' > .zshrc \
     && echo 'autoload -U compinit; compinit' >> .zshrc \
     && mkdir -p .zfunc \
     && rustup completions zsh cargo > .zfunc/_cargo

    # bash setup
    # (Note it will have already pre-populated .bashrc, including bash-completions)
    COPY tests/shell/show-completions.bash ./helpers/  # helper script to test completions
    RUN test -f .bashrc \
     && echo 'source <(rustup completions bash cargo)' >> .bashrc \
     && echo '. "$HOME/helpers/show-completions.bash"' >> .bashrc

    # fish setup (just run it once to initialize config)
    # no need to install cargo completions, they come w/ fish
    RUN mkdir -p ~/.config/fish/completions

autocomplete-env:
    # An image with cargo playground installed AND autocomplete configured
    # fish, bash, and zsh. Outputs an image directly for manual testing
    # when invoked directly as a build target.
    FROM +shell-testing-env

    COPY +exe/cargo-playground $CARGO_HOME/bin

    USER testuser
    WORKDIR $HOME

    # direct call setup
    RUN echo 'source <(cargo-playground completions bash)' >> .bashrc
    RUN cargo-playground completions zsh >> .zshrc
    RUN cargo-playground completions fish > .config/fish/completions/cargo-playground.fish

    # cargo subcmd setup
    RUN echo 'source <(cargo playground completions bash)' >> .bashrc
    RUN cargo playground completions zsh >> .zshrc
    RUN cargo playground completions fish > .config/fish/completions/cargo.fish

    SAVE IMAGE cpg_autocomplete:$(cargo playground --version | awk '{print $2}')


# ──────────────────────────────────────────────────────────────────────── #
# ───── Tests and checks                                             ───── #
# ──────────────────────────────────────────────────────────────────────── #
tests:
    # tests *all* the things
    WAIT
        BUILD +lints
        BUILD +dep-check
        BUILD +test-unit
    END
    BUILD +lint-readme  # <- not in WAIT block, it shouldn't block the main tests
    BUILD +test-e2e
    BUILD +test-shell-autocomplete

lints:
    FROM +src
    ENV RUSTFLAGS="-Dwarnings"

    COPY rustfmt.toml .

    # rust lints
    DO rust+CARGO --args="check"
    DO rust+CARGO --args="+nightly fmt --check -v"
    DO rust+CARGO --args="clippy -v"

dep-check:
    FROM +build-env
    COPY +build-cargo-deny/cargo-deny $CARGO_HOME/bin/
    DO +SETUP_CRATE_TREE

    DO rust+CARGO --args="deny -L info check"

test-unit:
    FROM +build-env
    ENV RUSTFLAGS="-Dwarnings"

    DO +SETUP_CRATE_TREE
    DO rust+CARGO --args="test"

test-e2e:
    FROM +src
    DO rust+CARGO --args="test -- --ignored"

test-shell-autocomplete:
    # Invokes configured shells to test autocomplete using a script
    # Currently this emits a lot of warnings but still basically
    # works, see "test_shell_completion_integration.sh"
    FROM +autocomplete-env
    COPY tests/shell/test_shell_completion_integration.sh ./
    RUN bash test_shell_completion_integration.sh

lint-readme:
    FROM node:lts-alpine3.22
    RUN npm install --global prettier@3.8.3

    WORKDIR /src
    COPY .prettierrc.yml README.md .
    RUN prettier -c .prettierrc.yml --check README.md

# ──────────────────────────────────────────────────────────────────────── #
# ───── Outputs                                                      ───── #
# ──────────────────────────────────────────────────────────────────────── #
exe:
    FROM +src
    ARG profile="release"
    ARG BIN_NAME="cargo-playground"

    DO rust+CARGO \
        --args="build --profile=${profile}" \
        --output="$profile/$BIN_NAME"
    RUN test -e target/$profile/$BIN_NAME

    LET BIN_TARGET="$(rustc --print host-tuple)/${BIN_NAME}-${CRATE_VERSION}-${profile}"
    SAVE ARTIFACT target/$profile/$BIN_NAME cargo-playground
    SAVE ARTIFACT \
        target/$profile/$BIN_NAME \
        AS LOCAL \
        "./artifacts/${BIN_TARGET}"


publish:
    ARG build_tag
    FROM +build-env
    DO +SETUP_CRATE_TREE
    ARG live="--dry-run"  # dry run by default, pass `--live=""` to do it live

    # ensure we have the version we want
    RUN echo "Computed version: $CRATE_VERSION"
    RUN echo "Git ref requested: $build_tag"
    IF test -z "$build_tag"
      RUN echo "Must specify the tag to publish via --build_tag"; \
        exit 1
    ELSE IF test "$CRATE_VERSION" != "$build_tag"
      RUN echo "Computed version '$CRATE_VERSION' does not match requested build tag '$build_tag'"; \
        exit 1
    ELSE IF echo "$CRATE_VERSION" | grep -q dirty
      RUN echo "Tree is dirty, I won't publish this"; \
        exit 1
    END

    BUILD +all  # entire pipeline must be successful before the push!
    RUN --push \
        --secret CARGO_REGISTRY_TOKEN=apitoken \
        cargo publish $live --allow-dirty


# ───── Helpers and utils ──────────────────────────────────────── #
# `cargo install` builds are not cached by cargo, so every time
# the layer gets invalidated they will get entirely recompiled.
# So build them here in separate layers and copy the artifacts
# when needed
build-cargo-set-version:
    FROM +build-env
    DO rust+CARGO --args="install --locked cargo-edit"
    SAVE ARTIFACT "$CARGO_HOME/bin/cargo-set-version"

build-cargo-deny:
    FROM +build-env
    DO rust+CARGO --args="install --locked cargo-deny"
    SAVE ARTIFACT "$CARGO_HOME/bin/cargo-deny"


