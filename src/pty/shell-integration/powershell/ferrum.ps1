if ($env:FERRUM_SHELL_INTEGRATION -ne "1") { return }

function prompt {
    # Emit OSC 7 with current directory (forward slashes for URI)
    $path = $PWD.Path -replace '\\', '/'
    $host_name = [System.Net.Dns]::GetHostName()
    [Console]::Write("`e]7;file://$host_name/$path`e\")
    # Standard PS prompt
    "PS $($PWD.Path)> "
}
