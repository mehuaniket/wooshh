use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, File, OpenOptions};
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    action: Option<Action>,

    #[clap(name = "command")]
    command: Option<String>,

    #[clap(name = "args")]
    args: Vec<String>,

    #[clap(short = 'a', long, help = "Append to output file instead of overwriting")]
    append: bool,

    #[clap(short = 'o', long, help = "Write output to a file instead of stdout")]
    output: Option<String>,

    #[clap(long, help = "Only notify if execution time exceeds duration (e.g. 30s, 2m)")]
    notify_if_over: Option<String>,

    #[clap(long, help = "Only notify when command fails")]
    on_failure_only: bool,

    #[clap(long, help = "Suppress standard timing output")]
    quiet: bool,

    #[clap(long, help = "Emit machine-readable JSON output")]
    json: bool,

    #[clap(long, help = "Tag this run for grouping in stats/history")]
    tag: Option<String>,

    #[clap(long, help = "Do not persist this run to history")]
    no_history: bool,
}

#[derive(Subcommand)]
enum Action {
    Stats,
    Slowest {
        #[clap(short = 'n', long, default_value_t = 5)]
        limit: usize,
    },
    Regressions {
        #[clap(short = 'n', long, default_value_t = 5)]
        limit: usize,
    },
    Compare {
        #[clap(short = 'n', long, default_value_t = 3)]
        runs: usize,
        #[clap(name = "command")]
        command: String,
        #[clap(name = "args")]
        args: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct HistoryEntry {
    ts_epoch_secs: u64,
    command: String,
    tag: Option<String>,
    duration_ms: u64,
    success: bool,
    exit_code: i32,
}

#[derive(Serialize, Deserialize, Default)]
struct HistoryFile {
    entries: Vec<HistoryEntry>,
}

#[derive(Serialize)]
struct RunJsonOutput {
    command: String,
    duration_ms: u64,
    success: bool,
    exit_code: i32,
    notified: bool,
    tag: Option<String>,
}

fn write_line(writer: &mut Option<BufWriter<File>>, line: &str, quiet: bool) {
    if let Some(output_writer) = writer {
        let _ = output_writer.write_all(line.as_bytes());
    } else if !quiet {
        print!("{}", line);
    }
}

fn run_quiet_command(program: &str, args: &[&str]) -> bool {
    match Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

fn play_notification_sound(success: bool) {
    #[cfg(target_os = "macos")]
    {
        let sound_file = if success {
            "/System/Library/Sounds/Hero.aiff"
        } else {
            "/System/Library/Sounds/Basso.aiff"
        };
        if Path::new(sound_file).exists() && run_quiet_command("afplay", &[sound_file]) {
            return;
        }

        let beep_count = if success { "1" } else { "2" };
        if run_quiet_command("osascript", &["-e", &format!("beep {}", beep_count)]) {
            return;
        }
    }

    #[cfg(target_os = "linux")]
    {
        let canberra_id = if success { "complete" } else { "dialog-error" };
        if run_quiet_command("canberra-gtk-play", &["-i", canberra_id]) {
            return;
        }
    }

    // Fallback: terminal bell (can be disabled by terminal settings).
    let mut stdout = std::io::stdout();
    let bell_count = if success { 1 } else { 2 };
    for _ in 0..bell_count {
        let _ = stdout.write_all(b"\x07");
    }
    let _ = stdout.flush();
}

#[cfg(target_os = "macos")]
fn try_native_macos_notification(title: &str, body: &str, success: bool) -> bool {
    let env_path = std::env::var("WOOSHH_NOTIFIER_PATH").ok().map(std::path::PathBuf::from);
    let home_app = std::env::var_os("HOME").map(|home| {
        Path::new(&home).join("Applications").join("WooshhNotifier.app")
    });
    let candidate_apps = [
        Some(std::path::PathBuf::from("./notifier-macos/WooshhNotifier.app")),
        Some(std::path::PathBuf::from("/Applications/WooshhNotifier.app")),
        home_app,
    ];
    let candidate_bins = [
        env_path,
        Some(std::path::PathBuf::from(
            "./notifier-macos/WooshhNotifier.app/Contents/MacOS/wooshh-notifier",
        )),
        Some(std::path::PathBuf::from(
            "/Applications/WooshhNotifier.app/Contents/MacOS/wooshh-notifier",
        )),
    ];

    let mut found_candidate = false;
    for app_path in candidate_apps.iter().flatten() {
        if !app_path.exists() {
            continue;
        }
        found_candidate = true;
        let status = Command::new("open")
            .arg(app_path)
            .arg("--args")
            .arg("--title")
            .arg(title)
            .arg("--body")
            .arg(body)
            .arg("--kind")
            .arg(if success { "success" } else { "failure" })
            .stdout(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                return true;
            }
        }
    }

    for bin_path in candidate_bins.iter().flatten() {
        if !bin_path.exists() {
            continue;
        }
        found_candidate = true;
        let status = Command::new(bin_path)
            .arg("--title")
            .arg(title)
            .arg("--body")
            .arg(body)
            .arg("--kind")
            .arg(if success { "success" } else { "failure" })
            .stdout(Stdio::null())
            .status();
        if let Ok(s) = status {
            if s.success() {
                return true;
            }
        }
    }

    if found_candidate {
        eprintln!("wooshh: native macOS notifier was found but failed; using AppleScript fallback");
    } else {
        eprintln!("wooshh: native macOS notifier not found; using AppleScript fallback");
    }
    false
}

fn show_desktop_notification(title: &str, body: &str, success: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        if try_native_macos_notification(title, body, success) {
            return true;
        }

        // Preferred path: Notification Center banner.
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body.replace('"', "'"),
            title.replace('"', "'")
        );
        if run_quiet_command("osascript", &["-e", &script]) {
            return true;
        }

        // Last resort on macOS: visible dialog popup.
        let alert_script = format!(
            "display alert \"{}\" message \"{}\" as informational giving up after 5",
            title.replace('"', "'"),
            body.replace('"', "'")
        );
        if run_quiet_command("osascript", &["-e", &alert_script]) {
            return true;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if run_quiet_command("notify-send", &[title, body]) {
            return true;
        }
    }
    false
}

fn should_notify(success: bool, duration: Duration, notify_if_over: Option<Duration>, on_failure_only: bool) -> bool {
    if on_failure_only && success {
        return false;
    }
    if !success {
        return true;
    }
    if let Some(threshold) = notify_if_over {
        return duration >= threshold;
    }
    true
}

fn parse_threshold(input: &Option<String>) -> Option<Duration> {
    input
        .as_ref()
        .and_then(|v| humantime::parse_duration(v).ok())
}

fn history_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| {
        Path::new(&home)
            .join(".config")
            .join("wooshh")
            .join("history.toml")
    })
}

