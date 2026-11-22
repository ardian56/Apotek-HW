@echo off
REM ============================================================
REM Script Pasang MariaDB sebagai Windows Service di PC Gudang
REM Jalankan sebagai Administrator!
REM ============================================================

set SCRIPT_DIR=%~dp0
set BASE_DIR=%SCRIPT_DIR%..
set MYSQLD=%BASE_DIR%\mariadb\bin\mysqld.exe
set MY_INI=%SCRIPT_DIR%my.ini

if not exist "%MYSQLD%" (
    echo [ERROR] File mysqld.exe tidak ditemukan di: %MYSQLD%
    echo Pastikan folder 'mariadb' sudah diekstrak sejajar dengan folder config!
    pause
    exit /b 1
)

echo Menghentikan service lama jika ada...
net stop MariaDBApotek >nul 2>&1
"%MYSQLD%" --remove MariaDBApotek >nul 2>&1

echo Memasang Windows Service MariaDBApotek...
"%MYSQLD%" --install MariaDBApotek --defaults-file="%MY_INI%"

echo Menjalankan Service...
net start MariaDBApotek

echo ============================================================
echo Service MariaDBApotek Berhasil Terpasang dan Berjalan!
echo Port: 3306 (Listening pada semua interface LAN)
echo ============================================================
pause
