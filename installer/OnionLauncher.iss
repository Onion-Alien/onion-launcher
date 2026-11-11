; OnionLauncherSetup.exe: the download. A normal per-user Inno Setup installer, like
; Onion Board's and Onion Watch's, so the app's own exe never installs itself (0.2.2's
; self-installing exe was called a trojan by Windows Defender's machine-learning guess).
;
;   - no admin needed: %LOCALAPPDATA%\Programs\Onion Launcher, the same folder as 0.2.2
;   - one page: what it is, a picture of the launcher box, and a "Start with Windows"
;     box (ticked; an update keeps the last install's choice). Then
;     installing, then "All done" with "Open Onion Launcher now"
;   - Inno's Windows 11 look, light or dark like Windows, with no side panel or header
;     logo: the launcher picture is the only image
;   - Start menu: "Onion Launcher" and "Uninstall Onion Launcher"; Settings > Apps entry
;   - a running copy is asked to close first (OnionLauncher.exe --quit), on install and
;     uninstall, so its exe can be replaced or removed
;   - updating over 0.2.2: its own Settings > Apps entry is removed, so there's one entry,
;     and its startup choice is kept
;   - uninstalling leaves the list of apps in %APPDATA%\OnionLauncher
;
; Built by scripts\build-release.ps1 (needs Inno Setup 6.6 or newer:
; winget install JRSoftware.InnoSetup), which builds the exe and draws the pictures first
; (scripts\make_installer_art.py: the icon, and the launcher picture at each display scale). /DSHOTSTYLE=dark or light forces that look, for the
; README pictures; otherwise it follows Windows' own light or dark mode. /DPREVIEW is a
; look at the pages only: it installs nothing (scripts\installer_pictures.ps1).

#define AppName "Onion Launcher"
#define AppExeName "OnionLauncher"
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SHOTSTYLE
  #define SHOTSTYLE "dynamic"
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
AppComments=Your apps, one key away: Win + the key left of 1
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
WizardStyle=modern {#SHOTSTYLE} windows11 hidebevels
WizardSizePercent=100
SetupIconFile=..\target\installer\OnionLauncher.ico
; no side panel or header logo (InitializeWizard adds the launcher picture instead). The
; DynamicDark ones too: in "follow Windows" mode a dark PC otherwise gets Inno's own
; artwork (a box and a disc) behind the last page's text
WizardImageFile=
WizardSmallImageFile=
WizardImageFileDynamicDark=
WizardSmallImageFileDynamicDark=
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
SelectTasksLabel2=Press Win + ` (the key left of 1) and a box of your apps opens next to the mouse. Click one, or press its number.%n%nOnion Launcher waits in the tray, needs no admin rights and never goes online.
FinishedHeadingLabel=All done!
FinishedLabel=Onion Launcher is installed. To open your apps, press Win + ` or click its icon in the tray.%n%nTo start or uninstall it later, search the Start menu for Onion Launcher.
FinishedLabelNoIcons=Onion Launcher is installed. To open your apps, press Win + ` or click its icon in the tray.%n%nTo start or uninstall it later, search the Start menu for Onion Launcher.
ClickFinish=

[Tasks]
Name: "startup"; Description: "Start with Windows"

[Files]
; the launcher picture, for the pages only (never installed)
Source: "..\target\installer\box-*.png"; Flags: dontcopy
#ifndef PREVIEW
Source: "..\target\release\{#AppExeName}.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}.exe"; Comment: "Your apps, one key away: Win + the key left of 1"
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

var
  StartupChecked: Boolean;
  TasksBox, DoneBox: TBitmapImage;
  DoneText: TNewStaticText;   // the last page's text, in place of Inno's FinishedLabel

// The launcher picture for this screen's scale and Setup's light or dark look, made to
// size by make_installer_art.py so it's never stretched
function BoxPicture: String;
var
  Pct: Integer;
  Theme: String;
begin
  Pct := ScaleX(100);
  if Pct < 113 then Pct := 100
  else if Pct < 138 then Pct := 125
  else if Pct < 163 then Pct := 150
  else if Pct < 188 then Pct := 175
  else Pct := 200;
  if IsDarkInstallMode then Theme := 'dark' else Theme := 'light';
  Result := 'box-' + Theme + '-' + IntToStr(Pct) + '.png';
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

procedure InitializeWizard;
var
  Spare: Integer;
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
  // Inno puts the pages' content well below the heading: bring it up to a normal gap (the
  // heading's panel too, or it would cover the top of the page)
  WizardForm.MainPanel.Height := WizardForm.MainPanel.Height - ScaleY(12);
  WizardForm.InnerNotebook.Top := WizardForm.InnerNotebook.Top - ScaleY(20);
  WizardForm.InnerNotebook.Height := WizardForm.InnerNotebook.Height + ScaleY(20);
  // and its window is taller than these two short pages need: shorten it to fit the
  // picture, so there's no empty band above the buttons
  Spare := WizardForm.SelectTasksPage.ClientHeight - TasksBox.Height - ScaleY(8);
  if Spare > 0 then
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
    Right := TasksBox.Left - Gap;
    WizardForm.SelectTasksLabel.Width := Right - WizardForm.SelectTasksLabel.Left;
    WizardForm.SelectTasksLabel.AdjustHeight;
    WizardForm.TasksList.Width := Right - WizardForm.TasksList.Left;
    WizardForm.TasksList.Height := ScaleY(24);   // its one box
    TextH := WizardForm.SelectTasksLabel.Height + ScaleY(16) + WizardForm.TasksList.Height;
    BlockH := TasksBox.Height;
    if TextH > BlockH then BlockH := TextH;
    // right under the heading (InitializeWizard moved the page up and shortened the window)
    TasksBox.Top := (BlockH - TasksBox.Height) div 2;
    WizardForm.SelectTasksLabel.Top := (BlockH - TextH) div 2;
    WizardForm.TasksList.Top := WizardForm.SelectTasksLabel.Top + WizardForm.SelectTasksLabel.Height + ScaleY(16);
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
begin
  LayOut(CurPageID);
  if CurPageID <> wpSelectTasks then
    exit;
  WizardForm.NextButton.Caption := SetupMessage(msgButtonInstall);
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
