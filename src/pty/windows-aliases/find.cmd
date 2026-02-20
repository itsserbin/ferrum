@echo off
setlocal enabledelayedexpansion

set "path_arg=."
set "name_pattern="
set "type_arg="

:parse_args
if "%~1"=="" goto execute
if "%~1"=="-name" (
    set "name_pattern=%~2"
    shift
    shift
    goto parse_args
)
if "%~1"=="-type" (
    set "type_arg=%~2"
    shift
    shift
    goto parse_args
)
if not "%~1"=="-*" (
    set "path_arg=%~1"
    shift
    goto parse_args
)
shift
goto parse_args

:execute
if defined name_pattern (
    if defined type_arg (
        if "%type_arg%"=="d" (
            dir /b /s /ad "%path_arg%\%name_pattern%" 2>nul
        ) else (
            dir /b /s /a-d "%path_arg%\%name_pattern%" 2>nul
        )
    ) else (
        dir /b /s "%path_arg%\%name_pattern%" 2>nul
    )
) else (
    dir /b /s "%path_arg%" 2>nul
)
