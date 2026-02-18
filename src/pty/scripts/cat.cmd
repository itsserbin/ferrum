@echo off
setlocal enabledelayedexpansion

if "%~1"=="" (
    echo cat: missing file operand
    exit /b 1
)

:loop
if "%~1"=="" goto end
if "%~1"=="-n" (
    shift
    goto loop
)
if not exist "%~1" (
    echo cat: %~1: No such file or directory
    exit /b 1
)
type "%~1"
shift
goto loop

:end