fn load_history() -> HistoryFile {
    if let Some(path) = history_path() {
        if let Ok(content) = read_to_string(path) {
            if let Ok(parsed) = toml::from_str::<HistoryFile>(&content) {
                return parsed;
            }
        }
    }
    HistoryFile::default()
}

fn save_history(history: &HistoryFile) {
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        if let Ok(data) = toml::to_string_pretty(history) {
            let _ = std::fs::write(path, data);
        }
    }
}

fn add_to_history(entry: HistoryEntry) {
    let mut history = load_history();
    history.entries.push(entry);
    if history.entries.len() > 2000 {
        let start = history.entries.len() - 2000;
        history.entries = history.entries[start..].to_vec();
    }
    save_history(&history);
}

fn percentile(mut values: Vec<u64>, p: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let idx = ((values.len() - 1) as f64 * p).round() as usize;
    values[idx]
}

fn command_hint(command: &str) -> Option<&'static str> {
    let cmd = command.split_whitespace().next().unwrap_or("");
    match cmd {
        "cargo" => Some("Hint: try `cargo check` for a faster compile pass, or run the failing package only."),
        "npm" | "pnpm" | "yarn" => Some("Hint: dependency/build failures are often fixed by a clean install before retry."),
        "pytest" | "python" => Some("Hint: rerun a narrow test target first to isolate the failing area quickly."),
        "go" => Some("Hint: run `go test ./...` to confirm whether failure is isolated or widespread."),
        _ => None,
    }
}

