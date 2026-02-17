# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**jdx** — an interactive terminal-based JSON viewer/explorer written in Rust. Supports JSON, YAML, TOML, CSV, and NDJSON formats. Features a dot-notation query language with filters/transforms, tree navigation, AI-powered natural language queries, schema inference, and fuzzy autocomplete.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo test --test query_tests  # Run a specific test file
cargo test query               # Run tests matching a pattern
cargo fmt --all                # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint (warnings are errors in CI)
```

CI runs fmt check, clippy, and tests on Ubuntu/macOS/Windows.

## Architecture

### Entry Point & App Loop

`main.rs` parses CLI args (clap), reads input from file or stdin, auto-detects format, and parses everything into `serde_json::Value`. In non-interactive mode, it evaluates the query and exits. In interactive mode, it initializes `App` and enters the ratatui event loop.

The core loop in `app.rs`: **Event → key mapping (`keys.rs`) → Action → mode-specific handler → render widgets**.

### Modes (`modes.rs`)

The app has distinct modes: **Query** (dot-notation with autocomplete), **Tree** (collapsible navigation), **AI** (natural language via API), **Schema** (type inference view), and **Help**.

### Engine (`src/engine/`)

- **query.rs** — Parses dot-notation queries (`.users[0].name`, `[price < 10]`, `[0:5]`) into a `PathSegment` enum
- **json.rs** — Recursive JSON tree traversal following path segments, predicate evaluation
- **transform.rs** — Chained data transforms (`:pick`, `:sort`, `:filter`, `:count`, `:keys`, `:values`, `:flatten`, `:uniq`, `:group_by`, `:sum`, `:avg`, `:min`, `:max`)
- **schema.rs** — Infers type schema from JSON data
- **suggestion.rs** — Fuzzy key completion at current query depth

### Format Support (`src/format/`)

Auto-detection and parsing/serialization for JSON, YAML, TOML, CSV. Each format has its own module.

### AI Integration (`src/ai/`)

Trait-based provider system (`AiProvider` trait) with OpenAI and Ollama implementations. AI queries run on a background tokio task, results return via `mpsc` channel, polled every 50ms in the event loop.

### Widgets (`src/widgets/`)

Ratatui `Widget` implementations: `json_view` (syntax-highlighted display), `query_input`, `candidate_popup` (autocomplete), `tree_view`, `ai_panel`, `status_bar`, `help_overlay`.

### Init Wizard (`src/init.rs`)

`jdx init` subcommand — interactive setup wizard that configures AI providers (Ollama, OpenAI, Anthropic) and saves to `config.toml`. Offers to install Ollama if not present.

### Configuration

`~/.config/jdx/config.toml` — AI provider settings, display options. Parsed in `config.rs`.

## Testing

Test fixtures live in `fixtures/`. Snapshot tests use the `insta` crate. HTTP mocking for AI tests uses `wiremock`.
