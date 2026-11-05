@echo off
tasklist | findstr /I /C:"javaw.exe" /C:"VALORANT-Win64" /C:"League of Legends.exe" >nul && (echo JEU OUVERT, compilation annulee & exit /b 99)
start "" /low /b /wait cargo %* -j 3
exit /b %errorlevel%
