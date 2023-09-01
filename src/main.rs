use clap::Parser;
use rusty_audio::Audio;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::BufWriter;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

// Define a Cli struct to hold command line arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    // Define a command field to hold the command to execute
    #[clap(name = "command")]
    command: String,

    // Define an args field to hold arguments for the command
    #[clap(name = "args")]
    args: Vec<String>,

    // Define an append field to specify whether to append to an output file
    #[clap(
        short = 'a',
        long,
        help = "Append to output file instead of overwriting"
    )]
    append: bool,

    // Define an output field to specify an output file
    #[clap(short = 'o', long, help = "Write output to a file instead of stdout")]
    output: Option<String>,
}

fn main() {
    // Create a new Audio instance
    let mut audio = Audio::new();
    // Add a success sound to the Audio instance
    audio.add("success", "./success-trumpets.mp3");
    // Add an error sound to the Audio instance
    audio.add("error", "./beep-warning.mp3");

    // Parse command line arguments into a Cli instance
    let cli = Cli::parse();

    // Get the current time
    let start = Instant::now();

    // Create a new Command instance for the specified command
    let mut command = Command::new(cli.command);
    // Add arguments to the Command instance
    command.args(cli.args);

    // Execute the command and capture its output
    let output = command
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute command");

    // Get the current time again
    let end = Instant::now();

    // Calculate the real time it took for the command to execute
    let real_time = end - start;

    // Declare a variable to hold an optional output writer
    let mut output_writer: Option<BufWriter<File>> = None;

    // Check if an output file was specified
    if let Some(filename) = &cli.output {
        // Open or create the specified output file
        let file = OpenOptions::new()
            .append(cli.append)
            .create(true)
            .open(filename)
            .expect("Unable to open file");
        // Create a new BufWriter for writing to the output file
        output_writer = Some(BufWriter::new(file));
    }

    // Check if an output writer was created
    if let Some(output_writer) = &mut output_writer {
        // Write the real time to the output file using the BufWriter
        output_writer
            .write_all(format!("real\t{:.2}\n", real_time.as_secs_f64()).as_bytes())
            .unwrap();
    } else {
        // Print the real time to stdout if no output writer was created
        println!("real\t{:.2}\n", real_time.as_secs_f64());
    }

    // Get user and system times from process exit status code.
    let user_time = output.status.code().expect("Failed to get user time") as f64;
    let sys_time = output.status.code().expect("Failed to get system time") as f64;

    if let Some(output_writer) = &mut output_writer {
        output_writer
            .write_all(format!("user\t{:.2}\n", user_time).as_bytes())
            .unwrap();
        output_writer
            .write_all(format!("sys\t{:.2}\n", sys_time).as_bytes())
            .unwrap();
        output_writer.flush().unwrap();
        audio.play("success");
        audio.wait();
    } else {
        println!("user\t{:.2}\n", user_time);
        println!("sys\t{:.2}\n", sys_time);
        audio.play("success");
        audio.wait();
    }
}
