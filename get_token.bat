@echo off
.\youtube-dl-server.exe get-token | clip
echo API token has been copied to your clipboard.
echo Press any key to close this window.
pause
@echo on
