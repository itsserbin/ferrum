@echo off
setlocal enabledelayedexpansion

set "recursive="
set "force=1"

:parse_args
if "%~1"=="" goto error_no_args
if "%~1"=="-r" set "recursive=1" & shift & goto parse_args
if "%~1"=="-f" set "force=1" & shift & goto parse_args
if "%~1"=="-rf" set "recursive=1" & set "force=1" & shift & goto parse_args
if "%~1"=="-fr" set "recursive=1" & set "force=1" & shift & goto parse_args
if "%~1"=="-R" set "recursive=1" & shift & goto parse_args
goto execute

:error_no_args
echo rm: missing operand
exit /b 1

:execute
if not exist "%~1" (
    if not defined force (
        echo rm: cannot remove '%~1': No such file or directory
        exit /b 1
    )
    exit /b 0
)

if exist "%~1\*" (
    if not defined recursive (
        echo rm: cannot remove '%~1': Is a directory
        exit /b 1
    )
    rmdir /s /q "%~1"
) else (
    del /q "%~1"
)

if "%~2"=="" exit /b 0
shift
goto execute
