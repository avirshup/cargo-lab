use camino::Utf8Path;
use color_print::{cformat, cprintln};

use super::new_script;
use crate::global_ctx::{GlobalCtx, ProjectPaths};
use crate::templates::TEMPLATES;
use crate::{data, global_ctx, util};

/// Initializes a new lab, very very roughly
/// matching how `cargo init` works.
///
/// We don't actually call `cargo init` for now, just
/// write out the initial manifest ourselves.
///
/// ## Notes: `cargo init`'s implementaiton
///  `cargo init`'s help string says
/// "Create a new cargo package in an existing directory".
///
/// However the "existing" part of that is optional,
/// the path (and even its parents) don't need to exist yet.
/// It will error and do nothing if `Cargo.toml` exists in
/// the destination. If other entries (e.g., `src`) exist
/// there, but not the manifest, then it will error
/// out in a partially completed state.
///
/// (We don't _have_ to mirror this behavior bug-for-bug here ...
/// but actually probably will end up doing so.)
pub fn init_new_lab(
    input_path: &Utf8Path,
    name: &str,
    edition: &str,
    ctx: GlobalCtx,
) -> crate::Result<()> {
    // TODO: handle (probably w/ just a warning?)
    //   if `--manifest-path` or `CARGO_LAB_MANIFEST_PATH`
    //   were provided (and ignore them otherwise)

    // path checks
    if input_path.exists() {
        if !input_path.is_dir() {
            return Err(crate::Error::FileErr {
                path: input_path.to_owned(),
                description: "provided lab directory exists but is not a \
                              directory."
                    .to_owned(),
            });
        }
        let manifest_path = input_path.join("Cargo.toml");
        if manifest_path.exists() {
            return Err(crate::Error::FileErr {
                path: input_path.to_owned(),
                description: "Already initialized as a cargo project."
                    .to_owned(),
            });
        }
    }

    // MAYBE: This could be made somewhat more atomic by assembling all of
    //   this in a temporary directory (under the same parent, probably)
    //   then moving it into place upon success

    // 1. Create directories
    if ctx.verbosity > global_ctx::Quiet {
        println!("Initializing project directory ...");
    }
    util::create_dir(input_path)?;
    let paths = ProjectPaths::from_manifest_dir(input_path)?;
    util::create_dir(&paths.template_dir)?;
    util::create_dir(&paths.script_dir)?;

    // 2. Create Cargo.toml
    util::write_file(
        &paths.cargo_dot_toml,
        &_init_manifest_content(name, edition),
    )?;

    // 3. Write builtin templates
    for template in TEMPLATES {
        let target = paths.template_path(template.name);
        util::write_file(&target, template.content)?;
    }

    // 4. Add the first script
    if ctx.verbosity > global_ctx::Quiet {
        println!("Creating first script ...");
    }
    new_script::new_script(
        &data::ScriptConfigRequest::nodeps("my-first-experiment".to_owned()),
        None,
        false,
        ctx.with_paths(paths.clone()),
    )?;

    if ctx.verbosity > global_ctx::Quiet {
        cprintln!(
            "\n<green>success</>: Lab initialized in directory '<blue>{}</>'",
            paths
                .manifest_dir
                .strip_prefix(&ctx.cwd)
                .unwrap_or_else(|_| &paths.manifest_dir)
        );

        println!(
            "\n{}",
            _usage_tips(
                paths.manifest_dir.canonicalize_utf8().unwrap().as_str()
            )
        );
    } else {
        println!("{}", paths.manifest_dir.canonicalize_utf8().unwrap());
    }

    Ok(())
}

/// Create a template for the initial manifest.
/// Just done as plain text so we can add comments
fn _init_manifest_content(project_name: &str, edition: &str) -> String {
    format!(
        r#"
[package]
name = "{project_name}"
edition = "{edition}"
publish = false

[package.metadata.cargo-lab]
enabled = true
# editor-cmd = ["vim"]
"#
    )
}

fn _usage_tips(manifest_dir_abspath: &str) -> String {
    let env_var = global_ctx::CARGO_LAB_MANIFEST_DIR;
    cformat!(
        r#"Tips:
 1) To enable tab-completion, run `<cyan>cargo lab completions --help</>`
 2) To easily run experiments from any working directory, set
    `<cyan>{env_var}={manifest_dir_abspath}</cyan>`
    or use the `--manifest-path` flag
    (`<cyan>cargo lab --manifest-path={manifest_dir_abspath}</>`).
 3) To alias this command to something shorter, prefer a shell alias
    (e.g., `alias clb="cargo lab"`) over a cargo alias (in `.cargo/config.toml`),
    as tab-completion does not currently support cargo aliases.
    "#
    )
}
