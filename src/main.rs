// kp: AtCoder project management CLI
// ------------------------------------------------------------
// * kp new <contest_id>      : generate contest workspace
// * kp test <contest_id> <problem> : build & `oj test` a single task
// ------------------------------------------------------------

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{exit, Command},
};

/// CLI definition
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Init the kp-rust template
    Init {},
    /// Create a new contest workspace
    New {
        /// Contest ID (e.g. abc300)
        contest: String,
    },
    /// Build & `oj test` a problem
    Test {
        /// Contest ID (e.g. abc300)
        contest: String,
        /// Problem ID letter (e.g. a)
        problem: String,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Init {} => init_template(),
        Cmd::New { contest } => create_contest(&contest),
        Cmd::Test { contest, problem } => test_problem(&contest, &problem),
    }
}

//
// -------- sub-command implementations
//

/// `kp init`
fn init_template() -> Result<()> {
    // 1. Obtain the path printed by `acc config-dir`
    let output = Command::new("acc")
        .arg("config-dir")
        .output()
        .context("failed to start `acc config-dir`")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "`acc config-dir` exited with status {}",
            output.status
        ));
    }
    let config_dir =
        String::from_utf8(output.stdout).context("`acc config-dir` produced non-UTF-8 output")?;
    // Remove trailing new-line(s) and convert to PathBuf
    let config_dir = PathBuf::from(config_dir.trim());

    // 2. Decide whether `kp-rust` exists
    let kp_path = config_dir.join("kp-rust");

    if kp_path.exists() {
        // 3-a. Pull the latest changes
        let status = Command::new("git")
            .arg("pull")
            .current_dir(&kp_path)
            .status()
            .context("failed to run `git pull`")?;

        if !status.success() {
            return Err(anyhow::anyhow!("`git pull` failed with status {}", status));
        }
    } else {
        // 3-b. Clone the repository
        let status = Command::new("git")
            .arg("clone")
            .arg("https://github.com/wogikaze/kp-rust")
            .current_dir(&config_dir)
            .status()
            .context("failed to run `git clone`")?;

        if !status.success() {
            return Err(anyhow::anyhow!("`git clone` failed with status {}", status));
        }
    }

    // 4. Set Config the template
    let default_template = Command::new("acc")
        .arg("config")
        .arg("default-template")
        .output()
        .context("failed to run `acc config default-template`")?;

    let status = default_template.status;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "`acc config default-template` failed with status {}",
            status
        ));
    }
    let current_template = String::from_utf8(default_template.stdout)
        .context("`acc config default-template` produced non-UTF-8 output")?;
    if current_template.trim() != "kp-rust" {
        // acc config default-template
        let set_template = Command::new("acc")
            .arg("config")
            .arg("default-template")
            .arg("kp-rust")
            .status()
            .context("failed to run `acc config default-template kp-rust`")?;
        if !set_template.success() {
            return Err(anyhow::anyhow!(
                "`acc config default-template kp-rust` failed with status {}",
                set_template
            ));
        }
    }
    Command::new("acc")
        .arg("config")
        .arg("default-task-dirname-format")
        .arg("./")
        .status()
        .context("failed to run `acc config default-task-dirname-format ./`")?;
    
    Command::new("acc")
        .arg("config")
        .arg("default-task-choice")
        .arg("all")
        .status()
        .context("failed to run `acc config default-task-choice all`")?;
    
    Ok(())
}

/// `kp new`
fn create_contest(contest: &str) -> Result<()> {
    let root = Path::new(contest);
    if root.exists() {
        bail!("Directory {contest} already exists");
    }
    // Remove directories
    // Create the contest directory
    Command::new("acc")
        .arg("new")
        .arg(contest)
        .status()
        .context(format!("failed to run `acc new {}`", contest))?;

    // Read contest.acc.json
    let acc_json = root.join("contest.acc.json");
    let acc_content =
        fs::read_to_string(&acc_json).context(format!("failed to read {}", acc_json.display()))?;
    // Parse Problem IDs

    println!("Read contest configuration: {}", acc_content);
    Ok(())
}

/// Generate one problem sub-crate
fn make_problem(root: &Path, contest: &str, p: char) -> Result<()> {
    let dir = root.join(p.to_string());
    let crate_name = format!("{contest}_{p}");
    // cargo new --bin <dir> --name <crate_name>
    cmd("cargo")
        .args(["new", "--bin"])
        .arg(&dir)
        .args(["--name", &crate_name])
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("cargo new failed for {p}"))?;

    // overwrite src/main.rs with simple template
    let main = dir.join("src").join("main.rs");
    fs::write(
        &main,
        TEMPLATE.replace(
            "{{TITLE}}",
            &format!("{contest} {}", p.to_ascii_uppercase()),
        ),
    )?;
    println!("  • problem {p}");
    Ok(())
}

/// `kp test`
fn test_problem(contest: &str, problem: &str) -> Result<()> {
    let dir: PathBuf = [contest, problem].iter().collect();
    if !dir.exists() {
        bail!("{} does not exist", dir.display());
    }

    println!("🔧  cargo run --release");
    cmd("cargo")
        .current_dir(&dir)
        .args(["run", "--release"])
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("cargo run failed"))?;

    println!("🧪  oj test");
    cmd("oj")
        .current_dir(&dir)
        .arg("test")
        .status()?
        .success()
        .then_some(())
        .ok_or_else(|| anyhow!("oj test failed"))?;

    Ok(())
}

/// Convenience wrapper to spawn a Command
fn cmd<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    Command::new(program)
}

//
// -------- file templates
//

const TEMPLATE: &str = r#"// {{TITLE}}
// ------------------------------------------------------------
// Write your solution here.
// ------------------------------------------------------------
use std::io::{self, Read};

fn main() {
    // Fast input
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap();

    // TODO: implement
    println!("0"); // placeholder
}
"#;
