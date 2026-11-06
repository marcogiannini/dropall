This program was written with the help of [Claude](https://claude.ai), Anthropic's AI assistant.

## Grabbit — universal downloader

Download video and music from YouTube and Spotify, and any file with the built-in download manager.

### What's new
- **Windows installer**: `Grabbit-Setup-windows-x86_64.exe` installs for your user only (no administrator rights), adds a Start menu entry and an uninstaller, and the app keeps updating itself. The portable `.exe` is still available.
- **Download manager** (new tab): files from any http(s) link, with multiple connections per file (up to 16), queues downloaded one after the other, a scheduler that starts and stops each queue by itself, and a speed limit shared by all downloads, video and music included. Pause and resume survive closing the app.
- **Playlists**: a YouTube playlist is saved in a folder named after the playlist; file names no longer repeat the playlist name.
- **Automatic updates at startup**: Grabbit checks for a new release as soon as it opens, installs it and restarts; a status line tells you how the check went.
- **Italian and English**: the interface follows your system language (Italian, otherwise English).
- **Screen reader support** on Windows, macOS and Linux (Narrator, NVDA, VoiceOver, Orca).
- **Window title bar fixed**: on Windows it no longer appears light and without buttons at first launch, and on Linux it follows the system theme.
- **More readable text**: selected text now has a clear contrast (it was unreadable on orange); text size can be increased.
- **Universal downloader**: the app now presents itself as such, with video and music and the download manager in two separate tabs.
- **Free software**: released under the GPL-3.0-or-later license.

### Downloads
`Grabbit-Setup-windows-x86_64.exe` (Windows installer) or `grabbit-windows-x86_64.exe` (Windows portable), `grabbit-linux-x86_64.AppImage` (Linux), `Grabbit-macos-*.app.zip` (macOS). Installed copies update themselves.

### Changes in this version
