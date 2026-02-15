use std::io::{self, IsTerminal, Read};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

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

/// jdx — JSON Data eXplorer
///
/// An interactive, AI-augmented terminal tool for exploring JSON data.
/// Pipe JSON from stdin or pass a file path as an argument.
#[derive(Parser, Debug)]
#[command(name = "jdx", version, about, long_about = None)]
struct Cli {
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

    // Read input data
    let content = read_input(&cli)?;

    // Determine input format
    let input_format = match &cli.input_format {
        Some(fmt) => DataFormat::from_str_name(fmt)?,
        None => detect_format(&content),
    };

    // Parse input
    let data = parse_input(&content, input_format).context("Failed to parse input data")?;

    // Non-interactive mode: evaluate query and print result, then exit
    if cli.non_interactive {
        let query_str = cli.initial_query.as_deref().unwrap_or(".");
        let segments = engine::query::parse(query_str)?;
        let result = engine::json::traverse(&data, &segments);
        match result.value {
            Some(val) => {
                let output = format_output_value(&val, &cli)?;
                print!("{output}");
                return Ok(());
            }
            None => {
                bail!("No match for query: {query_str}");
            }
        }
    }

    // If stdin was piped, reopen /dev/tty so crossterm can read key events
    if !io::stdin().is_terminal() {
        reopen_tty_stdin()?;
    }

    // Interactive mode
    let mut app = App::new(data, cli.query_output, cli.monochrome);

    if let Some(ref q) = cli.initial_query {
        app.query = q.clone();
        app.cursor = q.len();
    }

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main event loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result?;

    // Output result
    if app.confirmed {
        let output = if app.query_output_mode {
            app.query.clone()
        } else {
            let value = {
                let segments = engine::query::parse(&app.query).unwrap_or_default();
                let result = engine::json::traverse(&app.data, &segments);
                result.value
            };
            match value {
                Some(val) => format_output_value(&val, &cli)?,
                None => String::new(),
            }
        };
        if !output.is_empty() {
            println!("{output}");
        }
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

fn read_input(cli: &Cli) -> Result<String> {
    if let Some(ref path) = cli.file {
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

fn format_output_value(value: &serde_json::Value, cli: &Cli) -> Result<String> {
    let output_format = match &cli.output_format {
        Some(fmt) => DataFormat::from_str_name(fmt)?,
        None => DataFormat::Json,
    };
    format_output(value, output_format)
}
