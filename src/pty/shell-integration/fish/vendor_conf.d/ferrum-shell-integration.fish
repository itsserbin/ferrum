# Ferrum terminal shell integration for fish.
# Sends OSC 7 with the current working directory on every directory change.

# Guard: only run inside Ferrum.
if not set -q FERRUM_SHELL_INTEGRATION
    exit
end

function __ferrum_report_cwd --on-variable PWD
    printf '\e]7;file://%s%s\a' (hostname) (string escape --style=url -- $PWD)
end

# Report initial CWD.
__ferrum_report_cwd
