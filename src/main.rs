// Necessary crate and module imports
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::io::Write;
use std::time::Instant;

/// AtCoder project management program
///
/// Usage examples:
/// - kp.exe <contest_number> new
/// - kp.exe <contest_number> <problem_letter> <action> [sample_number]
///   (action: "test", "submit", or "debug"; sample_number is optional for "debug", default is 1)
/// You can specify the execution directory with the --root_dir (-r) option.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Contest number (generates project name "abc<contest_number>")
    contest: String,
    /// "new" or a problem identifier (e.g., "a", "b")
    arg: String,
    /// Action for problem commands: "test", "submit", or "debug" (optional; only used when arg is a problem identifier)
    action: Option<String>,
    /// Optional sample number for the debug action (default: 1)
    sample: Option<String>,
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

fn run_binary_directly(binary_path: &str, input_contents: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut process = Command::new(binary_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // Write the input_contents to the binary's stdin.
    if let Some(ref mut stdin) = process.stdin {
        stdin.write_all(input_contents.as_bytes())?;
    }

    let output = process.wait_with_output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(stdout)
}


fn main() {
    // Parse command-line arguments
    let args = Args::parse();
    let mode = highlight::HighlightMode::TrueColor;

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
        if !run_command_via_powershell("cargo install cargo-expand", &base_dir) {
            eprintln!("Error installing cargo-expand");
            std::process::exit(1);
        }
        if !run_command_via_powershell(&cmd_str, &base_dir) {
            eprintln!("Error executing new project command");
            std::process::exit(1);
        }
        let project_path = base_dir.join(&project_dir);
        match fs::read_dir(&project_path) {
            Ok(entries) => {
                // Iterate through each subdirectory in the project directory
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // Insert the URL comment at the beginning of main.rs if it exists.
                        let main_rs_path = path.join("main.rs");
                        if main_rs_path.exists() {
                            // Extract the problem letter from the deepest folder name.
                            let problem_letter = path.file_name().unwrap().to_str().unwrap();
                            // Construct the URL with the project directory and problem letter.
                            let url = format!("// https://atcoder.jp/contests/{}/tasks/{}_{}\n\n\n", project_dir, project_dir, problem_letter);
                            // Read the current contents of main.rs and remove BOM if present.
                            match fs::read_to_string(&main_rs_path) {
                                Ok(original_content) => {
                                    let cleaned_content = original_content.trim_start_matches("\u{feff}");
                                    // Prepend the URL line to the cleaned content.
                                    let new_content = format!("{}{}", url, cleaned_content);
                                    // Write the new content back to main.rs.
                                    if let Err(e) = fs::write(&main_rs_path, new_content) {
                                        eprintln!("Failed to write to {:?}: {}", main_rs_path, e);
                                        std::process::exit(1);
                                    }
                                },
                                Err(e) => {
                                    eprintln!("Failed to read {:?}: {}", main_rs_path, e);
                                    std::process::exit(1);
                                }
                            }
                            if !run_command_via_powershell("mkdir expand", &path) {
                                eprintln!("Failed to make dir expand {:?}", &path);
                                std::process::exit(1);
                            }
                        }
                        // Build the project in the subdirectory.
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
            if !run_command_via_powershell("cargo build", &problem_dir) {
                eprintln!("cargo build failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
            let oj_test_cmd = "oj test -c \"target/debug/bin.exe\" -d ./tests";
            if !run_command_via_powershell(oj_test_cmd, &problem_dir) {
                eprintln!("Tests failed in directory {:?}. Submission aborted.", problem_dir);
                std::process::exit(1);
            }
            let cmd_str = "npx atcoder-cli submit";
            if !run_command_via_powershell(cmd_str, &problem_dir) {
                eprintln!("npx atcoder-cli submit failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }
        } else if action == "debug" {
            // Debug mode: compile, show input, and display output in a beautified format
            if !run_command_via_powershell("$Env:RUST_BACKTRACE = 1 ; cargo expand | out-file -filepath expand/debug.rs ; cargo expand --release | out-file -filepath expand/main.rs ; cargo build ; cargo build --release", &problem_dir) {
                eprintln!("cargo build failed in directory {:?}", problem_dir);
                std::process::exit(1);
            }

            // Use provided sample number or default to "1"
            let sample_number = args.sample.unwrap_or_else(|| "1".to_string());
            let sample_file_in_path = problem_dir.join(format!("tests/sample-{}.in", sample_number));
            let sample_file_out_path = problem_dir.join(format!("tests/sample-{}.out", sample_number));

            println!("{}==================== [input] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
            let input_contents = match fs::read_to_string(&sample_file_in_path) {
                Ok(contents) => contents.trim_start_matches("\u{feff}").to_string(), // Remove BOM if present
                Err(e) => {
                    eprintln!("Failed to read sample input file {:?}: {}", sample_file_in_path, e);
                    std::process::exit(1);
                }
            };
            print!("{}", input_contents);
            println!("{}{:?}{}", highlight::bgcolors::blue(&mode), input_contents, highlight::reset(&mode));

            println!("{}==================== [debug output] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
            // Construct the debug binary path from problem_dir+"/target/debug/bin"
            let debug_bin_path = problem_dir.join("target").join("debug").join("bin");
            // Run the debug command with the debug binary directly using input_contents.
            match run_binary_directly(
                debug_bin_path.to_str().expect("Failed to convert debug binary path to string"),
                &input_contents
            ) {
                Ok(debug_output) => {
                    print!("{}", debug_output);
                },
                Err(e) => {
                    eprintln!("Debug run (debug binary) failed in directory {:?}: {}", problem_dir, e);
                    std::process::exit(1);
                }
            }

            println!("{}==================== [output] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
            // Construct the release binary path from problem_dir+"/target/release/bin"
            let release_bin_path = problem_dir.join("target").join("release").join("bin");

            // Measure execution time for the release binary run
            let start = Instant::now();
            let release_output = match run_binary_directly(
                release_bin_path.to_str().expect("Failed to convert release binary path to string"),
                &input_contents
            ) {
                Ok(output) => {
                    print!("{}", output);
                    println!("{}{:?}{}", highlight::bgcolors::blue(&mode), output, highlight::reset(&mode));
                    output // Store the output for comparison later.
                },
                Err(e) => {
                    eprintln!("Debug run (release binary) failed in directory {:?}: {}", problem_dir, e);
                    std::process::exit(1);
                }
            };
            let duration = start.elapsed();
            println!("{}Execution Time: {:?}{}", highlight::bgcolors::orange(&mode) , duration, highlight::reset(&mode));

            println!("{}==================== [expect] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
            let expected_output = match fs::read_to_string(&sample_file_out_path) {
                Ok(contents) => {
                    let cleaned = contents.trim_start_matches("\u{feff}").to_string(); // Remove BOM if present
                    print!("{}", cleaned);
                    cleaned
                },
                Err(e) => {
                    eprintln!("Failed to read expected output file {:?}: {}", sample_file_out_path, e);
                    std::process::exit(1);
                }
            };
            println!("{}{:?}{}", highlight::bgcolors::blue(&mode), expected_output, highlight::reset(&mode));

            // Check if the release output matches the expected output and display a message.
            println!("{}==================== [comparison result] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
            if release_output.trim() == expected_output.trim() {
                println!("{}✅ Output matches expected output.{}", highlight::bgcolors::lightblue(&mode), highlight::reset(&mode));
            } else {
                println!("{}❌ Output does not match expected output.{}", highlight::bgcolors::red(&mode), highlight::reset(&mode));
            }

            println!("{}==================== [complete] ===================={}", highlight::bgcolors::green(&mode), highlight::reset(&mode));
        } else {
            eprintln!("Unknown action: {}. Allowed actions are 'test', 'submit', or 'debug'.", action);
            std::process::exit(1);
        }
    } else {
        eprintln!("Invalid arguments. For new project creation, use: kp.exe <contest_number> new. For problem commands, use: kp.exe <contest_number> <problem_letter> <action> [sample_number]");
        std::process::exit(1);
    }
}

/// Highlighter module for syntax highlighting.
pub mod highlight {
    /// Highlight mode enum.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum HighlightMode {
        None,
        Color16,
        Color256,
        TrueColor,
    }

    impl HighlightMode {
        pub fn from_str(s: &str) -> HighlightMode {
            match s {
                "false" => HighlightMode::None,
                "16" => HighlightMode::Color16,
                "256" => HighlightMode::Color256,
                "true" => HighlightMode::TrueColor,
                _ => HighlightMode::None,
            }
        }
    }

    /// Returns the reset escape sequence.
    pub fn reset(mode: &HighlightMode) -> String {
        match mode {
            HighlightMode::None => "".to_string(),
            _ => "\x1b[0m".to_string(),
        }
    }

    /// Color functions.
    pub mod colors {
        use super::HighlightMode;
        pub fn pink(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[35m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;207m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;250;105;200m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn blue(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[34m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;27m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;50;50;255m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn white(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[37m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;15m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;255;255;255m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn green(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[32m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;82m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;100;230;60m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn red(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[31m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;196m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;250;80;50m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn yellow(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[33m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;11m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;240;230;0m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn orange(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[33m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;208m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;255;165;0m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
        pub fn lightblue(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16 => "\x1b[94m".to_string(),
                HighlightMode::Color256 => "\x1b[38;5;153m".to_string(),
                HighlightMode::TrueColor => "\x1b[38;2;53;255;255m".to_string(),
                HighlightMode::None => "".to_string(),
            }
        }
    }
    pub mod bgcolors {
        use super::HighlightMode;
        pub fn pink(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[45m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;88m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;60;20;60m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn blue(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[44m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;18m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;20;40;80m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn white(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[47m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;237m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;40;40;40m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn yellow(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[43m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;100m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;60;60;20m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn orange(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[43m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;95m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;70;40;10m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn lightblue(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[104m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;20m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;20;30;60m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn green(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[42m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;64m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;40;80;24m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
        pub fn red(mode: &HighlightMode) -> String {
            match mode {
                HighlightMode::Color16   => "\x1b[41m".to_string(),
                HighlightMode::Color256  => "\x1b[48;5;90m".to_string(),
                HighlightMode::TrueColor => "\x1b[48;2;60;20;20m".to_string(),
                HighlightMode::None      => "".to_string(),
            }
        }
    }

    /// Returns an escape code for opening parentheses color based on depth.
    pub fn paren_color(depth: usize, mode: &HighlightMode) -> String {
        if *mode == HighlightMode::None {
            return "".to_string();
        }
        match mode {
            HighlightMode::Color16 => {
                let palette = [91, 92, 93, 94, 95, 96];
                let code = palette[depth % palette.len()];
                format!("\x1b[{}m", code)
            },
            HighlightMode::Color256 => {
                let palette = [196, 202, 208, 214, 220, 226];
                let code = palette[depth % palette.len()];
                format!("\x1b[38;5;{}m", code)
            },
            HighlightMode::TrueColor => {
                let palette = [
                    (164, 219, 211),
                    (217, 201, 145),
                    (145, 189, 217),
                    (217, 187, 145),
                    (132, 137, 140),
                ];
                let (r, g, b) = palette[depth % palette.len()];
                format!("\x1b[38;2;{};{};{}m", r, g, b)
            },
            HighlightMode::None => "".to_string(),
        }
    }

    /// Colorize a plain string with the given color (by name) for foreground.
    pub fn colorize_plain(text: &str, color: &str, mode: &HighlightMode) -> String {
        if *mode == HighlightMode::None {
            return text.to_string();
        }
        let color_code = match color {
            "pink" => colors::pink(mode),
            "blue" => colors::blue(mode),
            "white" => colors::white(mode),
            "green" => colors::green(mode),
            "red" => colors::red(mode),
            _ => "".to_string(),
        };
        format!("{}{}{}", color_code, text, reset(mode))
    }
}
