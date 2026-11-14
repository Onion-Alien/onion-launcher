# Changelog

## 0.2.10 (2026-11-14)

- **Pick your hotkey when you install:** the installer shows the hotkey (Win + `, or the
  one you picked before). Click it and press the keys you want, like Ctrl + Space. A
  shortcut another app already uses is refused, like in **Change hotkey...**.
- The README and the website now say you can pick any hotkey, not only Win + `.
- Fixed: with more than one keyboard layout installed (like US and UK), Win + the key left
  of 1 only worked in the layout that was on when Onion Launcher started. That key has a
  different key code in each layout, so after switching layouts it did nothing. It now
  works in every installed layout.

## 0.2.9 (2026-11-12)

- **Change the hotkey:** right-click the tray icon and pick **Change hotkey...**, then press
  the new keys (with Ctrl, Alt or Win, like Ctrl + Space). It's saved with your apps, and
  the tray menu and tooltip show the hotkey you picked. A shortcut another app already
  uses is refused with a message, and **Esc** keeps the old one.
- The tray icon's menu now always opens above the taskbar. On some Windows 11 builds the
  taskbar covered its last item (Quit).

## 0.2.8 (2026-11-11)

- The installer now scans clean on VirusTotal. A few virus scanners guessed "malware" from
  the look of the setup file alone (Onion Launcher itself always scanned clean). To stop
  that, the setup file has the standard setup icon instead of Onion Launcher's own, and a
  plain light window. Onion Launcher itself is the same as 0.2.7 and keeps its icon.

## 0.2.7 (2026-11-11)

Fixes from a read-through of all the code. Nothing looks or works differently otherwise.

- Picking a program with **Browse files** no longer leaves that folder "in use" until
  Onion Launcher is closed (it couldn't be renamed or deleted, and a USB stick couldn't
  be ejected).
- A list with the very same app in it twice (only possible by editing `apps.json` by
  hand) drew the two tiles on top of each other. Each copy now gets its own tile.
- Pasting a huge text into the search box or a category name can no longer freeze the
  box: it takes up to 256 characters.
- An app whose icon can't be loaded at a new screen scale keeps the icon it had, instead
  of falling back to a letter.
- The animation timer always stops when the box is hidden, so a hidden Onion Launcher
  does no work at all.

## 0.2.6 (2026-11-11)

- A more compact installer: the window fits its content, with the page right under the
  heading and no empty band above the buttons. On the last page, "Open Onion Launcher
  now" sits straight under the text.

## 0.2.5 (2026-11-11)

- Rows of tiles are only as tall as their names: a row where every name fits on one line
  no longer leaves an empty gap under it, and a row with a two-line name gets the same
  space under that second line as above the icons.

## 0.2.4 (2026-11-11)

- The installer's last page showed Inno Setup's own picture behind the text when Windows
  is in dark mode; it's gone. Finish sits on the right, the picture lines up with the
  text, and the box now just says "Start with Windows".
- A tile's highlight (on hover, when pressed or dragged) now ends just under its name,
  instead of leaving room for a second line the name doesn't have.

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
