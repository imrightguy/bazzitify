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
/// With `dry_run`, the module body is printed instead of executed: every
/// line of the requested function is echoed with a `[dry-run]` prefix and
/// nothing is run.
pub fn run_module(modules_dir: &Path, module: &Module, action: &str) -> std::io::Result<RunResult> {
    run_module_opts(modules_dir, module, action, RunOpts::default())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RunOpts {
    pub dry_run: bool,
}

pub fn run_module_opts(
    modules_dir: &Path,
    module: &Module,
    action: &str,
    opts: RunOpts,
) -> std::io::Result<RunResult> {
    let script = modules_dir.join(format!("{}.sh", module.name));
    let inner = format!(
        "source {script:?}; module_{action} \"$@\"",
        script = script,
        action = action
    );
    let cmd_str = if opts.dry_run {
        // Print the function body without executing it.
        format!(
            "source {script:?}; declare -f module_{action} | sed 's/^/[dry-run] /'",
            script = script,
            action = action
        )
    } else {
        inner
    };
    let mut child = Command::new("bash")
        .arg("-c")
        .arg(&cmd_str)
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
    fn dry_run_prints_body_without_executing() {
        let (dir, m) = temp_module(
            "touchy",
            "#!/bin/bash\nmodule_apply() { echo SHOULD-NOT-RUN; touch /tmp/bz-dryrun-marker; }\n",
        );
        let _ = std::fs::remove_file("/tmp/bz-dryrun-marker");
        let r = run_module_opts(&dir, &m, "apply", RunOpts { dry_run: true }).unwrap();
        assert!(r.success);
        assert!(r.output.contains("[dry-run]"));
        assert!(r.output.contains("SHOULD-NOT-RUN")); // body shown…
        assert!(!std::path::Path::new("/tmp/bz-dryrun-marker").exists()); // …not run
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

    #[test]
    fn batch_apply_runs_multiple_modules_in_order() {
        let dir = std::env::temp_dir().join(format!("bazzitify-run-{}-batch", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let m1 = Module::parse("a", "# desc: A\nmodule_apply() { echo 'apply-a'; }\n").unwrap();
        let m2 = Module::parse("b", "# desc: B\nmodule_apply() { echo 'apply-b'; }\n").unwrap();
        let m3 = Module::parse("c", "# desc: C\nmodule_apply() { echo 'apply-c'; }\n").unwrap();

        fs::write(
            dir.join("a.sh"),
            "# desc: A\nmodule_apply() { echo 'apply-a'; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.sh"),
            "# desc: B\nmodule_apply() { echo 'apply-b'; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("c.sh"),
            "# desc: C\nmodule_apply() { echo 'apply-c'; }\n",
        )
        .unwrap();

        let modules = vec![m1, m2, m3];
        let mut outputs = Vec::new();
        for m in &modules {
            let r = run_module(&dir, m, "apply").unwrap();
            assert!(r.success, "module {} failed: {}", m.name, r.output);
            outputs.push(r.output);
        }

        // Verify each module ran and produced output
        assert!(outputs[0].contains("apply-a"));
        assert!(outputs[1].contains("apply-b"));
        assert!(outputs[2].contains("apply-c"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn batch_dry_run_shows_all_modules_without_executing() {
        let dir =
            std::env::temp_dir().join(format!("bazzitify-run-{}-batch-dry", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let m1 = Module::parse(
            "a",
            "# desc: A\nmodule_apply() { echo 'apply-a'; touch /tmp/bz-batch-marker-a; }\n",
        )
        .unwrap();
        let m2 = Module::parse(
            "b",
            "# desc: B\nmodule_apply() { echo 'apply-b'; touch /tmp/bz-batch-marker-b; }\n",
        )
        .unwrap();

        fs::write(
            dir.join("a.sh"),
            "# desc: A\nmodule_apply() { echo 'apply-a'; touch /tmp/bz-batch-marker-a; }\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.sh"),
            "# desc: B\nmodule_apply() { echo 'apply-b'; touch /tmp/bz-batch-marker-b; }\n",
        )
        .unwrap();

        let _ = std::fs::remove_file("/tmp/bz-batch-marker-a");
        let _ = std::fs::remove_file("/tmp/bz-batch-marker-b");

        let modules = vec![m1, m2];
        for m in &modules {
            let r = run_module_opts(&dir, m, "apply", RunOpts { dry_run: true }).unwrap();
            assert!(r.success);
            assert!(r.output.contains("[dry-run]"));
            assert!(r.output.contains(&format!("apply-{}", m.name)));
        }

        // Markers should not exist because dry-run doesn't execute
        assert!(!std::path::Path::new("/tmp/bz-batch-marker-a").exists());
        assert!(!std::path::Path::new("/tmp/bz-batch-marker-b").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
