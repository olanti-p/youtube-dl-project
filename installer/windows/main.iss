// Designed for Inno Setup version 6.2.2

[Setup]
AppID=youtube-dl-server
AppName=Youtube-DL Server
AppVersion=0.1
WizardStyle=modern
DefaultDirName={autopf}\Youtube-DL Server
DefaultGroupName=Youtube-DL Server
Compression=lzma2
SolidCompression=yes
SourceDir=..\..
OutputDir=target\innosetup
OutputBaseFilename=youtube-dl-server-setup
LicenseFile=LICENSE.txt
PrivilegesRequired=admin

// Can be disabled for debug purposes
Output=yes

// Installing as service requires setting up a virtual user, skip it for now
// [UninstallRun]
// Filename: "{app}\youtube-dl-server.exe"; Parameters: "uninstall-service"; RunOnceId: "DelService"

[Files]
Source: target\release\youtube-dl-server.exe; DestDir: "{app}"
Source: LICENSE.txt; DestDir: "{app}"
Source: yt-dlp.exe; DestDir: "{app}"
Source: AtomicParsley.exe; DestDir: "{app}"
Source: phantomjs.exe; DestDir: "{app}"
Source: Rocket.toml; DestDir: "{app}"
Source: get_token.bat; DestDir: "{app}"
Source: webui\*; DestDir: "{app}\webui"
Source: webui\img\*; DestDir: "{app}\webui\img"

[Icons]
// Run on startup for all users
Name: "{commonstartup}\Youtube-DL Server"; Filename: "{app}\youtube-dl-server.exe"; Parameters: "run"
// Add to start menu
Name: "{commonstartmenu}\Youtube-DL Server\Run Server"; Filename: "{app}\youtube-dl-server.exe"; Parameters: "run"
Name: "{commonstartmenu}\Youtube-DL Server\Get API token"; Filename: "{app}\get_token.bat"
Name: "{commonstartmenu}\Youtube-DL Server\Uninstall"; Filename: "{app}\unins000.exe"

// Installing as service requires setting up a virtual user, skip it for now
// [Run]
// Filename: "{app}\youtube-dl-server.exe"; Parameters: "install-service --verbose"
