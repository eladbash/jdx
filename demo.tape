# jdx demo recording
# Generate with: vhs demo.tape

Output demo.gif
Set Shell "zsh"
Set Width 1200
Set Height 700
Set FontSize 15
Set Padding 20
Set Theme "Catppuccin Mocha"
Set TypingSpeed 60ms


# ─── Part 1: Non-interactive mode ───────────────────────────────────────────

Sleep 500ms
Type "# Part 1: Non-interactive CLI — convert nested JSON to CSV"
Enter
Sleep 1s

Type "jdx fixtures/nested.json --non-interactive -Q '.store.books' -o csv"
Sleep 500ms
Enter
Sleep 2s

# ─── Part 2: Interactive — Filtering, sum, avg ─────────────────────────────

Type "# Part 2: Interactive mode — filters, sum, avg"
Enter
Sleep 1s

Type "jdx fixtures/nested.json"
Sleep 500ms
Enter
Sleep 2s

# Filter books cheaper than $15
Type ".store.books[price < 15]"
Sleep 5s

# Clear and show :pick + :sort
Ctrl+U
Sleep 500ms
Type ".store.books :pick title,price :sort price"
Sleep 5s

# Clear and show :sum
Ctrl+U
Sleep 500ms
Type ".store.books :sum price"
Sleep 2s

# Clear and show :avg
Ctrl+U
Sleep 500ms
Type ".store.books :avg price"
Sleep 2s

# Clear and show :count
Ctrl+U
Sleep 500ms
Type ".users :count"
Sleep 2s

# Clear and show :filter + :min
Ctrl+U
Sleep 500ms
Type ".store.books :filter price < 20 :min price"
Sleep 2s

# Exit interactive mode
Escape
Sleep 500ms

# ─── Part 3: AI mode ───────────────────────────────────────────────────────

Type "# Part 3: AI-powered natural language queries"
Enter
Sleep 1s

Type "jdx fixtures/nested.json"
Sleep 500ms
Enter
Sleep 2s

# Enter AI mode with /
Type "/"
Sleep 1s

# Type a natural language question
Type "what is the total price of all books?"
Sleep 1s
Enter
Sleep 15s

# Exit
Escape
Sleep 1s
