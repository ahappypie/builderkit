Output raw-demo.mp4

Set Shell "zsh"
Set FontFamily "JetBrainsMono Nerd Font"
Set FontSize 20
Set Width 1200
Set Height 680
Set Padding 28
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set TypingSpeed 65ms

Hide
Type "clear"
Enter
Show

# Download + Install kit
Type "cd ~/Downloads"
Enter
Sleep 1.75s
Type "mv kit-darwin-aarch64 kit"
Enter
Sleep 1.75s
Type "chmod +x ./kit"
Enter
Sleep 1.75s
Type "./kit --version"
Enter
Sleep 1.75s

Hide
Type "export PS1='$ '"
Enter
Type "export OPENAI_API_KEY=sk-demo"
Enter
Type "clear"
Enter
Show

Sleep 750ms

# 1. Before: clearly missing
Type "codex --version"
Enter
Sleep 1.75s

Hide
Type "clear"
Enter
Show

# 2. Run real kit setup (note the timestamp when it starts & finishes)
Type "./kit setup --version 2026.11.0"
Enter
Sleep 1.5s
Type "y"
Sleep 1s
Enter
Sleep 1s
Type "y"
Enter
Sleep 1s
Type "Easton Student"
Enter
Sleep 1s
Type "easton@email.com"
Enter
Sleep 1s
Type "center"
Enter
Sleep 1s
Type "y"
Enter
Sleep 300s       # Adjust to cover your full install; trim the excess in Step B

# Get new shell
Hide
Type "exec zsh -l"
Enter
Sleep 1s
Type "export OPENAI_API_KEY=sk-demo"
Enter
Type "clear"
Enter
Show

# 3. After: clearly installed
Type "codex --version"
Enter
Sleep 1.75s

# 4. Launch agent and open Home Manager skills list
Type "codex"
Sleep 60ms
Enter
Sleep 1.75s
Type "3"
Sleep 1.75s
Enter
Sleep 600ms
Type "$systems-design"
Sleep 1.25s
Enter
Sleep 600ms
Type "let's build a prototype"
Sleep 3s