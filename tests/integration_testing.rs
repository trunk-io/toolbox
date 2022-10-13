use assert_cmd::prelude::*; // Add methods on commands

use std::error::Error;
use std::fs;
use std::process::Command;
use std::result::Result;

pub struct TestRepo {
    dir: tempfile::TempDir,
}

impl TestRepo {
    pub fn make() -> anyhow::Result<TestRepo> {
        let dir = tempfile::tempdir()?;

        Command::new("git")
            .arg("init")
            .arg("--initial-branch")
            .arg("main")
            .current_dir(dir.path())
            .spawn()?
            .wait()?;

        Command::new("git")
            .arg("config")
            .arg("user.name")
            .arg("horton integration test")
            .current_dir(dir.path())
            .spawn()?
            .wait()?;

        Command::new("git")
            .arg("config")
            .arg("user.email")
            .arg("horton@whoville.trunk.io")
            .current_dir(dir.path())
            .spawn()?
            .wait()?;

        Command::new("git")
            .arg("commit")
            .arg("--message")
            .arg("Initial commit")
            .arg("--allow-empty")
            .current_dir(dir.path())
            .spawn()?
            .wait()?;

        Ok(TestRepo { dir })
    }

    pub fn write(&self, relpath: &str, data: &str) -> anyhow::Result<()> {
        let path = {
            let mut path = self.dir.path().to_path_buf();
            path.push(relpath);
            path
        };
        Ok(fs::write(&path, data).expect(format!("Unable to write {:#?}", path).as_str()))
    }

    pub fn git_add_all(&self) -> anyhow::Result<()> {
        Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(self.dir.path())
            .spawn()?
            .wait()?;

        Ok(())
    }

    pub fn run_horton(&self) -> Result<String, Box<dyn Error>> {
        let mut cmd = Command::cargo_bin("trunk-toolbox")?;

        cmd.env("RUST_LOG", "debug");
        cmd.arg("--upstream")
            .arg("HEAD")
            .current_dir(self.dir.path());

        let output = cmd.output()?;

        return Ok(String::from_utf8(output.stdout)?);
    }
}
