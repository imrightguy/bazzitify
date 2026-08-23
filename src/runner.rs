//! Runner: executes module bash functions with streamed output.

use crate::module::Module;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};

/// Outcome of running a module function.
#[derive(Debug)]
pub struct RunResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: String,
}

/// Run `module_apply` (or `module_undo`) for the given module script.
///
/// `action` is "apply" or "undo". Output (stdout+stderr merged) is captured.
pub fn run_module(modules_dir: &Path, module: &Module, action: &str) -> std::io::Result<RunResult> {
    let script = modules_dir.join(format!("{}.sh", module.name));
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source {script:?}; module_{action} \"$@\"",
            script = script,
            action = action
        ))
        .arg("bazzitify")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let mut output = String::new();
    let out_handle = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            output.push_str(&line);
            output.push('\n');
        }
        output
    });
    // stderr appended into same capture via separate thread joined below is overkill;
    // read stderr after stdout completes.
    let err_output: String = BufReader::new(stderr)
        .lines()
        .map_while(Result::ok)
        .collect::<Vec<_>>()
        .join("\n");

    let mut output = out_handle.join().unwrap_or_default();
    if !err_output.is_empty() {
        output.push_str(&err_output);
        output.push('\n');
    }

    let status = child.wait()?;
    Ok(RunResult {
        success: status.success(),
        exit_code: status.code(),
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::Module;
    use std::fs;

    fn temp_module(name: &str, body: &str) -> (std::path::PathBuf, Module) {
        let dir = std::env::temp_dir().join(format!("bazzitify-run-{}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let m = Module::parse(name, body).unwrap();
        fs::write(dir.join(format!("{name}.sh")), body).unwrap();
        (dir, m)
    }

    #[test]
    fn runs_apply_and_captures_output() {
        let (dir, m) = temp_module(
            "hello",
            "#!/bin/bash\n# desc: t\nmodule_apply() { echo ran-ok; }\n",
        );
        let r = run_module(&dir, &m, "apply").unwrap();
        assert!(r.success);
        assert!(r.output.contains("ran-ok"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failing_module_reports_failure_and_stderr() {
        let (dir, m) = temp_module(
            "boom",
            "#!/bin/bash\nmodule_apply() { echo oops >&2; return 3; }\n",
        );
        let r = run_module(&dir, &m, "apply").unwrap();
        assert!(!r.success);
        assert_eq!(r.exit_code, Some(3));
        assert!(r.output.contains("oops"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
