use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub static SAMPLE1: &str = include_str!("../../data/1004916019.dat");
pub static SAMPLE2: &str = include_str!("../../data/119232022.dat");
pub static INVALID: &str = include_str!("../../data/invalid.dat");

pub type MatchResult = Result<(), String>;

#[derive(Debug)]
pub struct CommandBuilder<'a> {
    command: String,
    pica_bin: PathBuf,
    root_dir: &'a Path,
    args: Vec<String>,

    expect_exit_code: Option<i32>,
    expect_stdout: Option<String>,
    expect_stdout_one_of: Option<Vec<String>>,
    expect_stderr: Option<String>,
}

impl<'a> CommandBuilder<'a> {
    pub fn new<S: ToString>(command: S) -> Self {
        let root_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

        CommandBuilder {
            command: command.to_string(),
            pica_bin: root_dir.join("target/debug/pica"),
            root_dir,
            args: Vec::new(),
            expect_exit_code: Some(0),
            expect_stdout: None,
            expect_stdout_one_of: None,
            expect_stderr: None,
        }
    }

    pub fn arg<S: ToString>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.to_string());
        self
    }

    pub fn args<S: ToString>(&mut self, arg: S) -> &mut Self {
        let arg = arg.to_string();
        let args: Vec<String> = arg.split(' ').map(|x| x.to_string()).collect();

        self.args.extend(args);
        self
    }

    pub fn with_status(&mut self, status: i32) -> &mut Self {
        self.expect_exit_code = Some(status);
        self
    }

    pub fn with_stdout(&mut self, expected: &str) -> &mut Self {
        match self.expect_stdout {
            None => self.expect_stdout = Some(expected.to_string()),
            Some(ref mut stdout) => stdout.push_str(expected),
        }

        self
    }

    pub fn with_stdout_one_of(&mut self, expected: Vec<&str>) -> &mut Self {
        self.expect_stdout_one_of =
            Some(expected.iter().map(|x| x.to_string()).collect());
        self
    }

    pub fn with_stdout_empty(&mut self) -> &mut Self {
        self.expect_stdout = Some("".to_string());
        self
    }

    pub fn with_stderr(&mut self, expected: &str) -> &mut Self {
        match self.expect_stderr {
            None => self.expect_stderr = Some(expected.to_string()),
            Some(ref mut stderr) => stderr.push_str(expected),
        }

        self
    }

    fn match_status(&self, output: &Output) -> MatchResult {
        match self.expect_exit_code {
            None => Ok(()),
            Some(expected) if output.status.code() == Some(expected) => Ok(()),
            Some(expected) => Err(format!(
                "exited with '{:?}', expected '{}'",
                output.status.code(),
                expected
            )),
        }
    }

    fn match_stdout(&self, output: &Output) -> MatchResult {
        let actual = String::from_utf8(output.stdout.clone()).unwrap();

        if let Some(expected) = &self.expect_stdout {
            if actual != *expected {
                return Err(format!(
                    "expected stdout '{}', got '{}'",
                    expected, actual
                ));
            }
        }

        if let Some(expected) = &self.expect_stdout_one_of {
            if !expected.iter().any(|x| *x == actual) {
                return Err(format!(
                    "expected stdout one of '{:?}', got '{}'",
                    expected, actual
                ));
            }
        }

        Ok(())
    }

    fn match_stderr(&self, output: &Output) -> MatchResult {
        if let Some(expected) = &self.expect_stderr {
            let actual = String::from_utf8(output.stderr.clone()).unwrap();

            if actual != *expected {
                return Err(format!(
                    "expected stderr '{}', got '{}'",
                    expected, actual
                ));
            }
        }

        Ok(())
    }

    fn match_output(&self, output: &Output) -> MatchResult {
        self.match_status(output)
            .and(self.match_stdout(output))
            .and(self.match_stderr(output))
    }

    pub fn output(&self) -> std::io::Result<Output> {
        Command::new(&self.pica_bin)
            .current_dir(self.root_dir)
            .arg(&self.command)
            .args(&self.args)
            .output()
    }

    pub fn run(&mut self) -> MatchResult {
        match self.output() {
            Ok(output) => self.match_output(&output),
            Err(_) => Err("could not run command".to_string()),
        }
    }
}
