This program was written with the help of [Claude](https://claude.ai), Anthropic's AI assistant.

## Dropall — universal downloader

Download video and music from YouTube and Spotify, any file with the built-in download manager, and torrents (magnet links and .torrent files).

### What's new
- **Torrents** (new tab, powered by [libtorrent](https://github.com/arvidn/libtorrent)): paste magnet links or `.torrent` addresses, open `.torrent` files from your computer (button or drag and drop), or click a **magnet link in your browser**: Dropall opens, or comes to the front if it is already running, and adds it. Torrents resume after a restart; you can keep seeding after the download and limit the upload speed. On Windows the installer registers Dropall for `magnet:` links and `.torrent` files (optional); on Linux the AppImage menu entry does the same. On macOS use paste, open or drag and drop.
- **System tray**: minimize Dropall to the notification area and downloads (videos, files and torrents) keep going; closing the window while something is downloading does the same. Use the tray icon to show the window or quit. Can be turned off at the bottom of the window.
- **Percentage next to every progress bar**, in all three tabs.
- **Clearer interface**: three tabs (Video & music, Files, Torrent), each with a one-line explanation, and a new description under the title.
- **Third-party licenses** (libtorrent BSD-3-Clause, Boost, OpenSSL) are shown in the app (*Licenses & credits*) and included in every download.
- **Windows installer**: `Dropall-Setup-windows-x86_64.exe` installs for your user only (no administrator rights), adds a Start menu entry and an uninstaller, and the app keeps updating itself. The portable `.exe` is still available.
- **Download manager** (new tab): files from any http(s) link, with multiple connections per file (up to 16), queues downloaded one after the other, a scheduler that starts and stops each queue by itself, and a speed limit shared by all downloads, video and music included. Pause and resume survive closing the app.
- **Playlists**: a YouTube playlist is saved in a folder named after the playlist; file names no longer repeat the playlist name.
- **Automatic updates at startup**: Dropall checks for a new release as soon as it opens, installs it and restarts; a status line tells you how the check went.
- **Italian and English**: the interface follows your system language (Italian, otherwise English).
- **Screen reader support** on Windows, macOS and Linux (Narrator, NVDA, VoiceOver, Orca).
- **Window title bar fixed**: on Windows it no longer appears light and without buttons at first launch, and on Linux it follows the system theme.
- **More readable text**: selected text now has a clear contrast (it was unreadable on orange); text size can be increased.
- **Universal downloader**: the app now presents itself as such, with video and music, the download manager and torrents in separate tabs.
- **Free software**: released under the GPL-3.0-or-later license.

### Downloads
`Dropall-Setup-windows-x86_64.exe` (Windows installer) or `dropall-windows-x86_64.exe` (Windows portable), `dropall-linux-x86_64.AppImage` (Linux), `Dropall-macos-*.app.zip` (macOS). Use Dropall only for content you have the right to download and share. Installed copies update themselves.

### Changes in this version
