# Add user-local tools and Homebrew to PATH for new Darwin shell sessions.
initContent = lib.mkBefore ''
  export PATH="$HOME/.local/bin:$PATH"

  if [ -x /opt/homebrew/bin/brew ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
  fi
'';
