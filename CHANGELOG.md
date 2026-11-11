# Changelog

## 0.2.3 (2026-11-11)

- A proper installer, `OnionLauncherSetup.exe`, instead of an exe that installed itself:
  Windows Defender's machine-learning check took 0.2.2's self-installing exe for a trojan
  (`Trojan:Win32/Bearfoos.A!ml`, a false alarm) and removed it. The app itself no longer
  has any install code, and its exe now has a name, publisher and version in
  Properties > Details. The installer follows Windows' light or dark mode, has the same
  "Start Onion Launcher when Windows starts" box, installs to the same folder with no admin
  rights, and updates 0.2.2 in place (your list of apps stays).

## 0.2.2 (2026-11-11)

- An installer, the same as Onion Peel's: run the downloaded `OnionLauncher.exe` and a
  small window installs it to `%LOCALAPPDATA%\Programs\Onion Launcher` (no admin rights),
  with Start menu entries, an entry in Settings > Apps, and an optional "Start Onion
  Launcher when Windows starts" box (ticked by default). Running a newer exe updates it
  and keeps your list; **Uninstall Onion Launcher** in the Start menu removes it.

## 0.2.1 (2026-11-10)

- Fixed: if another program had `apps.json` open when Onion Launcher started, it started
  with an empty list and the next change wrote over the real one. Now it leaves the file
  alone, says so, and saves nothing until it's restarted.
- Fixed: a list saved by Notepad as "UTF-8 with BOM" was treated as damaged.
- Fixed: the hotkey is now the key left of 1 on every keyboard layout (it was on a
  different key on UK, German and French keyboards, for example).
- Saving flushes the list to disk before swapping it in, so a power cut can't leave an
  empty file.
- Self-test lists in `tests/fixtures/`, unit tests for loading and saving, and checks on
  every push (build, Clippy, tests, privacy scan).

## 0.2.0 (2026-11-10)

First release.

- **Win + `** or the tray icon opens a box of your apps next to the mouse; click one or press
  its number (1-9, 0) to open it.
- **+** searches every app in Start (desktop and Store apps); **Browse files** picks any
  program or shortcut; files can be dropped onto the box.
- Drag tiles to reorder them, sort them into named categories, and remove one by holding it
  outside the box for 2 seconds or from the right-click menu.
- Follows Windows' light or dark mode; sharp at any display scale.
- The list is saved in `%APPDATA%\OnionLauncher\apps.json`; a damaged file is kept as
  `apps.broken.json` instead of being overwritten.
- Never goes online.
