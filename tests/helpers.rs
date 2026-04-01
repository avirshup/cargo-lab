use camino::Utf8PathBuf;

pub struct Runner {
    workdir: Utf8PathBuf,
}

impl Runner {
    pub fn new_tempdir() -> Self {
        todo!()
    }

    pub fn run(&self, args: &[&str]) -> std::process::Output {
        todo!()
    }

    pub fn cd(&mut self, path: &str) {
        self.workdir = self.workdir.join(path);
        if !self.workdir.is_dir() {
            panic!(
                "Cannot cd to {}: {}",
                self.workdir,
                if self.workdir.exists() {
                    "not a directory"
                } else {
                    "does not exist"
                }
            )
        }
    }
}

// ───── Helpers for process::Output ─────
pub trait OutputHelpers {
    fn expect_ok(&self, msg: &'static str) -> &Self;
    fn expect_stdout(&self, expected: &str) -> &Self;

    fn stdout_str(&self) -> &str;
}

impl OutputHelpers for std::process::Output {
    fn expect_ok(&self, msg: &str) -> &Self {
        if self.status.code() != Some(0) {
            eprintln!(
                "Failed!\n\nstdout:\n{}\nstderr:\n\n{}",
                str::from_utf8(&self.stdout).unwrap_or("<non-utf-8 output>"),
                str::from_utf8(&self.stderr).unwrap_or("<non-utf-8 output>")
            );
            panic!("{msg} (exit code {:?})", self.status.code());
        }
        self
    }

    fn expect_stdout(&self, expected: &str) -> &Self {
        assert_eq!(self.stdout_str().trim(), expected);
        self
    }

    fn stdout_str(&self) -> &str {
        str::from_utf8(&self.stdout).expect("STDOUT must be UTF-8")
    }
}