fn run_compare(runs: usize, command: &str, args: &[String]) {
    let mut durations = Vec::new();
    for _ in 0..runs.max(1) {
        let start = Instant::now();
        let status = Command::new(command)
            .args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        if status.is_err() {
            eprintln!("Failed to execute compare target command");
            return;
        }
        let elapsed = start.elapsed().as_millis() as u64;
        durations.push(elapsed);
    }
    let median = percentile(durations.clone(), 0.5);
    let mean = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
    let variance = durations
        .iter()
        .map(|d| {
            let diff = *d as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / durations.len() as f64;
    let stddev = variance.sqrt();
    println!(
        "compare\tcommand=\"{} {}\"\truns={}\tmedian_ms={}\tstddev_ms={:.2}",
        command,
        args.join(" "),
        durations.len(),
        median,
        stddev
    );
}

fn run_stats() {
    let history = load_history();
    if history.entries.is_empty() {
        println!("No history yet. Run a few commands first.");
        return;
    }
    let mut groups: HashMap<String, Vec<HistoryEntry>> = HashMap::new();
    for entry in history.entries {
        groups.entry(entry.command.clone()).or_default().push(entry);
    }
    let mut rows: Vec<(String, usize, u64, u64, f64)> = groups
        .into_iter()
        .map(|(cmd, entries)| {
            let durations: Vec<u64> = entries.iter().map(|e| e.duration_ms).collect();
            let p50 = percentile(durations.clone(), 0.5);
            let p95 = percentile(durations, 0.95);
            let failures = entries.iter().filter(|e| !e.success).count();
            let fail_rate = failures as f64 * 100.0 / entries.len() as f64;
            (cmd, entries.len(), p50, p95, fail_rate)
        })
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    println!("command\tcount\tp50_ms\tp95_ms\tfail_rate");
    for (cmd, count, p50, p95, fail_rate) in rows.into_iter().take(20) {
        println!("{cmd}\t{count}\t{p50}\t{p95}\t{fail_rate:.1}%");
    }
}

fn run_slowest(limit: usize) {
    let mut entries = load_history().entries;
    if entries.is_empty() {
        println!("No history yet. Run a few commands first.");
        return;
    }
    entries.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
    println!("slowest\tlimit={}", limit);
    for entry in entries.into_iter().take(limit) {
        println!(
            "{}\t{}ms\tsuccess={}\ttag={}",
            entry.command,
            entry.duration_ms,
            entry.success,
            entry.tag.unwrap_or_else(|| "-".to_string())
        );
    }
}

fn run_regressions(limit: usize) {
    let history = load_history();
    if history.entries.is_empty() {
        println!("No history yet. Run a few commands first.");
        return;
    }
    let mut grouped: HashMap<String, Vec<HistoryEntry>> = HashMap::new();
    for entry in history.entries {
        grouped.entry(entry.command.clone()).or_default().push(entry);
    }
    let mut regressions = Vec::new();
    for (cmd, mut entries) in grouped {
        entries.sort_by(|a, b| a.ts_epoch_secs.cmp(&b.ts_epoch_secs));
        if entries.len() < 3 {
            continue;
        }
        let last = entries.last().map(|e| e.duration_ms).unwrap_or(0);
        let baseline_slice = &entries[..entries.len() - 1];
        let baseline_avg = baseline_slice.iter().map(|e| e.duration_ms).sum::<u64>() as f64
            / baseline_slice.len() as f64;
        if baseline_avg <= 0.0 {
            continue;
        }
        let delta_pct = ((last as f64 - baseline_avg) / baseline_avg) * 100.0;
        if delta_pct > 20.0 {
            regressions.push((cmd, last, baseline_avg, delta_pct));
        }
    }
    regressions.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(Ordering::Equal));
    println!("command\tlast_ms\tbaseline_ms\tdelta_pct");
    for (cmd, last, baseline, delta) in regressions.into_iter().take(limit) {
        println!("{cmd}\t{last}\t{baseline:.1}\t{delta:.1}%");
    }
}

fn main() {
    let cli = Cli::parse();

    if let Some(action) = cli.action {
        match action {
            Action::Stats => run_stats(),
            Action::Slowest { limit } => run_slowest(limit),
            Action::Regressions { limit } => run_regressions(limit),
            Action::Compare { runs, command, args } => run_compare(runs, &command, &args),
        }
        return;
    }

    let command_name = match cli.command {
        Some(command) => command,
        None => {
            eprintln!("No command provided. Use `wooshh --help` for usage.");
            std::process::exit(2);
        }
    };
    let args = cli.args.clone();
    let start = Instant::now();

    let mut command = Command::new(&command_name);
    command.args(&args);
    let status = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("Failed to execute command");

    let real_time = start.elapsed();
    let mut output_writer: Option<BufWriter<File>> = None;

    if let Some(filename) = &cli.output {
        let file = OpenOptions::new()
            .write(true)
            .append(cli.append)
            .truncate(!cli.append)
            .create(true)
            .open(filename)
            .expect("Unable to open file");
        output_writer = Some(BufWriter::new(file));
    }

    let duration_ms = real_time.as_millis() as u64;
    let exit_code = status.code().unwrap_or(1);
    let success = status.success();
    let notify_threshold = parse_threshold(&cli.notify_if_over);
    let notified = should_notify(success, real_time, notify_threshold, cli.on_failure_only);

    if cli.json {
        let payload = RunJsonOutput {
            command: format!("{} {}", command_name, args.join(" ")).trim().to_string(),
            duration_ms,
            success,
            exit_code,
            notified,
            tag: cli.tag.clone(),
        };
        if let Ok(json) = serde_json::to_string(&payload) {
            write_line(&mut output_writer, &(json + "\n"), cli.quiet);
        }
    } else {
        write_line(
        &mut output_writer,
        &format!("real\t{:.2}\n", real_time.as_secs_f64()),
        cli.quiet,
    );
        write_line(&mut output_writer, "user\t0.00\n", cli.quiet);
        write_line(&mut output_writer, "sys\t0.00\n", cli.quiet);
    }

    if let Some(output_writer) = &mut output_writer {
        let _ = output_writer.flush();
    }

    if notified {
        play_notification_sound(success);
        let title = if success { "wooshh: command finished" } else { "wooshh: command failed" };
        let body = format!(
            "{}{} completed in {:.2}s (exit {})",
            command_name,
            if args.is_empty() { "".to_string() } else { format!(" {}", args.join(" ")) },
            real_time.as_secs_f64(),
            exit_code
        );
        let _ = show_desktop_notification(title, &body, success);
    }

    if !success {
        if let Some(hint) = command_hint(&command_name) {
            eprintln!("{hint}");
        }
    }

    if !cli.no_history {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        add_to_history(HistoryEntry {
            ts_epoch_secs: ts,
            command: format!("{} {}", command_name, args.join(" ")).trim().to_string(),
            tag: cli.tag.clone(),
            duration_ms,
            success,
            exit_code,
        });
    }

    if !success {
        std::process::exit(exit_code);
    }
}
