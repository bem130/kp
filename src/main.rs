// Necessary crate and module imports
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// AtCoder project management program
///
/// Usage examples:
/// - kp.exe <contest_number> new
/// - kp.exe <contest_number> <problem_letter> <action>    (action: "test" or "submit")
/// You can specify the execution directory with the --root_dir (-r) option.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Contest number (generates project name "abc<contest_number>")
    contest: String,
    /// "new" or a problem identifier (e.g., "a", "b")
    arg: String,
    /// Action for problem commands: "test" or "submit" (optional; only used when arg is a problem identifier)
    action: Option<String>,
    /// Base directory to execute commands (optional)
    #[arg(short, long)]
    root_dir: Option<String>,
}

/// Executes a command via PowerShell in the specified directory
fn run_command_via_powershell(command_str: &str, current_dir: &PathBuf) -> bool {
    let status = Command::new("powershell")
        .arg("-Command")
        .arg(command_str)
        .current_dir(current_dir)
        .status();

    match status {
        Ok(status) => status.success(),
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
            false
        }
    }
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Determine the base directory for command execution
    let base_dir = if let Some(root) = args.root_dir {
        PathBuf::from(root)
    } else {
        std::env::current_dir().expect("Failed to get the current directory")
    };

    // Create the project directory name (e.g., "abc300")
    let project_dir = format!("abc{}", args.contest);

    // "new" mode: create a new project and run cargo build in each subdirectory
    if args.arg == "new" && args.action.is_none() {
        let cmd_str = format!("npx atcoder-cli new {} --template rust", project_dir);
        if !run_command_via_powershell(&cmd_str, &base_dir) {
            eprintln!("Error executing new project command");
            std::process::exit(1);
        }
        let project_path = base_dir.join(&project_dir);
        match fs::read_dir(&project_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        println!("Running cargo build in directory {:?}", path);
                        if !run_command_via_powershell("cargo build", &path) {
                            eprintln!("cargo build failed in directory {:?}", path);
                            std::process::exit(1);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read project directory: {}", e);
                std::process::exit(1);
            }
        }
    } else if let Some(action) = args.action {
        // Problem command mode
        let problem_letter = args.arg;
        let problem_dir = base_dir.join(&project_dir).join(&problem_letter);

        if action == "test" {
            // Test mode: build and run tests
            if !run_command_via_powershell("cargo build", &problem_dir) {
                eprintln!("cargo build failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
            let oj_test_cmd = "oj test -c \"target/debug/bin.exe\" -d ./tests";
            if !run_command_via_powershell(oj_test_cmd, &problem_dir) {
                eprintln!("oj test failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
        } else if action == "submit" {
            // Submit mode: first run tests, then submit if tests pass
            // Execute cargo build
            if !run_command_via_powershell("cargo build", &problem_dir) {
                eprintln!("cargo build failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
            // Execute tests
            let oj_test_cmd = "oj test -c \"target/debug/bin.exe\" -d ./tests";
            if !run_command_via_powershell(oj_test_cmd, &problem_dir) {
                eprintln!("Tests failed in directory {:?}. Submission aborted.", problem_dir);
                std::process::exit(1);
            }
            // If tests pass, execute the submission command
            let cmd_str = "npx atcoder-cli submit";
            if !run_command_via_powershell(cmd_str, &problem_dir) {
                eprintln!("npx atcoder-cli submit failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
        } else {
            eprintln!("Unknown action: {}. Allowed actions are 'test' or 'submit'.", action);
            std::process::exit(1);
        }
    } else {
        eprintln!("Invalid arguments. For new project creation, use: kp.exe <contest_number> new. For problem commands, use: kp.exe <contest_number> <problem_letter> <action>");
        std::process::exit(1);
    }
}
