; OnionLauncherSetup.exe: the download. A normal per-user Inno Setup installer, like
; Onion Board's and Onion Watch's, so the app's own exe never installs itself (0.2.2's
; self-installing exe was called a trojan by Windows Defender's machine-learning guess).
;
;   - no admin needed: %LOCALAPPDATA%\Programs\Onion Launcher, the same folder as 0.2.2
;   - one page: what it is, a picture of the launcher box, the hotkey (Win + the key
;     left of 1, or the one picked before: click it and press new keys to change it),
;     and a "Start with Windows" box (ticked; an update keeps the last install's
;     choice). Then installing, then "All done" with "Open Onion Launcher now"
;   - a changed hotkey is saved by the installed exe (OnionLauncher.exe --set-hotkey),
;     so apps.json is only ever written by the app's own code
;   - Inno's plain "modern" look with no side panel or header logo: the launcher picture
;     is the only image. Setup's own icon is Inno's, and it starts with Inno's 64-bit
;     loader: see "Virus scanners" at [Setup]
;   - Start menu: "Onion Launcher" and "Uninstall Onion Launcher"; Settings > Apps entry
;   - a running copy is asked to close first (OnionLauncher.exe --quit), on install and
;     uninstall, so its exe can be replaced or removed
;   - updating over 0.2.2: its own Settings > Apps entry is removed, so there's one entry,
;     and its startup choice is kept
;   - uninstalling leaves the list of apps in %APPDATA%\OnionLauncher
;
; Built by scripts\build-release.ps1 (needs Inno Setup 6.7 or newer:
; winget install JRSoftware.InnoSetup), which builds the exe and draws the pictures first
; (scripts\make_installer_art.py: the launcher picture at each display scale). /DPREVIEW
; is a look at the pages only: it installs nothing (scripts\installer_pictures.ps1).

