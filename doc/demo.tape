# klik typing TUI demo
# Record with: vhs doc/demo.tape
# Requires: klik on PATH, vhs installed (brew install vhs)
#
# Uses -p for deterministic prompts so the script can type exact characters.

Output doc/demo.gif
Set Width 900
Set Height 500
Set FontSize 16
Set Shell zsh
Set Theme "Catppuccin Mocha"

# --- Typing session ---
Type "klik -p 'the quick brown fox jumps over the lazy dog'"
Enter
Sleep 1s

Set TypingSpeed 80ms
Type "the quick brown fox jumps over the lazy dog"

# Results screen
Sleep 3s

# Show character stats
Type "s"
Sleep 3s

# Back to results
Type "b"
Sleep 1s

# Exit
Escape
Sleep 500ms
