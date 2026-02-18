@echo off
setlocal enabledelayedexpansion

set "ignore_case="
set "pattern="
set "files="

:parse_args
if "%~1"=="" goto error_no_pattern
if "%~1"=="-i" set "ignore_case=/i" & shift & goto parse_args
if "%~1"=="-n" shift & goto parse_args
if "%~1"=="--color" shift & goto parse_args
if "%~1"=="--color=auto" shift & goto parse_args
if not defined pattern set "pattern=%~1" & shift & goto parse_args
set "files=%files% %~1"
shift
goto parse_args

:error_no_pattern
echo grep: missing pattern
exit /b 1

:execute
if "%files%"=="" (
    findstr %ignore_case% /r "%pattern%" 2>nul
) else (
    findstr %ignore_case% /r "%pattern%" %files% 2>nul
)
