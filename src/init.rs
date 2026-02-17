use anyhow::Result;
use console::{style, Style};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Password, Select};
use std::net::TcpStream;
use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::config::{self, AiConfig};

fn theme() -> ColorfulTheme {
    ColorfulTheme {
        prompt_prefix: style(">".to_string()).for_stderr().cyan(),
        prompt_suffix: style("::".to_string()).for_stderr().dim(),
        prompt_style: Style::new().for_stderr().white().bold(),
        active_item_prefix: style(">>".to_string()).for_stderr().cyan().bold(),
        active_item_style: Style::new().for_stderr().cyan().bold(),
        inactive_item_prefix: style("  ".to_string()).for_stderr(),
        inactive_item_style: Style::new().for_stderr().dim(),
        success_prefix: style("[ok]".to_string()).for_stderr().green(),
        success_suffix: style("=".to_string()).for_stderr().dim(),
        values_style: Style::new().for_stderr().green(),
        defaults_style: Style::new().for_stderr().dim(),
        error_prefix: style("[!!]".to_string()).for_stderr().red(),
        error_style: Style::new().for_stderr().red(),
        hint_style: Style::new().for_stderr().dim(),
        ..ColorfulTheme::default()
    }
}

fn banner() {
    eprintln!(
        "\n{}",
        style("  ┌─────────────────────────────┐").dim()
    );
    eprintln!(
        "{}",
        style("  │  jdx init                   │").dim()
    );
    eprintln!(
        "{}",
        style("  │  interactive setup wizard    │").dim()
    );
    eprintln!(
        "{}\n",
        style("  └─────────────────────────────┘").dim()
    );
}

fn section(label: &str) {
    let pad = 36usize.saturating_sub(label.len() + 3);
    eprintln!(
        "\n  {} {} {}",
        style("─").dim(),
        style(label).white().bold(),
        style("─".repeat(pad)).dim()
    );
}

fn info(msg: &str) {
    eprintln!("  {} {}", style("..").dim(), style(msg).dim());
}

fn ok(msg: &str) {
    eprintln!("  {} {}", style("[ok]").green(), msg);
}

fn warn(msg: &str) {
    eprintln!("  {} {}", style("[!!]").yellow(), msg);
}

fn fail(msg: &str) {
    eprintln!("  {} {}", style("[!!]").red(), msg);
}

pub fn run_wizard() -> Result<()> {
    let t = theme();
    banner();

    info(&format!("platform   {}", std::env::consts::OS));
    info(&format!("arch       {}", std::env::consts::ARCH));

    let cfg_path = config::config_file_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.config/jdx/config.toml".into());
    info(&format!("config     {cfg_path}"));

    let mut config = config::load_config();

    // ── provider selection ─────────────────────────
    section("ai provider");

    let providers = &["ollama", "openai", "anthropic", "none"];
    let default_idx = providers
        .iter()
        .position(|&p| p == config.ai.provider)
        .unwrap_or(0);

    let provider_idx = Select::with_theme(&t)
        .with_prompt("provider")
        .items(providers)
        .default(default_idx)
        .interact()?;

    let provider = providers[provider_idx];

    config.ai = match provider {
        "ollama" => configure_ollama(&t, &config.ai)?,
        "openai" => configure_cloud(&t, "openai", "gpt-4o-mini", &config.ai)?,
        "anthropic" => {
            configure_cloud(&t, "anthropic", "claude-sonnet-4-5-20250929", &config.ai)?
        }
        _ => {
            info("ai disabled");
            AiConfig {
                provider: "none".into(),
                ..AiConfig::default()
            }
        }
    };

    // ── save ───────────────────────────────────────
    section("write config");

    config::save_config(&config)?;
    ok(&format!("saved {cfg_path}"));

    // preview
    eprintln!();
    let toml_str = toml::to_string_pretty(&config)?;
    for line in toml_str.lines() {
        eprintln!("  {}", style(line).dim());
    }

    eprintln!(
        "\n  {}",
        style("done. run `jdx <file>` to start exploring.").green()
    );
    eprintln!();

    Ok(())
}

