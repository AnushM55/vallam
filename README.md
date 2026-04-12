# WM 15
- A [neuswc](https://git.sr.ht/~shrub900/neuswc) based window manager written in rust
- Warning : Work in progress

# Installation 
- You need to have cargo installed
- run `make` in project root
- optional install: `sudo make install` (installs `wm15`, `wm15-wayrec`, `swcsnap`)

# Screen recording
- Keybinding: `Super+Shift+r` toggles recording.
- Recorder uses `wm15-wayrec` (default command; override with `WM15_WAYREC_CMD`).
- `wm15-wayrec` auto-detects `swcsnap`; override path with `WM15_SWCSNAP_BIN`.
- Top-level behavior mirrors your `wayrec` script: toggle start/stop, select area with `slurp`, save mp4 to `$HOME/vids/recs`, notifications.
- Backend capture uses neuswc `swcsnap` + `ffmpeg` encoding.
- Dependencies: `slurp`, `ffmpeg`, `notify-send` (optional), and `swcsnap`.
- Status check: `wm15-wayrec --status` (shows running state + last status/log path).
- Crash diagnostics: logs in `${XDG_RUNTIME_DIR:-/tmp}/wm15-wayrec/` (`status`, `debug.log`, `*.log`).
- If notifications are unavailable in your session, set `WM15_DISABLE_NOTIFY=1`.

# Etymology

![XKCD - 927](https://imgs.xkcd.com/comics/standards.png)

# STATUS

![Recording](./assets/recording.gif)

[Watch MP4](./assets/recording.mp4)
