@echo off
setlocal enabledelayedexpansion

set "show_lines=1"
set "show_words="
set "show_chars="
set "show_bytes="
set "file="

:parse_args
if "%~1"=="" goto error_no_file
if "%~1"=="-l" set "show_lines=1" & set "show_words=" & set "show_chars=" & shift & goto parse_args
if "%~1"=="-w" set "show_words=1" & set "show_lines=" & set "show_chars=" & shift & goto parse_args
if "%~1"=="-c" set "show_bytes=1" & set "show_lines=" & set "show_words=" & shift & goto parse_args
if "%~1"=="-m" set "show_chars=1" & set "show_lines=" & set "show_words=" & shift & goto parse_args
set "file=%~1"
goto execute

:error_no_file
echo wc: missing file operand
exit /b 1

:execute
if not exist "%file%" (
    echo wc: '%file%': No such file or directory
    exit /b 1
)

set "lines=0"
for /f %%i in ('type "%file%" ^| find /c /v ""') do set "lines=%%i"

if defined show_lines (
    echo %lines% %file%
    exit /b 0
)

if defined show_words (
    for /f %%i in ('type "%file%" ^| find /c " "') do echo %%i %file%
    exit /b 0
)

if defined show_bytes (
    for %%i in ("%file%") do echo %%~zi %file%
    exit /b 0
)

echo %lines% %file%
