use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use tempfile::tempdir;

const BIN_NAME: &str = "cargo-playground";

#[derive(Debug)]
pub struct Runner {
    working_dir: Utf8PathBuf,
    tempdir: tempfile::TempDir,
}

impl Runner {
    pub fn new() -> Self {
        let tempdir = tempdir().expect("create tempdir");

        Self {
            working_dir: Utf8Path::from_path(tempdir.path())
                .expect("temp dir is UTF-8")
                .to_owned(),
            tempdir,
        }
    }

    #[must_use]
    pub fn run(&self, args: &[&str]) -> std::process::Output {
        Command::cargo_bin(BIN_NAME)
            .expect("found cargo exe")
            .current_dir(&self.working_dir)
            .args(args)
            .output()
            .expect("spawning cmd")
    }

    pub fn run_cargo(&self, args: &[&str; 1]) -> std::process::Output {
        Command::new("cargo")
            .current_dir(&self.working_dir)
            .args(args)
            .output()
            .expect("spawning cargo cmd")
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
        self.run_cargo(&["check"]).expect_ok("cargo check failed");
        self.run(&["check"])
            .expect_ok("cargo playground check failed");
    }
}

/// For debugging, ensure the temp dir does not get cleaned up during a panic
impl Drop for Runner {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!(
                "Tempdir saved: {}",
                self.tempdir.path().to_string_lossy()
            );
            self.tempdir.disable_cleanup(true);
        }
    }
}

// ───── Helpers for process::Output ─────
pub trait OutputHelpers {
    fn expect_ok(&self, msg: &'static str) -> &Self;
    fn expect_fail(&self, msg: &str) -> &Self;
    fn expect_stdout(&self, expected: &str) -> &Self;
    fn expect_stdout_contains(&self, substr: &str) -> &Self;

    fn stdout_str(&self) -> &str;
    fn dump_output(&self);
}

impl OutputHelpers for std::process::Output {
    fn expect_ok(&self, msg: &str) -> &Self {
        let code = self.status.code();

        if code != Some(0) {
            eprintln!("Failed! (expected exitcode 0)");
            self.dump_output();
            panic!("{msg}");
        }
        self
    }

    fn expect_fail(&self, msg: &str) -> &Self {
        let code = self.status.code();
        if code == Some(0) || code.is_none() {
            eprintln!("Expected non-zero exitcode");
            self.dump_output();
            panic!("{msg}");
        }
        self
    }

    fn expect_stdout(&self, expected: &str) -> &Self {
        assert_eq!(self.stdout_str().trim(), expected);
        self
    }

    fn expect_stdout_contains(&self, substr: &str) -> &Self {
        if !self.stdout_str().contains(substr) {
            self.dump_output();
            panic!("Not found: {substr}")
        }
        self
    }

    fn stdout_str(&self) -> &str {
        str::from_utf8(&self.stdout).expect("STDOUT must be UTF-8")
    }

    fn dump_output(&self) {
        eprintln!(
            "\nstdout:\n{}\n",
            str::from_utf8(&self.stdout).unwrap_or("<non-utf-8 output>"),
        );
        eprintln!(
            "stderr:\n\n{}",
            str::from_utf8(&self.stderr).unwrap_or("<non-utf-8 output>")
        );
        eprintln!("\nExitcode: {:?}\n", self.status.code());
    }
}
