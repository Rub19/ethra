@echo off
set SUB=%1
shift
set REST=
:loop
if "%~1"=="" goto run
set REST=%REST% %1
shift
goto loop
:run
start "" /low /b /wait cargo %SUB% -j 3 %REST%
exit /b %errorlevel%
