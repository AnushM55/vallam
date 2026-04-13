# Vallam
- A [neuswc](https://git.sr.ht/~shrub900/neuswc) based window manager written in Rust.
- Warning: work in progress.

## Dependencies

### Build dependencies
- `rust` / `cargo`
- `meson`
- `ninja`
- System libraries required by `neuswc`

### Runtime dependencies
- `swc-launch` (used to start the compositor)
- `swcsnap` (built from `neuswc` by `make`)
- `slurp` (for area selection in recording)
- `ffmpeg` (for encoding recordings)
- `notify-send` (optional, for desktop notifications)

## Installation
- Run `make` in the project root.
- Optional install: `sudo make install` (installs `vallam`, `vallam-wayrec`, `swcsnap`).

## Running the project
(Usually called **Usage** or **Running** in READMEs.)

Start Vallam through `swc-launch`:

- Without installing:
  - `make`
  - `swc-launch ./target/release/vallam`
- After installing:
  - `swc-launch vallam`

## STATUS

![Recording](./assets/recording.gif)

[Watch MP4](./assets/recording.mp4)
