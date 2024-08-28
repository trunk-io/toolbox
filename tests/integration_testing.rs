use assert_cmd::prelude::*;

use std::fmt;
use std::fs;
use std::process::Command;

pub struct TestRepo {
    dir: tempfile::TempDir,
}

#[derive(Debug)]
pub struct HortonOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

impl fmt::Display for HortonOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.exit_code {
            Some(c) => write!(f, "toolbox exit code was {}\n", c)?,
            None => write!(f, "toolbox exited abnormally\n")?,
        };

        if self.stdout.is_empty() {
            write!(f, "toolbox stdout: (empty)\n")?;
        } else {
            write!(f, "toolbox stdout:\n{}\n", self.stdout.as_str())?;
        }

        if self.stderr.is_empty() {
            write!(f, "toolbox stderr: (empty)\n")
        } else {
            write!(f, "toolbox stderr:\n{}\n", self.stderr.as_str())
        }
    }
}

impl TestRepo {
    pub fn make() -> anyhow::Result<TestRepo> {
        // TODO: tempdir is a poor choice:
        //
        //   * created directories do not clearly map to test cases, so
        //     debugging is hard
        //   * capturing the tempdir at the end of the test requires
        //     hooking into the TestRepo dtor to cp -r its contents out
        //
        // The only thing it does well is clean up after itself.
        let dir = tempfile::tempdir()?;

        Command::new("git")
            .arg("init")
            .arg("--initial-branch")
            .arg("main")
            .current_dir(dir.path())
            .output()?;

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("horton integration test")
            .current_dir(dir.path())
            .output()?;

        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("horton@whoville.trunk.io")
            .current_dir(dir.path())
            .output()?;

        Command::new("git")
            .arg("commit")
            .arg("--message")
            .arg("Initial commit")
            .arg("--allow-empty")
            .current_dir(dir.path())
            .output()?;

        Ok(TestRepo { dir })
    }

    pub fn write(&self, relpath: &str, data: &[u8]) {
        let path = {
            let mut path = self.dir.path().to_path_buf();
            path.push(relpath);
            path
        };

        // Create the directory hierarchy if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect(format!("Unable to create directories for {:#?}", parent).as_str());
        }

        fs::write(&path, data).expect(format!("Unable to write {:#?}", path).as_str());
    }

    pub fn git_add_all(&self) -> anyhow::Result<()> {
        Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(self.dir.path())
            .output()?;

        Ok(())
    }

    pub fn git_commit_all(&self, message: &str) {
        self.git_add_all().expect("add worked");

        let output = Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(message)
            .current_dir(self.dir.path())
            .output()
            .expect("Failed to execute git command");

        assert!(output.status.success(), "Git commit failed");
    }

    pub fn run_horton(&self) -> anyhow::Result<HortonOutput> {
        self.run_horton_with("HEAD", "sarif")
    }

    pub fn set_toolbox_toml(&self, config: &str) {
        fs::write(
            self.dir.path().join("toolbox.toml"),
            config.as_bytes(),
        )
        .expect("Failed to write toolbox.toml");
    }

    pub fn run_horton_with(
        &self,
        upstream_ref: &str,
        format: &str,
    ) -> anyhow::Result<HortonOutput> {
        let mut cmd = Command::cargo_bin("trunk-toolbox")?;

        let modified_paths =
            horton::git::modified_since(upstream_ref, Some(self.dir.path()))?.paths;
        let files: Vec<String> = modified_paths
            .keys()
            .map(|key| key.to_string())
            .collect();

        cmd.env("RUST_LOG", "debug");
        cmd.arg("--upstream")
            .arg(upstream_ref)
            .current_dir(self.dir.path());
        cmd.arg("--output-format").arg(format);
        for path in files {
            cmd.arg(path);
        }

        log::debug!("Command: {}", format!("{:?}", cmd));

        let output = cmd.output()?;

        return Ok(HortonOutput {
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
            exit_code: output.status.code(),
        });
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        log::info!(
            "TestRepo will clean up after itself: {:#?}",
            self.dir.path()
        );
    }
}
