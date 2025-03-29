// Necessary crate and module imports
use clap::Parser;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

/// OSに応じたシェルコマンド実行関数
fn run_command(command_str: &str, current_dir: &Path) -> Result<(), String> {
    println!("cmd: '{}'　(dir: '{}')", command_str, current_dir.display());
    let status = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .arg("-Command")
            .arg(command_str)
            .current_dir(current_dir)
            .status()
    } else {
        Command::new("bash")
            .arg("-c")
            .arg(command_str)
            .current_dir(current_dir)
            .status()
    };
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!(
            "command '{}' failed with Error Code {} .",
            command_str, status
        )),
        Err(e) => Err(format!("failed to execute '{}' : {}", command_str, e)),
    }
}

/// バイナリを直接実行し、標準入力へ文字列を渡す関数
fn run_binary_directly(
    binary_path: &str,
    input_contents: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("run bin: {}", binary_path);
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

/// 指定されたビルドモード(debug/release)の実行ファイルパスを取得
fn get_executable_path(problem_dir: &Path, build_mode: &str) -> PathBuf {
    let mut path = problem_dir.join("target").join(build_mode);
    if cfg!(target_os = "windows") {
        path.push("bin.exe");
    } else {
        path.push("bin");
    }
    path
}

/// OSに応じた cargo expand とビルドのコマンド文字列を返す
fn get_expand_and_build_command() -> String {
    if cfg!(target_os = "windows") {
        "$Env:RUST_BACKTRACE = 1 ; cargo expand | out-file -filepath expand/debug.rs -Encoding utf8 ; cargo expand --release | out-file -filepath expand/main.rs -Encoding utf8 ; cargo build ; cargo build --release".to_string()
    } else {
        "RUST_BACKTRACE=1 cargo expand > expand/debug.rs && RUST_BACKTRACE=1 cargo expand --release > expand/main.rs && cargo build && cargo build --release".to_string()
    }
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
    let project_dir_name = format!("abc{}", args.contest);
    let project_dir = base_dir.join(&project_dir_name);

    // "new" mode: create a new project and run cargo build in each subdirectory
    if args.arg == "new" && args.action.is_none() {
        // 新規プロジェクト作成モード
        if let Err(e) = run_command("cargo install cargo-expand", &base_dir) {
            eprintln!("cargo-expand のインストールに失敗しました: {}", e);
            std::process::exit(1);
        }
        let new_project_cmd = format!("npx atcoder-cli new {} --template rust", project_dir_name);
        if let Err(e) = run_command(&new_project_cmd, &base_dir) {
            eprintln!(
                "新規プロジェクト作成コマンド '{}' でエラー: {}",
                new_project_cmd, e
            );
            std::process::exit(1);
        }

        // プロジェクトディレクトリ内の各サブディレクトリに対して処理
        match fs::read_dir(&project_dir) {
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
                            let url_comment = format!(
                                "// https://atcoder.jp/contests/{}/tasks/{}_{}\n\n\n",
                                project_dir_name, project_dir_name, problem_letter
                            );
                            // Read the current contents of main.rs and remove BOM if present.
                            match fs::read_to_string(&main_rs_path) {
                                Ok(original_content) => {
                                    let cleaned_content =
                                        original_content.trim_start_matches("\u{feff}");
                                    let new_content = format!("{}{}", url_comment, cleaned_content);
                                    if let Err(e) = fs::write(&main_rs_path, new_content) {
                                        eprintln!(
                                            "Failed to write to main.rs({}) : {}",
                                            main_rs_path.display(),
                                            e
                                        );
                                        std::process::exit(1);
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "Failed to read main.rs({}) : {}",
                                        main_rs_path.display(),
                                        e
                                    );
                                    std::process::exit(1);
                                }
                            }
                            // OSに応じた mkdir コマンド
                            let mkdir_cmd = if cfg!(target_os = "windows") {
                                "mkdir expand"
                            } else {
                                "mkdir -p expand"
                            };
                            if let Err(e) = run_command(mkdir_cmd, &path) {
                                eprintln!("Failed to make dir expand  ({}): {}", path.display(), e);
                                std::process::exit(1);
                            }
                        }
                        // Build the project in the subdirectory.
                        println!("Running cargo build in directory '{}'", path.display());
                        if let Err(e) = run_command("cargo build", &path) {
                            eprintln!(
                                "cargo build failed in directory ({}): {}",
                                path.display(),
                                e
                            );
                            std::process::exit(1);
                        }
                        if let Err(e) = run_command("cargo build --release", &path) {
                            eprintln!(
                                "cargo build failed in directory ({}): {}",
                                path.display(),
                                e
                            );
                            std::process::exit(1);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to read project directory ({}) : {}",
                    project_dir.display(),
                    e
                );
                std::process::exit(1);
            }
        }
    } else if let Some(action) = args.action {
        // Problem command mode
        let problem_letter = args.arg;
        let problem_dir = project_dir.join(&problem_letter);

        if action == "test" {
            // Test mode: build and run tests
            let expand_build_cmd = get_expand_and_build_command();
            if let Err(e) = run_command(&expand_build_cmd, &problem_dir) {
                eprintln!(
                    "cargo expand/build failed in directory ({}): {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
            let bin_path = if cfg!(target_os = "windows") {
                "target/release/bin.exe"
            } else {
                "target/release/bin"
            };
            let oj_test_cmd = format!("oj test -c \"{}\" -d ./tests", bin_path);
            if let Err(e) = run_command(&oj_test_cmd, &problem_dir) {
                eprintln!(
                    "oj test failed in directory ({}): {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
        } else if action == "submit" {
            // Submit mode: first run tests, then submit if tests pass
            let expand_build_cmd = get_expand_and_build_command();
            if let Err(e) = run_command(&expand_build_cmd, &problem_dir) {
                eprintln!(
                    "cargo expand/build failed in directory ({}): {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
            let bin_path = if cfg!(target_os = "windows") {
                "target/release/bin.exe"
            } else {
                "target/release/bin"
            };
            let oj_test_cmd = format!("oj test -c \"{}\" -d ./tests", bin_path);
            if let Err(e) = run_command(&oj_test_cmd, &problem_dir) {
                eprintln!(
                    "Tests failed in directory ({}). Submission aborted. {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
            let submit_cmd = "npx atcoder-cli submit";
            if let Err(e) = run_command(submit_cmd, &problem_dir) {
                eprintln!(
                    "npx atcoder-cli submit failed in directory ({}): {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
        } else if action == "debug" {
            // Debug mode: compile, show input, and display output in a beautified format
            let expand_build_cmd = get_expand_and_build_command();
            if let Err(e) = run_command(&expand_build_cmd, &problem_dir) {
                eprintln!(
                    "cargo expand/build failed in directory ({}): {}",
                    problem_dir.display(),
                    e
                );
                std::process::exit(1);
            }
            let sample_number = args.sample.unwrap_or_else(|| "1".to_string());
            let sample_in_path = problem_dir.join(format!("tests/sample-{}.in", sample_number));
            let sample_out_path = problem_dir.join(format!("tests/sample-{}.out", sample_number));

            println!(
                "{}==================== [input] ===================={}{}",
                highlight::bgcolors::green(&mode),
                highlight::reset(&mode),
                ""
            );
            let input_contents = match fs::read_to_string(&sample_in_path) {
                Ok(contents) => contents.trim_start_matches("\u{feff}").to_string(),
                Err(e) => {
                    eprintln!(
                        "Failed to read sample input file '{}' : {}",
                        sample_in_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };
            println!("{}", input_contents);

            println!(
                "{}==================== [debug output] ===================={}{}",
                highlight::bgcolors::green(&mode),
                highlight::reset(&mode),
                ""
            );
            // Construct the debug binary path from problem_dir+"/target/debug/bin"
            let debug_bin_path = get_executable_path(&problem_dir, "debug");
            // Run the debug command with the debug binary directly using input_contents.
            let debug_output = match run_binary_directly(
                debug_bin_path
                    .to_str()
                    .expect("Failed to convert debug binary path to string"),
                &input_contents,
            ) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(
                        "Debug run (debug binary) failed in directory ({}): {}",
                        problem_dir.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };
            println!("{}", debug_output);

            println!(
                "{}==================== [output] ===================={}{}",
                highlight::bgcolors::green(&mode),
                highlight::reset(&mode),
                ""
            );
            // Construct the release binary path from problem_dir+"/target/release/bin"
            let release_bin_path = get_executable_path(&problem_dir, "release");
            // Measure execution time for the release binary run
            let start = Instant::now();
            let release_output = match run_binary_directly(
                release_bin_path
                    .to_str()
                    .expect("Failed to convert release binary path to string"),
                &input_contents,
            ) {
                Ok(output) => output,
                Err(e) => {
                    eprintln!(
                        "Debug run (release binary) failed in directory ({}): {}",
                        problem_dir.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };
            let duration = start.elapsed();
            println!("{}", release_output);
            println!(
                "{}Execution Time: {:?}{}",
                highlight::bgcolors::orange(&mode),
                duration,
                highlight::reset(&mode)
            );

            println!(
                "{}==================== [expect] ===================={}{}",
                highlight::bgcolors::green(&mode),
                highlight::reset(&mode),
                ""
            );
            let expected_output = match fs::read_to_string(&sample_out_path) {
                Ok(contents) => {
                    let cleaned = contents.trim_start_matches("\u{feff}").to_string();
                    println!("{}", cleaned);
                    cleaned
                }
                Err(e) => {
                    eprintln!(
                        "Failed to read expected output file {:?}: {}",
                        sample_out_path.display(),
                        e
                    );
                    std::process::exit(1);
                }
            };

            // Check if the release output matches the expected output and display a message.
            println!(
                "{}==================== [comparison result] ===================={}{}",
                highlight::bgcolors::green(&mode),
                highlight::reset(&mode),
                ""
            );
            if release_output.trim() == expected_output.trim() {
                println!(
                    "{}[✅ Complete] Output matches expected output.{}",
                    highlight::bgcolors::lightblue(&mode),
                    highlight::reset(&mode)
                );
            } else {
                println!(
                    "{}[❌ Failed] Output does not match expected output.{}",
                    highlight::bgcolors::red(&mode),
                    highlight::reset(&mode)
                );
            }
        } else {
            eprintln!(
                "Unknown action: {}. Allowed actions are 'test', 'submit', or 'debug'.",
                action
            );
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
}
