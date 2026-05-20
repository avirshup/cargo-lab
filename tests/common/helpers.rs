use std::path::Path;

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use color_print::ceprintln;
use tempfile::tempdir;

const BIN_NAME: &str = "cargo-playground";

#[derive(Debug)]
pub struct ScratchDir(tempfile::TempDir);

#[allow(clippy::new_without_default)]
impl ScratchDir {
    pub fn new() -> Self {
        Self(tempdir().expect("can create tempdir"))
    }

    pub fn path(&self) -> &Path {
        self.0.path()
    }
}

/// For debugging, ensure the temp dir does NOT get cleaned up during a panic
impl Drop for ScratchDir {
    fn drop(&mut self) {
        if std::thread::panicking() {
            ceprintln!(
                "Tempdir saved: <bright-blue,underline>{}</>",
                self.0.path().to_string_lossy()
            );
            self.0.disable_cleanup(true);
        }
    }
}

#[derive(Debug)]
pub struct Runner<'dir> {
    working_dir: Utf8PathBuf,
    tempdir: &'dir ScratchDir,
    env: Vec<(String, String)>,
}

impl<'dir> Runner<'dir> {
    pub fn new(tempdir: &'dir ScratchDir) -> Self {
        Self {
            working_dir: Utf8Path::from_path(tempdir.path())
                .expect("temp dir is UTF-8")
                .to_owned(),
            tempdir,
            env: Vec::new(),
        }
    }

    pub fn with_env(
        &self,
        vars: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let mut env = self.env.clone();
        env.extend(vars);
        Self {
            working_dir: self.working_dir.clone(),
            tempdir: self.tempdir,
            env,
        }
    }

    /// run an arbitrary command
    #[must_use]
    pub fn run(&self, exe_cmd: &str, args: &[&str]) -> ProcResult {
        self._runit(Command::new(exe_cmd), args)
    }

    /// build and run the cargo playground executable with the
    /// given args
    #[must_use]
    pub fn run_cpg(&self, args: &[&str]) -> ProcResult {
        self._runit(
            Command::cargo_bin(BIN_NAME).expect("found cargo exe"),
            args,
        )
    }

    #[must_use]
    pub fn run_cargo(&self, args: &[&str]) -> ProcResult {
        self._runit(Command::new("cargo"), args)
    }

    fn _runit(&self, mut cmd: Command, args: &[&str]) -> ProcResult {
        let output = cmd
            .current_dir(&self.working_dir)
            .args(args)
            .envs(self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .output()
            .expect("spawning cmd");

        ProcResult {
            output,
            cmd_str: format!("{cmd:?}"),
        }
    }

    pub fn cd(&mut self, path: &str) -> &mut Self {
        self.working_dir = self
            .working_dir
            .join(path)
            .canonicalize()
            .expect("invalid path")
            .try_into()
            .expect("non-UTF-8 path");
        if !self.working_dir.is_dir() {
            panic!("Cannot cd to {}: {}", self.working_dir, "not a directory")
        }
        self
    }

    pub fn expect_file(&self, relpath: &str) -> &Self {
        let path = self.working_dir.join(relpath);

        if !self.working_dir.join(&path).exists() {
            panic!("no such file: {path}");
        }

        if !self.working_dir.join(&path).is_file() {
            panic!("not a regular file: {path}");
        }

        self
    }

    pub fn consistency_checks(&self) {
        self.run_cargo(&["check"]).expect_ok();
        self.run_cpg(&["check"]).expect_ok();
    }
}

pub struct ProcResult {
    pub output: std::process::Output,
    pub cmd_str: String, // human-readable representation of the cli command
}

impl ProcResult {
    #[track_caller]
    pub fn expect_ok(&self) -> &Self {
        let code = self.output.status.code();

        if code != Some(0) {
            eprintln!("Failed! (expected exitcode 0)");
            self.dump_output();
            panic!("Cmd failed: {}", self.cmd_str);
        }
        self
    }

    #[track_caller]
    pub fn expect_fail(&self) -> &Self {
        let code = self.output.status.code();
        if code == Some(0) || code.is_none() {
            eprintln!("Expected non-zero exitcode");
            self.dump_output();
            panic!("Cmd succeeded but should have failed: {}", self.cmd_str);
        }
        self
    }
    //
    // #[track_caller]
    // pub fn expect_stdout_nonempty(&self) -> &Self {
    //     if self.stdout_str().trim().is_empty() {
    //         self.dump_output();
    //         panic!(
    //             "Command: `{}`\nwas expected to produce stdout, but did not. ",
    //             self.cmd_str
    //         )
    //     }
    //     self
    // }

    #[track_caller]
    pub fn expect_stdout(&self, expected: &str) -> &Self {
        if self.stdout_str().trim() != expected {
            self.dump_output();
            panic!(
                "Command: `{}`\ndid not produce expected stdout: \
                 \"{expected}\"",
                self.cmd_str
            )
        }
        self
    }

    #[track_caller]
    pub fn expect_stdout_contains(&self, substr: &str) -> &Self {
        if !self.stdout_str().contains(substr) {
            self.dump_output();
            panic!(
                "Command: `{}`\nSTDOUT did not contain: \"{substr}\"",
                self.cmd_str
            )
        }
        self
    }

    pub fn stdout_str(&self) -> &str {
        str::from_utf8(&self.output.stdout).expect("STDOUT must be UTF-8")
    }

    pub fn dump_output(&self) {
        eprintln!("CMD: {}", self.cmd_str);
        eprintln!(
            "\nstdout:\n{}\n",
            str::from_utf8(&self.output.stdout).unwrap_or("<non-utf-8 output>"),
        );
        eprintln!(
            "stderr:\n\n{}",
            str::from_utf8(&self.output.stderr).unwrap_or("<non-utf-8 output>")
        );
        eprintln!("\nExitcode: {:?}\n", self.output.status.code());
    }
}

// ───── utils ─────
pub fn projpath(dir: &str, name: &str, ext: &str) -> String {
    format!("{}/{}.{}", dir, name.replace('-', "_"), ext)
}
