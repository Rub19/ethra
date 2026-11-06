@echo off
tasklist | findstr /I /C:"VALORANT-Win64" /C:"League of Legends.exe" >nul && (echo JEU OUVERT, compilation annulee & exit /b 99)
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
