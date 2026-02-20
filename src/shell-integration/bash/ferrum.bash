# Ferrum terminal shell integration for bash.
# Sends OSC 7 with the current working directory on every prompt.

# Guard: only run inside Ferrum.
[[ -n "$FERRUM_SHELL_INTEGRATION" ]] || return

_ferrum_last_reported_cwd=""

_ferrum_report_cwd() {
  if [[ "$_ferrum_last_reported_cwd" != "$PWD" ]]; then
    _ferrum_last_reported_cwd="$PWD"
    builtin printf '\e]7;file://%s%s\a' "$HOSTNAME" "$PWD"
  fi
}

PROMPT_COMMAND="_ferrum_report_cwd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
