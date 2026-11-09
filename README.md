# Onion Launcher

Your apps, one key away. Press **Win + `** and a small box of your favourite apps pops up
next to the mouse. Click one, or press its number, and it opens.

<p align="center">
  <img src="docs/images/launcher-dark.png" alt="Onion Launcher in dark mode: a box of app tiles in sections called Games and Create" width="49%">
  <img src="docs/images/launcher-light.png" alt="The same box in light mode" width="49%">
</p>

Free and open source, for Windows 11. Small (one exe, no install), and it never goes online.

## Using it

- **Open the box:** press **Win + `** (the key left of 1), or click its icon in the tray.
  Press **Esc** or click anywhere else to close it.
- **Open an app:** click it, or press its number (**1** to **9**, then **0**).
- **Add apps:** click the **+** tile (or press **+**) to search every app on the PC, then click
  one or press **Enter**. **Browse files** at the bottom picks any program or shortcut.
  You can also drop files straight onto the box.
- **Move apps around:** drag a tile to a new spot.
- **Categories:** right-click an app, **Move to category > New category...**, and give it a
  name. Drag apps in and out of a category. Right-click a heading to rename or delete it
  (its apps go back to the top).
- **Remove an app:** right-click it and pick **Remove from Onion Launcher**, or drag it
  out of the box and hold it there for 2 seconds until it says **Release to remove**.
  Let go sooner and nothing happens.

<p align="center">
  <img src="docs/images/release-to-remove.png" alt="An app held outside the box: the box says Release to remove" width="49%">
</p>

It follows Windows' light or dark mode by itself. If another app already uses Win + `,
Onion Launcher says so once and the tray icon still works.

## Download

Get `OnionLauncher.exe` from the [latest release](../../releases/latest) and run it. There's
nothing to install. The exe isn't code-signed yet, so Windows may show a "Windows protected
your PC" warning the first time: click **More info**, then **Run anyway**.

**Start it with Windows:** press **Win + R**, type `shell:startup`, and put a shortcut to
`OnionLauncher.exe` in the folder that opens.

## Your data

Your list of apps is saved in `%APPDATA%\OnionLauncher\apps.json`, and that's all it
stores. Onion Launcher has no account, no updates check and no usage counting: it never
connects to the internet.

**To uninstall:** right-click the tray icon, **Quit Onion Launcher**, then delete
`OnionLauncher.exe` and the `%APPDATA%\OnionLauncher` folder.

## Build it yourself

You need [Rust](https://rustup.rs) (stable).

```
cargo build --release
```

The exe lands in `target\release\OnionLauncher.exe`. To make a copy to share, use
`powershell -File scripts\build-release.ps1`: it builds the same exe but leaves your own
folder names out of it, and checks that they're gone.

### The self-test

The app can drive itself (opening the box, hovering, clicking, typing, dragging) and save a
picture of every step. It runs on a hidden desktop, so nothing appears on your screen:

```
powershell -File tools\run_hidden.ps1 -CommandLine "target\release\OnionLauncher.exe --selftest <out folder> --tag cats --theme dark --config <test list>.json"
```

Tags: `full`, `dark`, `empty`, `big`, `cats` (categories and dragging), `frame` and `readme`
(the pictures on this page). `--config` points it at a test copy of the app list, so your
real list is never touched. Use a made-up list for anything you share.

## Licence

[GPL-3.0](LICENSE). App names and icons in the pictures belong to their owners.
