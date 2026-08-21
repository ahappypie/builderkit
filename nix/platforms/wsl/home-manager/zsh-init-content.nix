# Add user-local tools to PATH for new WSL shell sessions.
initContent = lib.mkBefore ''
  export PATH="$HOME/.local/bin:$PATH"
'';
