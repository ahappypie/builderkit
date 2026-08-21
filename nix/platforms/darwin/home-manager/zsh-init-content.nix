# Add user-local tools and Homebrew to PATH for new Darwin shell sessions.
initContent = lib.mkBefore ''
  export PATH="$HOME/.local/bin:$PATH"
  eval "$(/opt/homebrew/bin/brew shellenv)"
'';
