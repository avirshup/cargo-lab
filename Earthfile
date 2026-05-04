VERSION 0.8

FROM rust:1.94.1-slim

# pre-built rust commands
# https://github.com/earthly/lib/tree/main/rust
IMPORT github.com/earthly/lib/rust:2.2.11 AS rust

tools:
    RUN cargo install --locked cargo-deny
    RUN rustup component add \
        clippy \
        rustfmt

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

    DO rust+INIT --keep_fingerprints=true

src:
    FROM +tools
    ARG build_tag=""

    WORKDIR /src/project
    ENV CARGO_TERM_COLOR=always

    # get the requested code version and update version numbers
    IF test -z "$build_tag"
      COPY --dir --keep-ts . /src/project
    ELSE
      COPY --dir --keep-ts .git /src/project/.git
      RUN git reset --hard "$build_tag"
    END
    ENV CRATE_VERSION=$(git describe --dirty --tags)
    RUN sed -r -i 's/version\s*=.+$/version = "'"$CRATE_VERSION"'"/g' Cargo.toml
    RUN git diff --color=always Cargo.toml

    # ensure code is ready for next steps
    DO rust+CARGO --args="check"

check:
    FROM +src
    RUN echo "Checking version $CRATE_VERSION"
    DO rust+CARGO --args="+nightly fmt --check -v"
    DO rust+CARGO --args="clippy -v"
    DO rust+CARGO --args="deny -L info check"
    RUN echo "Checked $CRATE_VERSION"

test:
    FROM +src
    RUN echo "Testing version $CRATE_VERSION"
    DO rust+CARGO --args="test"

all:
    BUILD +check
    BUILD +test


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