fn configure_ollama(t: &ColorfulTheme, existing: &AiConfig) -> Result<AiConfig> {
    section("ollama");

    // check binary
    info("checking ollama installation...");
    let installed = Command::new("ollama")
        .arg("--version")
        .output()
        .map(|o| {
            if o.status.success() {
                let ver = String::from_utf8_lossy(&o.stdout);
                let ver = ver.trim();
                if ver.is_empty() {
                    None
                } else {
                    Some(ver.to_string())
                }
            } else {
                None
            }
        })
        .unwrap_or(None);

    match &installed {
        Some(ver) => ok(&format!("found {ver}")),
        None => {
            warn("ollama not found in PATH");
            offer_ollama_install(t)?;
        }
    }

    // model
    section("model");

    let default_model = if existing.model.is_empty() {
        "llama3.2".to_string()
    } else {
        existing.model.clone()
    };

    let model: String = Input::with_theme(t)
        .with_prompt("model")
        .default(default_model)
        .interact_text()?;

    let pull = Select::with_theme(t)
        .with_prompt(format!("pull {model} now?"))
        .items(&["yes", "skip"])
        .default(0)
        .interact()?;

    if pull == 0 {
        if ensure_ollama_serving() {
            info(&format!("$ ollama pull {model}"));
            let status = Command::new("ollama").args(["pull", &model]).status();
            match status {
                Ok(s) if s.success() => ok("model ready"),
                Ok(s) => fail(&format!("ollama pull exited with {s}")),
                Err(e) => fail(&format!("failed to exec ollama: {e}")),
            }
        } else {
            fail("server not reachable — skipping pull");
            info("start the server manually, then run: ollama pull {model}");
        }
    }

    // endpoint
    section("endpoint");

    let endpoint: String = Input::with_theme(t)
        .with_prompt("custom endpoint (enter to skip)")
        .default(existing.endpoint.clone())
        .allow_empty(true)
        .interact_text()?;

    Ok(AiConfig {
        provider: "ollama".into(),
        model,
        api_key: String::new(),
        endpoint,
    })
}

/// Check if ollama server is reachable; if not, try to start it.
/// Returns true if the server is up and ready for commands.
fn ensure_ollama_serving() -> bool {
    let addr: std::net::SocketAddr = "127.0.0.1:11434".parse().unwrap();

    if TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok() {
        return true;
    }

    // Try platform-appropriate start methods
    if cfg!(target_os = "macos") {
        info("$ brew services start ollama");
        let _ = Command::new("brew")
            .args(["services", "start", "ollama"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    // Also try `ollama serve` as a fallback (works on Linux, and macOS
    // without Homebrew services).  Spawned in the background so it
    // outlives the wizard.
    if TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_err() {
        info("$ ollama serve &");
        let _ = Command::new("ollama")
            .arg("serve")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    // Poll until the port is accepting connections (up to ~10s)
    for i in 0..40 {
        thread::sleep(Duration::from_millis(250));
        if TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok() {
            ok("ollama server ready");
            return true;
        }
        if i == 19 {
            info("still waiting for server...");
        }
    }

    warn("ollama server did not start in time");
    false
}

fn offer_ollama_install(t: &ColorfulTheme) -> Result<()> {
    let options = if cfg!(target_os = "macos") {
        vec!["brew install ollama", "skip"]
    } else if cfg!(target_os = "linux") {
        vec!["curl -fsSL https://ollama.com/install.sh | sh", "skip"]
    } else {
        info("install from https://ollama.com/download");
        return Ok(());
    };

    let choice = Select::with_theme(t)
        .with_prompt("install ollama?")
        .items(&options)
        .default(0)
        .interact()?;

    if choice == 0 {
        let status = if cfg!(target_os = "macos") {
            info("$ brew install ollama");
            Command::new("brew").args(["install", "ollama"]).status()
        } else {
            info("$ curl -fsSL https://ollama.com/install.sh | sh");
            Command::new("sh")
                .args(["-c", "curl -fsSL https://ollama.com/install.sh | sh"])
                .status()
        };

        match status {
            Ok(s) if s.success() => ok("ollama installed"),
            Ok(s) => fail(&format!("installer exited with {s}")),
            Err(e) => fail(&format!("failed to exec installer: {e}")),
        }
    }

    Ok(())
}

fn configure_cloud(
    t: &ColorfulTheme,
    provider: &str,
    default_model: &str,
    existing: &AiConfig,
) -> Result<AiConfig> {
    section(provider);

    let model_default = if existing.provider == provider && !existing.model.is_empty() {
        existing.model.clone()
    } else {
        default_model.to_string()
    };

    let model: String = Input::with_theme(t)
        .with_prompt("model")
        .default(model_default)
        .interact_text()?;

    let api_key: String = Password::with_theme(t)
        .with_prompt("api key")
        .allow_empty_password(true)
        .interact()?;

    let endpoint: String = Input::with_theme(t)
        .with_prompt("custom endpoint (enter to skip)")
        .default(existing.endpoint.clone())
        .allow_empty(true)
        .interact_text()?;

    Ok(AiConfig {
        provider: provider.into(),
        model,
        api_key,
        endpoint,
    })
}
