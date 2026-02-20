@echo off
setlocal enabledelayedexpansion

set "opts="
set "show_hidden="
set "path_arg=."

:parse_args
if "%~1"=="" goto execute
if "%~1"=="-a" set "show_hidden=1" & shift & goto parse_args
if "%~1"=="-A" set "show_hidden=1" & shift & goto parse_args
if "%~1"=="-l" shift & goto parse_args
if "%~1"=="-la" set "show_hidden=1" & shift & goto parse_args
if "%~1"=="-al" set "show_hidden=1" & shift & goto parse_args
if "%~1"=="-ll" shift & goto parse_args
if "%~1"=="-lh" shift & goto parse_args
if "%~1"=="-h" shift & goto parse_args
if "%~1"=="--color" shift & goto parse_args
if "%~1"=="--color=auto" shift & goto parse_args
if "%~1"=="--color=always" shift & goto parse_args
if not "%~1"=="-*" set "path_arg=%~1" & shift & goto parse_args
shift
goto parse_args

:execute
if defined show_hidden (
    dir /a "%path_arg%"
) else (
    dir "%path_arg%"
)
