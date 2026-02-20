@echo off
setlocal enabledelayedexpansion

set "opts="

:parse_args
if "%~1"=="" goto execute
if "%~1"=="-e" goto execute
if "%~1"=="-f" goto execute
if "%~1"=="aux" goto execute
shift
goto parse_args

:execute
tasklist
