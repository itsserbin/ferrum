@echo off
setlocal enabledelayedexpansion

set "lines=10"
set "file="

:parse_args
if "%~1"=="" goto error_no_file
if "%~1"=="-n" (
    set "lines=%~2"
    shift
    shift
    goto parse_args
)
if "%~1"=="-f" (
    echo tail: -f option not supported on Windows
    exit /b 1
)
set "file=%~1"
goto execute

:error_no_file
echo tail: missing file operand
exit /b 1

:execute
if not exist "%file%" (
    echo tail: cannot open '%file%': No such file or directory
    exit /b 1
)

powershell -NoProfile -Command "Get-Content '%file%' -Tail %lines%"
