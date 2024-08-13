use std::process::{Command, ExitStatus, Stdio};

use camino::Utf8PathBuf;

#[derive(Debug)]
pub struct CmdOutput {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl CmdOutput {
    pub fn status(&self) -> &ExitStatus {
        &self.status
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }
}

pub struct Cmd {
    name: String,
    args: Vec<String>,
    current_dir: Option<Utf8PathBuf>,
    hide_stdout: bool,
}

impl Cmd {
    pub fn new<I, S>(cmd_name: &str, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string())
            .collect();
        Self {
            name: cmd_name.to_string(),
            args,
            current_dir: None,
            hide_stdout: false,
        }
    }

    pub fn with_current_dir(&mut self, dir: impl Into<Utf8PathBuf>) -> &mut Self {
        self.current_dir = Some(dir.into());
        self
    }

    pub fn hide_stdout(&mut self) -> &mut Self {
        self.hide_stdout = true;
        self
    }

    pub fn run(&self) -> CmdOutput {
        println!("🚀 {} {}", self.name, self.args.join(" "));
        let mut command = Command::new(&self.name);
        if let Some(dir) = &self.current_dir {
            command.current_dir(dir);
        }
        let child = command
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = child.wait_with_output().unwrap();

        let output_stdout = String::from_utf8(output.stdout).unwrap();
        let output_stderr = String::from_utf8(output.stderr).unwrap();
        if !self.hide_stdout {
            println!("{}", output_stdout);
        }
        eprintln!("{}", output_stderr);

        assert!(output.status.success());

        CmdOutput {
            status: output.status,
            stdout: output_stdout,
            stderr: output_stderr,
        }
    }
}
