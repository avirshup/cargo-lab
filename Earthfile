VERSION 0.8

FROM rust:1.94.1-slim

# pre-built rust commands
# https://github.com/earthly/lib/tree/main/rust
IMPORT github.com/earthly/lib/rust:2.2.11 AS rust

all:
    BUILD +check
    BUILD +test
    BUILD +test-e2e

# ───── Base environment setup ─────────────────────────────────── #
env:
    # project dir
    WORKDIR /src/project

    # bin tool setup
    RUN rustup component add \
        clippy \
        rustfmt
    RUN rustup component add \
        --toolchain=nightly rustfmt
    RUN apt-get update --quiet=2 \
     && apt-get install --no-install-recommends --quiet --yes \
        autoconf \
        autotools-dev \
        bsdmainutils \
        clang \
        cmake \
        git \
        libtool-bin \
     && rm -rf /var/lib/apt/lists/*

    # cargo setup
    DO rust+INIT --keep_fingerprints=true
    ENV CARGO_TERM_COLOR=always
    ENV RUSTFLAGS="-Dwarnings"
    DO rust+CARGO --args="install --locked \
        cargo-deny \
        cargo-edit"

src-tree-prep:
    FROM +env
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

    # Prep dir for export
    # There's no way (AFAICT) to use .earthlyignore here, so it's easiest
    # to just remove the files we don't wannt
    RUN rm -rf .git \
     && rm -r dev_notes Earthfile .earthlyignore
    RUN echo "$CRATE_VERSION" > VERSION  # gotta pass information somehow
    SAVE ARTIFACT --keep-ts ./ project

    # WARNING: The following commands have different copy semantics!
    #   "SAVE ARTIFACT ." -> "COPY +target/ dest"
    #   will always copy to *a new folder* called "dest/project" (assuming
    #   the source directory was named "project".)
    #
    #   "SAVE ARTIFACT . somename" -> "COPY +target/somename dest"
    #   will *merge* the contents of the source directory _into_ dest.
    #
    #   (Noting this here because this has confused me for *years* and
    #   I just figured out what's going on)

src:
    FROM +env
    # Image layer w/ prepped source tree

    # For caching purposes we _copy_ this from +src-tree-prep
    # so changes to irrelevant files (such as everything in .git) won't
    # invalidate image layers.
    COPY --keep-ts +src-tree-prep/project ./
    ENV CRATE_VERSION=$(cat VERSION)

# ───── Testing ────────────────────────────────────────────────── #
check:
    FROM +src

    # rust lints
    DO rust+CARGO --args="check"
    DO rust+CARGO --args="+nightly fmt --check -v"
    DO rust+CARGO --args="clippy -v"
    DO rust+CARGO --args="deny -L info check"

    # worth checking for as I keep screwing it up
    COPY Earthfile .
    IF grep -E '^ +RUN cargo' Earthfile
        RUN echo 'In Earthfile: use "DO rust+CARGO", not "RUN cargo"';\
         exit 1
    END

test:
    FROM +src
    DO rust+CARGO --args="test"

test-e2e:
    FROM +src
    DO rust+CARGO --args="test -- --ignored"

# TODO: `cargo install` it then run the e2e tests in situ

# ───── Outputs ────────────────────────────────────────────────── #
build:
    FROM +src
    ARG MODE="release"
    ARG BIN_NAME="cargo-playground"

    DO rust+CARGO \
        --args="build --${MODE}" \
        --output="$MODE/$BIN_NAME"
    RUN test -e target/$MODE/$BIN_NAME

    LET BIN_TARGET="$(rustc --print host-tuple)/${BIN_NAME}-${CRATE_VERSION}-${MODE}"
    SAVE ARTIFACT \
        target/$MODE/$BIN_NAME \
        AS LOCAL \
        "./artifacts/${BIN_TARGET}"


publish:
    FROM +src
    ARG build_tag=""
    ARG live="--dry-run"

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

    WAIT
        BUILD +check
        BUILD +test
    END
    RUN --push \
        --secret CARGO_REGISTRY_TOKEN=apitoken \
        cargo publish $live --allow-dirty
