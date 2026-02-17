use std::io::{self, BufRead, IsTerminal, Read};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use serde_json::Value;

use jdx::app::App;
use jdx::engine;
use jdx::format::{detect_format, format_output, parse_input, DataFormat};

/// Reopen `/dev/tty` as stdin (fd 0) so that both crossterm's event reader
/// and `enable_raw_mode()` can access the real terminal after data was piped
/// through stdin.
#[cfg(unix)]
fn reopen_tty_stdin() -> Result<()> {
    use std::os::unix::io::AsRawFd;

    // Open with read+write to match what crossterm expects internally
    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .context(
            "Failed to open /dev/tty — interactive mode requires a terminal.\n\
             Hint: use --non-interactive when piping data through other programs.",
        )?;

    let tty_fd = tty.as_raw_fd();

    // SAFETY: dup2 atomically replaces fd 0 with a copy of tty_fd.
    // This is safe because we own both file descriptors.
    let ret = unsafe { libc::dup2(tty_fd, libc::STDIN_FILENO) };
    if ret == -1 {
        bail!(
            "Failed to redirect /dev/tty to stdin (errno: {})",
            std::io::Error::last_os_error()
        );
    }

    // Intentionally leak `tty` — its fd is now duplicated onto fd 0.
    // Dropping it would close tty_fd but fd 0 remains valid.
    std::mem::forget(tty);
    Ok(())
}

#[cfg(not(unix))]
fn reopen_tty_stdin() -> Result<()> {
    bail!(
        "Interactive mode with piped input is not supported on this platform.\n\
         Use --non-interactive or pass a file argument instead."
    );
}

/// Duplicate stdin (fd 0) before it gets replaced by `/dev/tty`, so we can
/// keep reading the original pipe in a background thread.
#[cfg(unix)]
fn dup_stdin_fd() -> Result<std::fs::File> {
    use std::os::unix::io::FromRawFd;

    // SAFETY: dup() returns a new fd that is a copy of fd 0 (the pipe).
    let new_fd = unsafe { libc::dup(libc::STDIN_FILENO) };
    if new_fd == -1 {
        bail!(
            "Failed to dup stdin (errno: {})",
            std::io::Error::last_os_error()
        );
    }
    // SAFETY: new_fd is a valid, owned file descriptor from dup().
    Ok(unsafe { std::fs::File::from_raw_fd(new_fd) })
}

/// Read NDJSON lines from `reader` until `deadline` or EOF, returning the
/// collected content. This gives a quick initial batch without blocking forever
/// on a streaming source.
fn read_initial_ndjson(
    reader: &mut io::BufReader<std::fs::File>,
    deadline: Instant,
) -> (String, bool) {
    let mut content = String::new();
    let mut line = String::new();
    let mut hit_eof = false;
    loop {
        if Instant::now() >= deadline {
            break;
        }
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => {
                hit_eof = true;
                break;
            }
            Ok(_) => {
                content.push_str(&line);
            }
            Err(_) => {
                hit_eof = true;
                break;
            }
        }
    }
    (content, hit_eof)
}

/// Background thread that reads remaining NDJSON lines from the pipe and sends
/// parsed values over the channel. Exits on EOF or channel disconnect.
fn stdin_reader_thread(reader: io::BufReader<std::fs::File>, tx: mpsc::Sender<Value>) {
    for line in reader.lines() {
        match line {
            Ok(l) => {
                let trimmed = l.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&trimmed) {
                    Ok(val) => {
                        if tx.send(val).is_err() {
                            return; // receiver dropped
                        }
                    }
                    Err(_) => continue, // skip malformed lines
                }
            }
            Err(_) => return, // pipe error / closed
        }
    }
}

/// jdx — JSON Data eXplorer
///
/// An interactive, AI-augmented terminal tool for exploring JSON data.
/// Pipe JSON from stdin or pass a file path as an argument.
#[derive(Parser, Debug)]
#[command(name = "jdx", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    viewer: ViewerArgs,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the interactive setup wizard to configure AI providers
    Init,
}

#[derive(Parser, Debug)]
struct ViewerArgs {
    /// File to read JSON from (reads stdin if omitted)
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// Initial query (e.g., ".users[0]")
    #[arg(short = 'Q', long = "query")]
    initial_query: Option<String>,

    /// Output the query string instead of the result (for piping to jq)
    #[arg(short = 'q', long = "query-output")]
    query_output: bool,

    /// Input format (auto-detected if omitted): json, yaml, toml, csv, ndjson
    #[arg(short = 'i', long = "input")]
    input_format: Option<String>,

    /// Output format (default: json): json, yaml, toml, csv, ndjson
    #[arg(short = 'o', long = "output")]
    output_format: Option<String>,

    /// Monochrome output (no colors)
    #[arg(short = 'M', long = "monochrome")]
    monochrome: bool,

    /// Pretty-print result on exit
    #[arg(short = 'p', long = "pretty", default_value = "true")]
    pretty: bool,