#define AppName "Onion Launcher"
#define AppExeName "OnionLauncher"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{606E0F0C-9394-47FC-BC66-EEBD5C3B4E8C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName}
; files keep their UTC time stamps: local ones would give away the build PC's time zone
TimeStampsInUTC=yes
AppPublisher=Onion Alien
AppPublisherURL=https://github.com/Onion-Alien/onion-launcher
AppSupportURL=https://github.com/Onion-Alien/onion-launcher/issues
AppUpdatesURL=https://github.com/Onion-Alien/onion-launcher/releases
AppCopyright=Copyright (C) Onion Launcher contributors
AppComments=Your apps, one key away
; a named, versioned setup file rather than a blank one (Properties > Details), which
; also helps machine-learning virus scanners that distrust those
VersionInfoVersion={#AppVersion}
VersionInfoProductName={#AppName}
VersionInfoProductVersion={#AppVersion}
VersionInfoCompany=Onion Alien
VersionInfoDescription={#AppName} Setup
VersionInfoCopyright=Copyright (C) Onion Launcher contributors
#ifdef PREVIEW
DefaultDirName={tmp}\preview
Uninstallable=no
#else
DefaultDirName={localappdata}\Programs\{#AppName}
#endif
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
DisableWelcomePage=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
; Virus scanners: a setup this small is called malware by a few machine-learning
; scanners on VirusTotal whatever is inside it (the app's own exe scans clean). Tested one
; change at a time, three things each cost a flag, so don't bring one back without
; scanning the result (scripts\vt_scan.py):
;   - Inno's 32-bit loader (Skyhigh, Cylance, DeepInstinct): UseSetupLdr=x64
;   - an icon of our own on the setup file (APEX), so no SetupIconFile: Setup keeps
;     Inno's icon. The installed app has its own icon as before
;   - the Windows 11 look, "dynamic windows11" (CrowdStrike, most builds): plain modern
UseSetupLdr=x64
WizardStyle=modern
WizardSizePercent=100
; no side panel or header logo (InitializeWizard adds the launcher picture instead)
WizardImageFile=
WizardSmallImageFile=
UninstallDisplayIcon={app}\{#AppExeName}.exe
UninstallDisplayName={#AppName}
OutputDir=..\target\installer
OutputBaseFilename=OnionLauncherSetup
; zip, not solid lzma: Microsoft's machine-learning scanner calls many solid-lzma
; installers Trojan:Win32/Wacatac.B!ml whatever is inside (Onion Board and Onion Watch
; found the same files zipped scan clean)
Compression=zip
SolidCompression=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes

[Messages]
SetupWindowTitle=Install %1
WizardSelectTasks=Install Onion Launcher
SelectTasksDesc=Your apps, one key away
SelectTasksLabel2=Press your hotkey and a box of your apps opens next to the mouse. Click one, or press its number.%n%nOnion Launcher waits in the tray, needs no admin rights and never goes online.
FinishedHeadingLabel=All done!
; (the hotkey is filled in by DoneCaption)
FinishedLabel=Onion Launcher is installed. To open your apps, press your hotkey or click its icon in the tray.%n%nTo start or uninstall it later, search the Start menu for Onion Launcher.
FinishedLabelNoIcons=Onion Launcher is installed. To open your apps, press your hotkey or click its icon in the tray.%n%nTo start or uninstall it later, search the Start menu for Onion Launcher.
ClickFinish=

[Tasks]
Name: "startup"; Description: "Start with Windows"

[Files]
; the launcher picture, for the pages only (never installed)
Source: "..\target\installer\box-light-*.png"; Flags: dontcopy
#ifndef PREVIEW
Source: "..\target\release\{#AppExeName}.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; Comment: "Your apps, one key away"
Name: "{autoprograms}\Uninstall {#AppName}"; Filename: "{uninstallexe}"; Comment: "Remove Onion Launcher from this PC"

[Registry]
; "Start with Windows": set when ticked, removed when unticked
; (on an update too) and on uninstall
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#AppExeName}"; ValueData: """{app}\{#AppExeName}.exe"""; Tasks: startup; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "{#AppExeName}"; Tasks: not startup; Flags: deletevalue dontcreatekey uninsdeletevalue
; 0.2.2's own Settings > Apps entry (its exe installed itself): this installer's
; replaces it
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppExeName}"; ValueType: none; Flags: deletekey dontcreatekey
#endif

[Run]
Filename: "{app}\{#AppExeName}.exe"; Description: "Open Onion Launcher now"; Flags: nowait postinstall skipifsilent

#ifndef PREVIEW
[UninstallRun]
Filename: "{app}\{#AppExeName}.exe"; Parameters: "--quit"; RunOnceId: "QuitApp"; Flags: runhidden waituntilterminated
#endif

[Code]
const
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';
  OldEntry = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppExeName}';

  // RegisterHotKey's modifier bits, as saved in apps.json
  MOD_ALT = 1;
  MOD_CONTROL = 2;
  MOD_SHIFT = 4;
  MOD_WIN = 8;
  TryHotkeyId = 31001;       // taken for a moment to see whether another app has it
  SwallowKey = $E8;          // the app's win::SWALLOW_KEY
  NoteGrey = $005F5F5F;
  NoteRed = $001C2BC4;       // Windows' own "danger" red

type
  TMsgRec = record
    Wnd: HWND;
    Message: Cardinal;
    WParam, LParam: Longint;
    Time: Cardinal;
    X, Y: Integer;
  end;

var
  StartupChecked: Boolean;
  TasksBox, DoneBox: TBitmapImage;
  DoneText: TNewStaticText;   // the last page's text, in place of Inno's FinishedLabel
  HotkeyCaption, HotkeyNote: TNewStaticText;
  HotkeyEdit: TNewEdit;
  // the hotkey in use now (apps.json's, else the usual one) and the one picked here
  OldMods, OldKey, PickMods, PickKey: Cardinal;
  RunningCopy: Boolean;       // an older Onion Launcher is running and holds OldMods/OldKey
  StoppedCopy: Boolean;       // ... and was closed when the hotkey box was clicked
  Installed: Boolean;

function RegisterHotKey(Wnd: HWND; Id: Integer; Mods, Vk: Cardinal): Integer;
  external 'RegisterHotKey@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
function UnregisterHotKey(Wnd: HWND; Id: Integer): Integer;
  external 'UnregisterHotKey@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
function GetKeyState(Vk: Integer): SmallInt;
  external 'GetKeyState@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
function MapVirtualKey(Code, MapType: Cardinal): Cardinal;
  external 'MapVirtualKeyW@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
function GetKeyNameText(LParam: Longint; Buf: String; Size: Integer): Integer;
  external 'GetKeyNameTextW@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
procedure KeybdEvent(Vk, Scan: Byte; Flags, ExtraInfo: Cardinal);
  external 'keybd_event@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)
function PeekMessage(var Msg: TMsgRec; Wnd: HWND; First, Last, Remove: Cardinal): Integer;
  external 'PeekMessageW@user32.dll stdcall';   // sensitive-scan: allow (not an e-mail address)

// ---- The hotkey, the same way the app names and checks it (src\hotkey.rs) ----

// Win + the key left of 1 on this keyboard (` on US layouts, ^ on German, ' on UK...)
function UsualKey: Cardinal;
begin
  Result := MapVirtualKey($29, 1);   // MAPVK_VSC_TO_VK
  if Result = 0 then Result := $C0;  // VK_OEM_3
end;

// The key's name on this keyboard: "A", "Space", "F5", "`"
function KeyName(Vk: Cardinal): String;
var
  Sc: Cardinal;
  LP: Longint;
  Buf: String;
  N: Integer;
begin
  Result := 'Key ' + IntToStr(Vk);
  Sc := MapVirtualKey(Vk, 0);   // MAPVK_VK_TO_VSC
  if Sc = 0 then exit;
  LP := (Sc and $FF) shl 16;
  // these share their scan code with the number pad: without this, Page Down is "Num 3"
  if ((Vk >= $21) and (Vk <= $28)) or (Vk = $2D) or (Vk = $2E) or (Vk = $6F) or (Vk = $90) then
    LP := LP or $1000000;
  Buf := StringOfChar(' ', 64);
  N := GetKeyNameText(LP, Buf, 64);
  if N > 0 then Result := Copy(Buf, 1, N);
end;

function ModNames(Mods: Cardinal): String;
begin
  Result := '';
  if Mods and MOD_WIN <> 0 then Result := Result + 'Win + ';
  if Mods and MOD_CONTROL <> 0 then Result := Result + 'Ctrl + ';
  if Mods and MOD_ALT <> 0 then Result := Result + 'Alt + ';
  if Mods and MOD_SHIFT <> 0 then Result := Result + 'Shift + ';
end;

// "Ctrl + Shift + Space"
function HotkeyLabel(Mods, Vk: Cardinal): String;
begin
  Result := ModNames(Mods) + KeyName(Vk);
end;

// The number after Name in S, looking from From on
function NumberAfter(const S, Name: String; From: Integer; var Value: Cardinal): Boolean;
var
  P, I: Integer;
  Digits: String;
begin
  Result := False;
  P := Pos(Name, Copy(S, From, Length(S)));
  if P = 0 then exit;
  I := From + P - 1 + Length(Name);
  while (I <= Length(S)) and (S[I] = ' ') do I := I + 1;
  Digits := '';
  while (I <= Length(S)) and (S[I] >= '0') and (S[I] <= '9') and (Length(Digits) < 9) do
  begin
    Digits := Digits + S[I];
    I := I + 1;
  end;
  if Digits = '' then exit;
  Value := StrToInt(Digits);
  Result := True;
end;

// The hotkey picked before (apps.json's "hotkey"), else the usual one. A "hotkey" inside
// an app's name is written \"hotkey\" in the file, so it can't be mistaken for this one.
procedure ReadOldHotkey;
var
  Raw: AnsiString;
  S: String;
  P: Integer;
  Mods, Vk: Cardinal;
begin
  OldMods := MOD_WIN;
  OldKey := UsualKey;
#ifdef PREVIEW
  // (the pictures show a new install's page, whatever the PC making them has saved)
  exit;
#endif
  if not LoadStringFromFile(ExpandConstant('{userappdata}\OnionLauncher\apps.json'), Raw) then exit;
  S := Raw;
  P := Pos('"hotkey"', S);
  if P = 0 then exit;
  if NumberAfter(S, '"mods":', P, Mods) and NumberAfter(S, '"key":', P, Vk) and
     (Mods and (MOD_CONTROL or MOD_ALT or MOD_WIN) <> 0) and (Mods <= 15) and
     (Vk >= 1) and (Vk <= $FE) then
  begin
    OldMods := Mods;
    OldKey := Vk;
  end;
end;

function HeldMods: Cardinal;
begin
  Result := 0;
  if (GetKeyState($5B) < 0) or (GetKeyState($5C) < 0) then Result := Result or MOD_WIN;
  if GetKeyState($11) < 0 then Result := Result or MOD_CONTROL;
  if GetKeyState($12) < 0 then Result := Result or MOD_ALT;
  if GetKeyState($10) < 0 then Result := Result or MOD_SHIFT;
end;

function IsModifier(Vk: Word): Boolean;
begin
  Result := (Vk = $10) or (Vk = $11) or (Vk = $12) or (Vk = $5B) or (Vk = $5C) or
    ((Vk >= $A0) and (Vk <= $A5));
end;

// False if another app already has it. The running Onion Launcher's own one counts as free.
function HotkeyFree(Mods, Vk: Cardinal): Boolean;
begin
  if RunningCopy and (Mods = OldMods) and (Vk = OldKey) then
  begin
    Result := True;
    exit;
  end;
  Result := RegisterHotKey(WizardForm.Handle, TryHotkeyId, Mods, Vk) <> 0;
  if Result then
    UnregisterHotKey(WizardForm.Handle, TryHotkeyId);
end;

procedure SetNote(const Text: String; Color: TColor);
begin
  HotkeyNote.Caption := Text;
  HotkeyNote.Font.Color := Color;
end;

// The box's text, with the caret after it
procedure ShowKeys(const Text: String);
begin
  HotkeyEdit.Text := Text;
  HotkeyEdit.SelStart := Length(Text);
end;

procedure ShowPick;
begin
  ShowKeys(HotkeyLabel(PickMods, PickKey));
end;

// A running Onion Launcher holds its hotkey, so pressing it here would open its box
// instead: close it now (Install would anyway), and start it again if setup is cancelled
procedure StopRunningCopy;
var
  Code: Integer;
  Exe: String;
begin
  if not RunningCopy then exit;
  Exe := ExpandConstant('{app}\{#AppExeName}.exe');
  if FileExists(Exe) then
    Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code);
  RunningCopy := CheckForMutexes('Local\OnionLauncher.single');
  StoppedCopy := not RunningCopy;
end;

procedure HotkeyEnter(Sender: TObject);
begin
  StopRunningCopy;
  SetNote('Press Ctrl, Alt or Win + a key', NoteGrey);
end;

procedure HotkeyExit(Sender: TObject);
begin
  ShowPick;
  SetNote('Click it and press new keys', NoteGrey);
end;

// A key pressed in the hotkey box: modifiers are shown as they're held; the next key
// makes the combination, which is kept if it's allowed and no other app has it
procedure HotkeyKeyDown(Sender: TObject; var Key: Word; Shift: TShiftState);
var
  Mods: Cardinal;
  M: TMsgRec;
begin
  // the do-nothing key pressed below while Windows is held
  if Key = SwallowKey then
  begin
    Key := 0;
    exit;
  end;
  Mods := HeldMods;
  if IsModifier(Key) then
  begin
    // let go on its own, Windows would open the Start menu
    if (Key = $5B) or (Key = $5C) then
    begin
      KeybdEvent(SwallowKey, 0, 0, 0);
      KeybdEvent(SwallowKey, 0, 2, 0);   // KEYEVENTF_KEYUP
    end;
    // a new try: an old "refused" goes away
    ShowKeys(ModNames(Mods) + '...');
    HotkeyEnter(nil);
    Key := 0;
    exit;
  end;
  if Mods and (MOD_CONTROL or MOD_ALT or MOD_WIN) = 0 then
  begin
    ShowPick;
    SetNote('Hold Ctrl, Alt or Win with it', NoteRed);
  end
  else if (Mods = MOD_ALT) and (Key = $73) then   // Alt + F4
  begin
    ShowPick;
    SetNote('That one closes windows', NoteRed);
  end
  else if not HotkeyFree(Mods, Key) then
  begin
    ShowKeys(HotkeyLabel(Mods, Key));
    SetNote('Another app already uses that one', NoteRed);
  end
  else
  begin
    PickMods := Mods;
    PickKey := Key;
    ShowPick;
    SetNote('Got it. Click it to pick another', NoteGrey);
  end;
  Key := 0;
  // the typed character (and Alt + key's menu character) is already on its way: drop it,
  // so Alt + Space doesn't open the window menu
  while PeekMessage(M, HotkeyEdit.Handle, $102, $102, 1) <> 0 do;   // WM_CHAR, PM_REMOVE
  while PeekMessage(M, HotkeyEdit.Handle, $106, $107, 1) <> 0 do;   // WM_SYSCHAR, WM_SYSDEADCHAR
end;

// Modifiers let go without a key: back to the hotkey as it is
procedure HotkeyKeyUp(Sender: TObject; var Key: Word; Shift: TShiftState);
var
  Mods: Cardinal;
begin
  if not IsModifier(Key) then exit;
  Mods := HeldMods;
  if Mods = 0 then
    ShowPick
  else
    ShowKeys(ModNames(Mods) + '...');
  Key := 0;
end;

function DoneCaption: String;
var
  Keys: String;
begin
  // kept on one line: "Ctrl +" at a line's end and "Shift + O" on the next reads badly
  Keys := HotkeyLabel(PickMods, PickKey);
  StringChangeEx(Keys, ' ', #$00A0, True);
  Result := 'Onion Launcher is installed. To open your apps, press ' +
    Keys + ' or click its icon in the tray. To change the hotkey, ' +
    'right-click that icon.' + #13#10#13#10 +
    'To start or uninstall it later, search the Start menu for Onion Launcher.';
end;

// The launcher picture for this screen's scale, made to size by make_installer_art.py so
// it's never stretched
function BoxPicture: String;
var
  Pct: Integer;
begin
  Pct := ScaleX(100);
  if Pct < 113 then Pct := 100
  else if Pct < 138 then Pct := 125
  else if Pct < 163 then Pct := 150
  else if Pct < 188 then Pct := 175
  else Pct := 200;
  Result := 'box-light-' + IntToStr(Pct) + '.png';
  ExtractTemporaryFile(Result);
  Result := ExpandConstant('{tmp}\') + Result;
end;

function AddBox(Parent: TWinControl): TBitmapImage;
begin
  Result := TBitmapImage.Create(WizardForm);
  Result.Parent := Parent;
  Result.AutoSize := True;
  Result.PngImage.LoadFromFile(BoxPicture);
end;

// The first page's text column, sized to the space left of the picture; its height
// comes back (the picture's position is set by LayOut)
function SizeTasksText: Integer;
var
  Left, Right: Integer;
begin
  Right := WizardForm.SelectTasksPage.ClientWidth - TasksBox.Width - ScaleX(24);
  Left := WizardForm.SelectTasksLabel.Left;
  WizardForm.SelectTasksLabel.Width := Right - Left;
  WizardForm.SelectTasksLabel.AdjustHeight;
  WizardForm.TasksList.Width := Right - WizardForm.TasksList.Left;
  WizardForm.TasksList.Height := ScaleY(24);   // its one box
  HotkeyCaption.Left := Left;
  HotkeyEdit.Left := Left;
  HotkeyEdit.Width := Right - Left;
  HotkeyEdit.Height := ScaleY(25);
  HotkeyNote.Left := Left;
  HotkeyNote.Width := Right - Left;
  // two lines, so a longer message never runs into the box below
  HotkeyNote.Height := ScaleY(32);
  Result := WizardForm.SelectTasksLabel.Height + ScaleY(14) + HotkeyCaption.Height + ScaleY(4) +
    HotkeyEdit.Height + ScaleY(4) + HotkeyNote.Height + ScaleY(4) + WizardForm.TasksList.Height;
end;

procedure InitializeWizard;
var
  Need, Spare: Integer;
begin
  // and in case Inno draws its own pictures anyway: hidden
  WizardForm.WizardBitmapImage.Visible := False;
  WizardForm.WizardBitmapImage2.Visible := False;
  WizardForm.WizardSmallBitmapImage.Visible := False;
  TasksBox := AddBox(WizardForm.SelectTasksPage);
  DoneBox := AddBox(WizardForm.FinishedPage);
  // Inno's FinishedLabel reports a box much taller than its text, so nothing can be
  // placed right under it: this label, with the same words, measures true
  DoneText := TNewStaticText.Create(WizardForm);
  DoneText.Parent := WizardForm.FinishedPage;
  DoneText.AutoSize := False;
  DoneText.WordWrap := True;
  DoneText.ShowAccelChar := False;
  DoneText.Caption := SetupMessage(msgFinishedLabel);
  // the hotkey: what's in use now, until new keys are pressed in its box
  ReadOldHotkey;
  PickMods := OldMods;
  PickKey := OldKey;
  // (its one-copy-at-a-time mutex: seen from any desktop, unlike its window)
  RunningCopy := CheckForMutexes('Local\OnionLauncher.single');
  HotkeyCaption := TNewStaticText.Create(WizardForm);
  HotkeyCaption.Parent := WizardForm.SelectTasksPage;
  HotkeyCaption.Caption := 'Hotkey';
  HotkeyCaption.Font.Style := [fsBold];
  HotkeyEdit := TNewEdit.Create(WizardForm);
  HotkeyEdit.Parent := WizardForm.SelectTasksPage;
  HotkeyEdit.ReadOnly := True;
  HotkeyEdit.AutoSelect := False;
  HotkeyEdit.OnKeyDown := @HotkeyKeyDown;
  HotkeyEdit.OnKeyUp := @HotkeyKeyUp;
  HotkeyEdit.OnEnter := @HotkeyEnter;
  HotkeyEdit.OnExit := @HotkeyExit;
  ShowPick;
  HotkeyNote := TNewStaticText.Create(WizardForm);
  HotkeyNote.Parent := WizardForm.SelectTasksPage;
  HotkeyNote.AutoSize := False;
  HotkeyNote.WordWrap := True;
  HotkeyNote.ShowAccelChar := False;
  SetNote('Click it and press new keys', NoteGrey);
  // an app that already has the usual hotkey would also keep it from Onion Launcher
  if not HotkeyFree(PickMods, PickKey) then
    SetNote('Another app already uses this one: click it and press new keys', NoteRed);
  // Inno puts the pages' content well below the heading: bring it up to a normal gap (the
  // heading's panel too, or it would cover the top of the page)
  WizardForm.MainPanel.Height := WizardForm.MainPanel.Height - ScaleY(12);
  WizardForm.InnerNotebook.Top := WizardForm.InnerNotebook.Top - ScaleY(20);
  WizardForm.InnerNotebook.Height := WizardForm.InnerNotebook.Height + ScaleY(20);
  // and its window is taller than these two short pages need: fit it to the first page's
  // picture or text, whichever is taller, so there's no empty band above the buttons
  Need := SizeTasksText;
  if TasksBox.Height > Need then Need := TasksBox.Height;
  Spare := WizardForm.SelectTasksPage.ClientHeight - Need - ScaleY(8);
  WizardForm.ClientHeight := WizardForm.ClientHeight - Spare;
end;

procedure InitializeUninstallProgressForm;
begin
  UninstallProgressForm.WizardSmallBitmapImage.Visible := False;
end;

// Text on the left, the picture on the right: side by side as one block, the text centred
// against the picture
procedure LayOut(CurPageID: Integer);
var
  Gap, Left, Right, TextH, BlockH: Integer;
begin
  Gap := ScaleX(24);
  if CurPageID = wpSelectTasks then
  begin
    TasksBox.Left := WizardForm.SelectTasksPage.ClientWidth - TasksBox.Width;
    TextH := SizeTasksText;
    BlockH := TasksBox.Height;
    if TextH > BlockH then BlockH := TextH;
    // right under the heading (InitializeWizard moved the page up and shortened the window)
    TasksBox.Top := (BlockH - TasksBox.Height) div 2;
    WizardForm.SelectTasksLabel.Top := (BlockH - TextH) div 2;
    HotkeyCaption.Top := WizardForm.SelectTasksLabel.Top + WizardForm.SelectTasksLabel.Height + ScaleY(14);
    HotkeyEdit.Top := HotkeyCaption.Top + HotkeyCaption.Height + ScaleY(4);
    HotkeyNote.Top := HotkeyEdit.Top + HotkeyEdit.Height + ScaleY(4);
    WizardForm.TasksList.Top := HotkeyNote.Top + HotkeyNote.Height + ScaleY(4);
  end
  else if CurPageID = wpFinished then
  begin
    DoneBox.Left := WizardForm.FinishedPage.ClientWidth - DoneBox.Width - ScaleX(40);
    DoneBox.Top := (WizardForm.FinishedPage.ClientHeight - DoneBox.Height) div 2;
    Right := DoneBox.Left - Gap;
    // Inno leaves room on the left for the side panel this setup doesn't have
    Left := ScaleX(40);
    WizardForm.FinishedHeadingLabel.Left := Left;
    WizardForm.FinishedHeadingLabel.Width := Right - Left;
    WizardForm.FinishedHeadingLabel.Height := ScaleY(30);   // one line (Inno's box is much taller)
    WizardForm.FinishedLabel.Visible := False;
    DoneText.Caption := DoneCaption;
    DoneText.Left := Left;
    DoneText.Width := Right - Left;
    DoneText.AdjustHeight;
    WizardForm.RunList.Left := Left;
    WizardForm.RunList.Width := Right - Left;
    WizardForm.RunList.Height := ScaleY(24);   // its one box
    TextH := WizardForm.FinishedHeadingLabel.Height + ScaleY(12) + DoneText.Height +
      ScaleY(20) + WizardForm.RunList.Height;
    WizardForm.FinishedHeadingLabel.Top := DoneBox.Top + (DoneBox.Height - TextH) div 2;
    DoneText.Top := WizardForm.FinishedHeadingLabel.Top + WizardForm.FinishedHeadingLabel.Height + ScaleY(12);
    WizardForm.RunList.Top := DoneText.Top + DoneText.Height + ScaleY(20);
    // Finish where Cancel was (it's hidden here), not stranded in the middle
    WizardForm.NextButton.Left := WizardForm.CancelButton.Left;
  end;
end;

// The page's button says Install: there's no Ready page after it
procedure CurPageChanged(CurPageID: Integer);
var
  Caption: String;
begin
  LayOut(CurPageID);
  if CurPageID <> wpSelectTasks then
    exit;
  // (without its Alt + I shortcut, which would install while Alt + I is pressed in the
  // hotkey box; Enter still clicks it)
  Caption := SetupMessage(msgButtonInstall);
  StringChangeEx(Caption, '&', '', True);
  WizardForm.NextButton.Caption := Caption;
  // An update over 0.2.2 (installed by its own exe, so this installer has no record of
  // the box): keep what was chosen then
  if not StartupChecked and RegKeyExists(HKCU, OldEntry) and
     not RegValueExists(HKCU, RunKey, '{#AppExeName}') then
    WizardSelectTasks('!startup');
  StartupChecked := True;
end;

// Ask a running copy to close (it saves after every change), so its exe can be replaced
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
  Exe: String;
begin
  Result := '';
  Exe := ExpandConstant('{app}\{#AppExeName}.exe');
  if FileExists(Exe) then
    Exec(Exe, '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code);
end;

// A hotkey picked on the first page: the new exe saves it with the apps, before it opens
procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
begin
  if CurStep = ssInstall then
    Installed := True;
#ifndef PREVIEW
  if (CurStep = ssPostInstall) and ((PickMods <> OldMods) or (PickKey <> OldKey)) then
    Exec(ExpandConstant('{app}\{#AppExeName}.exe'),
      '--set-hotkey ' + IntToStr(PickMods) + ' ' + IntToStr(PickKey), '', SW_HIDE,
      ewWaitUntilTerminated, Code);
#endif
end;

// Cancelled after the hotkey box closed the running copy: start it again
procedure DeinitializeSetup;
var
  Code: Integer;
begin
  if StoppedCopy and not Installed then
    Exec(ExpandConstant('{app}\{#AppExeName}.exe'), '', '', SW_SHOWNORMAL, ewNoWait, Code);
end;
