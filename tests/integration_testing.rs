use assert_cmd::prelude::*;

use serde_sarif::sarif::{Run, Sarif};
use std::fmt;
use std::fs;
use std::process::Command;
use tempfile::NamedTempFile;

pub struct TestRepo {
    dir: tempfile::TempDir,
}

#[derive(Debug)]
pub struct HortonOutput {
    pub stdout: String,
    pub stderr: String,
    pub results: String, // results get written to tmp file and then read back in
    pub exit_code: Option<i32>,
}

impl HortonOutput {
    #[allow(dead_code)]
    pub fn runs(&self) -> Vec<Run> {
        let sarif: Sarif = match serde_json::from_str(&self.results) {
            Ok(s) => s,
            Err(e) => panic!("Failed to parse stdout as SARIF: {}", e), // Panic if parsing fails
        };

        sarif.runs
    }

    #[allow(dead_code)]
    pub fn has_result(&self, rule_id: &str, message: &str, file: Option<&str>) -> bool {
        // Iterate over the runs and results to find the matching code and message
        for run in self.runs() {
            if let Some(results) = run.results {
                for result in results {
                    if result.rule_id.as_deref() == Some(rule_id) {
                        if let Some(text) = result.message.text.as_deref() {
                            if text.contains(message) {
                                if file.is_some() {
                                    if let Some(locations) = result.locations {
                                        for location in locations {
                                            if let Some(ph) = location.physical_location {
                                                if let Some(fp) = ph.artifact_location {
                                                    if let Some(f) = fp.uri {
                                                        if f.contains(file.unwrap()) {
                                                            return true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    #[allow(dead_code)]
    pub fn has_fix_with_content(&self, rule_id: &str, expected_content: &str) -> bool {
        for run in self.runs() {
            if let Some(results) = run.results {
                for result in results {
                    if result.rule_id.as_deref() == Some(rule_id) {
                        if let Some(fixes) = &result.fixes {
                            for fix in fixes {
                                for change in &fix.artifact_changes {
                                    for replacement in &change.replacements {
                                        if let Some(content) = &replacement.inserted_content {
                                            if let Some(text) = &content.text {
                                                if text.contains(expected_content) {
                                                    return true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn has_result_with_rule_id(&self, rule_id: &str) -> bool {
        // Iterate over the runs and results to find the matching code and message
        for run in self.runs() {
            if let Some(results) = run.results {
                for result in results {
                    if result.rule_id.as_deref() == Some(rule_id) {
                        return true;
                    }
                }
            }
        }

        false
    }
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

        // Ambient user config may require commit signing; opt the hermetic
        // test repo out so tests don't depend on signing infrastructure.
        Command::new("git")
            .arg("config")
            .arg("commit.gpgsign")
            .arg("false")
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
            fs::create_dir_all(parent)
                .expect(format!("Unable to create directories for {:#?}", parent).as_str());
        }

        fs::write(&path, data).expect(format!("Unable to write {:#?}", path).as_str());
    }

    #[allow(dead_code)]
    pub fn delete(&self, relpath: &str) {
        let path = {
            let mut path = self.dir.path().to_path_buf();
            path.push(relpath);
            path
        };
        fs::remove_file(&path).expect(format!("Unable to delete {:#?}", path).as_str());
    }

    pub fn git_add_all(&self) -> anyhow::Result<()> {
        Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir(self.dir.path())
            .output()?;

        Ok(())
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn run_horton(&self) -> anyhow::Result<HortonOutput> {
        self.run_horton_with("HEAD", "sarif", true)
    }

    #[allow(dead_code)]
    pub fn set_toolbox_toml(&self, config: &str) {
        self.write(".config/toolbox.toml", config.as_bytes());
    }

    pub fn run_horton_with(
        &self,
        upstream_ref: &str,
        format: &str,
        write_results_to_file: bool,
    ) -> anyhow::Result<HortonOutput> {
        self.run_horton_inner(
            upstream_ref,
            format,
            ResultsMode::from_flag(write_results_to_file),
            None,
        )
    }

    #[allow(dead_code)]
    pub fn run_horton_customized(
        &self,
        upstream_ref: &str,
        format: &str,
        results_path: Option<&std::path::Path>,
        cache_dir: Option<&str>,
    ) -> anyhow::Result<HortonOutput> {
        let results_mode = match results_path {
            Some(p) => ResultsMode::Explicit(p.to_path_buf()),
            None => ResultsMode::None,
        };
        self.run_horton_inner(upstream_ref, format, results_mode, cache_dir)
    }

    fn run_horton_inner(
        &self,
        upstream_ref: &str,
        format: &str,
        results_mode: ResultsMode,
        cache_dir: Option<&str>,
    ) -> anyhow::Result<HortonOutput> {
        let mut cmd = Command::cargo_bin("trunk-toolbox")?;

        let modified_paths =
            horton::git::modified_since(upstream_ref, Some(self.dir.path()))?.paths;
        let files: Vec<String> = modified_paths
            .keys()
            .map(|key| key.to_string_lossy().to_string())
            .collect();

        cmd.arg("--upstream")
            .arg(upstream_ref)
            .current_dir(self.dir.path());
        cmd.arg("--output-format").arg(format);
        if let Some(dir) = cache_dir {
            cmd.arg("--cache-dir").arg(dir);
        }
        for path in files {
            cmd.arg(path);
        }

        // Hold the tempfile alive for the duration of the run when we created one.
        let mut _tmpfile_guard: Option<NamedTempFile> = None;
        let results_path: Option<String> = match &results_mode {
            ResultsMode::None => None,
            ResultsMode::Auto => {
                let tmpfile = NamedTempFile::new()?;
                let path = tmpfile.path().to_str().unwrap().to_string();
                cmd.arg("--results").arg(&path);
                _tmpfile_guard = Some(tmpfile);
                Some(path)
            }
            ResultsMode::Explicit(p) => {
                let path = p.to_str().unwrap().to_string();
                cmd.arg("--results").arg(&path);
                Some(path)
            }
        };

        log::debug!("Command: {}", format!("{:?}", cmd));

        let output = cmd.output()?;

        let results = match &results_path {
            Some(p) => fs::read_to_string(p).unwrap_or_default(),
            None => String::new(),
        };

        return Ok(HortonOutput {
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
            results,
            exit_code: output.status.code(),
        });
    }
}

enum ResultsMode {
    None,
    Auto,
    Explicit(std::path::PathBuf),
}

impl ResultsMode {
    fn from_flag(write_results_to_file: bool) -> Self {
        if write_results_to_file {
            ResultsMode::Auto
        } else {
            ResultsMode::None
        }
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