    /// Non-interactive mode: evaluate query and print result
    #[arg(long = "non-interactive")]
    non_interactive: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Command::Init) = cli.command {
        return jdx::init::run_wizard();
    }

    let viewer = &cli.viewer;

    // Check if we should use the streaming NDJSON path:
    // stdin is piped + format is explicitly NDJSON + not non-interactive
    let is_stdin_piped = !io::stdin().is_terminal() && viewer.file.is_none();
    let is_ndjson = matches!(
        viewer.input_format.as_deref(),
        Some("ndjson") | Some("jsonl")
    );
    let use_streaming = is_stdin_piped && is_ndjson && !viewer.non_interactive;

    if use_streaming {
        #[cfg(unix)]
        {
            // Dup the pipe fd BEFORE reopen_tty_stdin replaces fd 0
            let pipe_file = dup_stdin_fd()?;
            let mut reader = io::BufReader::new(pipe_file);

            // Read initial batch with a short deadline
            let deadline = Instant::now() + Duration::from_millis(500);
            let (initial_content, hit_eof) = read_initial_ndjson(&mut reader, deadline);

            if initial_content.trim().is_empty() {
                bail!("No NDJSON data received from stdin within the initial timeout.");
            }

            let data = parse_input(&initial_content, DataFormat::Ndjson)
                .context("Failed to parse initial NDJSON data")?;

            // Now reopen /dev/tty so crossterm can read key events
            reopen_tty_stdin()?;

            let mut app = App::new(data, viewer.query_output, viewer.monochrome);

            // If the pipe hasn't ended, spawn background reader thread
            if !hit_eof {
                let (tx, rx) = mpsc::channel();
                app.set_stdin_rx(rx);
                std::thread::spawn(move || stdin_reader_thread(reader, tx));
            }

            if let Some(ref q) = viewer.initial_query {
                app.query = q.clone();
                app.cursor = q.len();
            }

            // Set up terminal
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;

            let result = run_app(&mut terminal, &mut app);

            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            result?;

            if app.confirmed {
                print_output(&app, viewer)?;
            }

            return Ok(());
        }

        #[cfg(not(unix))]
        {
            bail!(
                "Streaming NDJSON from stdin is not supported on this platform.\n\
                 Use --non-interactive or pass a file argument instead."
            );
        }
    }

    // Non-streaming path (original behavior)
    let content = read_input(viewer)?;

    let input_format = match &viewer.input_format {
        Some(fmt) => DataFormat::from_str_name(fmt)?,
        None => detect_format(&content),
    };

    let data = parse_input(&content, input_format).context("Failed to parse input data")?;

    if viewer.non_interactive {
        let query_str = viewer.initial_query.as_deref().unwrap_or(".");
        let segments = engine::query::parse(query_str)?;
        let result = engine::json::traverse(&data, &segments);
        match result.value {
            Some(val) => {
                let output = format_output_value(&val, viewer)?;
                print!("{output}");
                return Ok(());
            }
            None => {
                bail!("No match for query: {query_str}");
            }
        }
    }

    if !io::stdin().is_terminal() {
        reopen_tty_stdin()?;
    }

    let mut app = App::new(data, viewer.query_output, viewer.monochrome);

    if let Some(ref q) = viewer.initial_query {
        app.query = q.clone();
        app.cursor = q.len();
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;

    if app.confirmed {
        print_output(&app, viewer)?;
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| app.render(frame))?;

        if app.should_quit {
            break;
        }

        // Check for AI results from background thread
        app.poll_ai_result();

        // Check for new streaming NDJSON lines
        app.poll_stdin();

        // Poll for events with a small timeout for responsive rendering
        if event::poll(Duration::from_millis(50))? {
            let evt = event::read()?;
            if let Event::Key(_) = evt {
                app.handle_event(evt);
            } else if let Event::Resize(_, _) = evt {
                // Terminal will redraw on next loop iteration
            }
        }
    }

    Ok(())
}

fn read_input(viewer: &ViewerArgs) -> Result<String> {
    if let Some(ref path) = viewer.file {
        std::fs::read_to_string(path).context(format!("Failed to read file: {path}"))
    } else if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
        Ok(buf)
    } else {
        bail!(
            "No input provided. Pipe JSON to stdin or pass a file path:\n\
             \n  echo '{{\"key\": \"value\"}}' | jdx\n\
             \n  jdx data.json\n"
        );
    }
}

fn print_output(app: &App, viewer: &ViewerArgs) -> Result<()> {
    let output = if app.query_output_mode {
        app.query.clone()
    } else {
        let value = {
            let segments = engine::query::parse(&app.query).unwrap_or_default();
            let result = engine::json::traverse(&app.data, &segments);
            result.value
        };
        match value {
            Some(val) => format_output_value(&val, viewer)?,
            None => String::new(),
        }
    };
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}

fn format_output_value(value: &serde_json::Value, viewer: &ViewerArgs) -> Result<String> {
    let output_format = match &viewer.output_format {
        Some(fmt) => DataFormat::from_str_name(fmt)?,
        None => DataFormat::Json,
    };
    format_output(value, output_format)
}
