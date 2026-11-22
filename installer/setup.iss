; ============================================================
; INNO SETUP SCRIPT: POS APOTEK (ULTRA COMPACT INSTALLER)
; Kompresi LZMA2 ultra untuk target ukuran installer < 20 MB!
; ============================================================

[Setup]
AppName=POS Apotek
AppVersion=1.0.0
DefaultDirName={autopf}\POS Apotek
DefaultGroupName=POS Apotek
UninstallDisplayIcon={app}\kasir_apotek.exe
Compression=lzma2/ultra64
SolidCompression=yes
OutputDir=dist
OutputBaseFilename=Setup_Apotek_1Klik
ArchitecturesAllowed=x86 x64
ArchitecturesInstallIn64BitMode=x64

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
; 1. Binary Utama Aplikasi (Hanya ~4-6 MB)
Source: "target\i686-pc-windows-msvc\release\kasir_apotek.exe"; DestDir: "{app}"; Flags: ignoreversion

; 2. Konfigurasi Low-RAM
Source: "config\my.ini"; DestDir: "{app}\config"; Flags: ignoreversion

; 3. Engine Database MariaDB Minimalis (Opsional untuk Server)
; Hanya mysqld.exe + share/english (dihapus plugin berat)
Source: "mariadb_minimal\*"; DestDir: "{app}\mariadb"; Flags: ignoreversion recursesubdirs createallsubdirs; Tasks: ; Check: IsServerSelected

[Icons]
Name: "{autodesktop}\POS Apotek"; Filename: "{app}\kasir_apotek.exe"; Tasks: desktopicon
Name: "{group}\POS Apotek"; Filename: "{app}\kasir_apotek.exe"
Name: "{group}\Uninstall POS Apotek"; Filename: "{uninstallexe}"

[Run]
; Auto-register MariaDB sebagai Windows Service jika mode server dipilih (silent di background)
Filename: "{app}\mariadb\bin\mysqld.exe"; Parameters: "--install MariaDBApotek --defaults-file=""{app}\config\my.ini"""; Flags: runhidden waituntilterminated; Check: IsServerSelected
Filename: "net.exe"; Parameters: "start MariaDBApotek"; Flags: runhidden waituntilterminated; Check: IsServerSelected

; Jalankan aplikasi langsung setelah install selesai
Filename: "{app}\kasir_apotek.exe"; Description: "{cm:LaunchProgram,POS Apotek}"; Flags: nowait postinstall skipifsilent

[Code]
var
  ServerPage: TInputOptionWizardPage;

procedure InitializeWizard;
begin
  ServerPage := CreateInputOptionPage(wpSelectDir,
    'Pilihan Peran Komputer',
    'Tentukan fungsi komputer ini dalam jaringan apotek:',
    'Silakan pilih satu opsi di bawah ini:',
    True, False);

  ServerPage.Add('PC KASIR (Client) - Hanya untuk kasir, hemat ruang.');
  ServerPage.Add('PC GUDANG (Server Utama) - Memasang database server MariaDB.');
  ServerPage.Values[0] := True; // Default Kasir
end;

function IsServerSelected: Boolean;
begin
  Result := ServerPage.Values[1];
end;
