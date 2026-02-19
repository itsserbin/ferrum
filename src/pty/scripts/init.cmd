@echo off
chcp 65001 >nul
REM Unix-style command aliases for Windows
REM Scripts directory is added to PATH by Ferrum

REM Simple doskey aliases (don't need wrappers)
doskey pwd=cd
doskey clear=cls
doskey cp=copy $*
doskey mv=move $*
doskey mkdir=mkdir $*
doskey rmdir=rmdir /s /q $*
doskey which=where $*
doskey kill=taskkill /F /PID $*
doskey env=set $*
doskey touch=type nul $g $* 2$gnul
