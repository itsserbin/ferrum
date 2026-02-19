@echo off
setlocal enabledelayedexpansion

set "path_arg=."
set "human="
set "summary="

:parse_args
if "%~1"=="" goto execute
if "%~1"=="-h" set "human=1" & shift & goto parse_args
if "%~1"=="-s" set "summary=1" & shift & goto parse_args
if "%~1"=="-sh" set "human=1" & set "summary=1" & shift & goto parse_args
if not "%~1"=="-*" set "path_arg=%~1" & shift & goto parse_args
shift
goto parse_args

:execute
if defined summary (
    powershell -NoProfile -Command "$size = (Get-ChildItem -Path '%path_arg%' -Recurse -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum; if ($size -gt 1GB) { Write-Host ([math]::Round($size/1GB, 2))G } elseif ($size -gt 1MB) { Write-Host ([math]::Round($size/1MB, 2))M } elseif ($size -gt 1KB) { Write-Host ([math]::Round($size/1KB, 2))K } else { Write-Host $size }"
) else (
    dir /s "%path_arg%"
)
