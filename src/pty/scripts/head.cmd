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
if "%~1"=="-c" (
    shift
    shift
    goto parse_args
)
set "file=%~1"
goto execute

:error_no_file
echo head: missing file operand
exit /b 1

:execute
if not exist "%file%" (
    echo head: cannot open '%file%': No such file or directory
    exit /b 1
)

set "count=0"
for /f "delims=" %%i in (%file%) do (
    if !count! lss %lines% (
        echo %%i
        set /a count+=1
    )
)
