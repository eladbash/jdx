# jdx — JSON Data eXplorer

An interactive, AI-augmented terminal tool for exploring JSON data. Think `jq` meets a code editor — with fuzzy search, tree navigation, schema inspection, and natural language querying.

[![CI](https://github.com/eladbash/jdx/actions/workflows/ci.yml/badge.svg)](https://github.com/eladbash/jdx/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/jdx.svg)](https://crates.io/crates/jdx)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

![jdx demo](demo.gif)

---

## Install

### From crates.io

```bash
cargo install jdx
```

### From Homebrew (macOS / Linux)

```bash
brew install eladbash/jdx/jdx
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/eladbash/jdx/releases) — available for Linux, macOS, and Windows (x86_64 and aarch64).

### Build from Source

```bash
git clone https://github.com/eladbash/jdx.git
cd jdx
cargo build --release
# Binary at: target/release/jdx
```

---

## Quick Start

```bash
# Pipe JSON from stdin
echo '{"name": "Alice", "age": 30}' | jdx

# Explore an API response
curl -s https://api.github.com/repos/rust-lang/rust | jdx

# Open a file directly
jdx data.json

# YAML, TOML, CSV are auto-detected
cat config.yaml | jdx
```

---

## Features

### Interactive Dot-Notation Query

Type a path like `.users[0].name` to drill into JSON data in real-time. Every keystroke updates the filtered view instantly.

### Fuzzy Tab-Completion

Press **Tab** to see available keys at the current depth. Fuzzy matching ranks candidates so you can type `"us"` and match `"users"`. Ghost text shows the most likely completion inline (like fish shell).

### Collapsible Tree View

Press **Ctrl+\\** for a split panel with a tree navigator on the left. Expand and collapse nodes with arrow keys. The query input and tree stay synchronized.

### Schema Inspector

Press **S** to toggle schema view. Infers the shape of your JSON data — types, optional fields, value ranges, and array lengths — perfect for exploring unfamiliar API responses.

```
{
  users: [object]  # array of 3
  count: number  # 3
  next_page: string | null
}
```

### Filter Predicates

Filter arrays inline using bracket predicates:

```
.store.books[price < 10]              # books cheaper than $10
.users[role == "admin"]               # admin users only
.users[age >= 30]                     # users 30 and older
.items[status != "deleted"]           # exclude deleted items
.users[active == true]                # only active users
.items[deleted == null]               # items without a deleted field
```

Supported operators: `==`, `!=`, `<`, `>`, `<=`, `>=`

Values can be numbers (`10`, `3.5`), quoted strings (`"admin"`), booleans (`true`/`false`), or `null`.

Filters can be combined with path navigation and transforms:

```
.store.books[price < 10].title        # titles of cheap books
.store.books[price < 15] :pick title,price :sort price
.users[role == "admin"] :count        # count admin users
```

### Inline Data Transforms

Chain transforms after your query using `:` commands:

| Command | Description | Example |
|---------|-------------|---------|
| `:keys` | Object keys as array | `.store :keys` |
| `:values` | Object values as array | `.store :values` |
| `:count` | Count elements | `.users :count` |
| `:flatten` | Flatten nested arrays | `.tags :flatten` |
| `:pick` | Select fields | `.users :pick name,email` |
| `:omit` | Exclude fields | `.users :omit metadata` |
| `:sort` | Sort by field | `.users :sort age` |
| `:uniq` | Deduplicate | `.tags :uniq` |
| `:group_by` | Group by field | `.users :group_by role` |
| `:filter` | Filter by predicate | `.users :filter age > 30` |
| `:sum` | Sum numeric values | `.orders :sum total` |
| `:avg` | Average numeric values | `.scores :avg value` |
| `:min` | Minimum value | `.products :min price` |
| `:max` | Maximum value | `.products :max price` |

The `:filter` transform is equivalent to the bracket predicate syntax but works as a chained transform. Aggregate commands (`:sum`, `:avg`, `:min`, `:max`) work on arrays of numbers or on a specific field from objects:

```
.store.books :filter price < 10 :pick title,price
.users :pick name,age :filter age > 25 :sort age
.store.books :sum price                    # total of all book prices
.store.books :filter price < 15 :avg price # average of cheap books
.scores :min                               # minimum of a numeric array
```

### Natural Language AI Querying

Press **/** to switch to AI mode. Ask questions in plain English:

> "what is the total price of all books?"
> "which books cost less than $10?"
> "who are the admin users?"

The AI **answers your question directly** in natural language and optionally suggests a jdx query you can apply by pressing Enter. It sees the actual data, so it can compute totals, averages, find specific items, and more. Supports OpenAI, Anthropic, and local Ollama models.

### Multi-Format Input/Output

Auto-detects JSON, YAML, TOML, CSV, and NDJSON input. Output in any format with `--output`:

```bash
cat config.yaml | jdx                  # YAML auto-detected
cat data.csv | jdx --input csv         # CSV -> JSON
jdx data.json --output yaml            # Output as YAML
```

### Streaming NDJSON

```bash
tail -f logs.jsonl | jdx --input ndjson
```

### Clipboard Integration

- **Ctrl+Y** — Copy current value to clipboard
- **Ctrl+D** — Bookmark the current path

### Persistent History

Query history is saved across sessions. Press **Ctrl+R** to search through past queries.

---

## Keybindings

### Query Mode (default)

| Key | Action |
|-----|--------|
| **Tab** | Complete / cycle candidates |
| **Shift+Tab** | Cycle candidates backward |
| **Enter** | Confirm and output result |
| **Esc** / **Ctrl+C** | Quit without output |
| **Ctrl+L** | Toggle key-only mode |
| **Ctrl+U** | Clear query |
| **Ctrl+W** | Delete word backward |
| **Ctrl+A** / **Home** | Cursor to start |
| **Ctrl+E** / **End** | Cursor to end |
| **Ctrl+J** | Scroll down |
| **Ctrl+K** | Scroll up |
| **Ctrl+N** | Page down |
| **Ctrl+P** | Page up |
| **Ctrl+T** | Scroll to top |
| **Ctrl+G** | Scroll to bottom |
| **Ctrl+Y** | Copy current value to clipboard |
| **Ctrl+R** | Search query history |
| **Ctrl+D** | Bookmark current path |
| **Ctrl+\\** | Toggle split view (tree + JSON) |
| **S** | Toggle schema view |
| **/** | Switch to AI query mode |
| **?** | Show help overlay |

### Tree Mode

| Key | Action |
|-----|--------|
| **Up/Down** | Navigate tree nodes |
| **Right** / **Enter** | Expand node |
| **Left** | Collapse node |
| **Esc** / **q** | Back to query mode |

### AI Mode

| Key | Action |
|-----|--------|
| Type freely | Enter natural language question |
| **Enter** | Send question to AI / Apply suggested query |
| **Esc** | Back to query mode |

---

## Configuration

Configuration lives at `~/.config/jdx/config.toml`:

```toml
[ai]
provider = "ollama"       # "ollama", "openai", "anthropic", or "none"
model = "llama3.2"        # Model name
api_key = ""              # Required for cloud providers
endpoint = ""             # Custom API endpoint (optional)

[display]
monochrome = false        # Disable colors
max_candidates = 20       # Max items in autocomplete popup
schema_max_samples = 10   # Array elements to sample for schema inference
```

### AI Setup

**Local (Ollama):**

```bash
# Install Ollama: https://ollama.ai
ollama pull llama3.2
# jdx auto-connects to localhost:11434
```

**Cloud (OpenAI):**

```toml
[ai]
provider = "openai"
model = "gpt-4o-mini"
api_key = "sk-..."
```

---

## CLI Options

```
Usage: jdx [OPTIONS] [FILE]

Arguments:
  [FILE]  File to read JSON from (reads stdin if omitted)

Options:
  -Q, --query <QUERY>     Initial query (e.g., ".users[0]")
  -q, --query-output      Output the query string instead of the result
  -i, --input <FORMAT>    Input format: json, yaml, toml, csv, ndjson
  -o, --output <FORMAT>   Output format: json, yaml, toml, csv, ndjson
  -M, --monochrome        Disable colors
  -p, --pretty            Pretty-print output (default: true)
      --non-interactive   Evaluate query and print result without TUI
  -h, --help              Print help
  -V, --version           Print version
```

---

## Comparison

| Feature | jdx | jid | jq | fx | jless |
|---------|-----|-----|----|----|-------|
| Interactive TUI | Yes | Yes | No | Yes | Yes |
| Fuzzy completion | Yes | Prefix only | No | No | No |
| Tree navigation | Yes | No | No | No | Yes |
| Schema inspector | Yes | No | No | No | No |
| AI queries | Yes | No | No | No | No |
| Inline transforms | Yes | No | Yes (pipe) | Yes (JS) | No |
| Filter predicates | Yes | No | Yes | Yes (JS) | No |
| Multi-format input | Yes | No | No | Yes | No |
| Streaming NDJSON | Yes | No | Yes | Yes | No |
| Clipboard copy | Yes | No | No | No | No |
| Query history | Yes | No | No | No | No |
| Single binary | Yes | Yes | Yes | Yes | Yes |
| Written in | Rust | Go | C | Go | Rust |

---

## Contributing

```bash
# Clone and build
git clone https://github.com/eladbash/jdx.git
cd jdx
cargo build

# Run tests
cargo test

# Run with clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format code
cargo fmt --all
```

---

## License

MIT
