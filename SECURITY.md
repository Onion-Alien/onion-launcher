# Security

## What Onion Launcher does on your PC

- **It never goes online.** No account, no update check, no usage counting, no network code
  at all.
- **It stores one file:** your list of apps in `%APPDATA%\OnionLauncher\apps.json` (plus
  `apps.broken.json` if that file was ever damaged). Nothing else is written.
- **It reads:** that file, Start's app list (`shell:AppsFolder` and the Start menu shortcuts,
  once when it starts, for the **+** search), the icons of your apps, and one Windows
  setting (light or dark mode).
- **It runs programs only when you ask:** clicking a tile or pressing its number opens it
  the same way Explorer would. It doesn't start itself with Windows unless you put it in
  your Startup folder.
- **It needs no admin rights** and changes no system settings.

Anyone who can edit your `apps.json` can make a tile run any program, the same as editing
a shortcut on your desktop. Keep it in your own user folder.

## Reporting a problem

If you find a security problem, please don't open a public issue. Use GitHub's
**Report a vulnerability** button on this repo's **Security** tab, so it reaches the
maintainer privately. Please include the Onion Launcher version (the release you
downloaded) and the steps to reproduce it.

Only the latest release gets fixes.
