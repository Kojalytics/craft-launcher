# Kojalytics Launcher app icon

The app icon is the **Kojalytics "K" mark** in [`docs/brand/kojalytics-mark.svg`](../../docs/brand/kojalytics-mark.svg).
Every file in this directory is rendered from it by `packaging/icons.sh`.

**These files are a Kojalytics trademark. They are not open source and not covered by the
launcher's MIT or Apache-2.0 license.**

## Files

- `artcraft-launcher.svg`: a copy of `docs/brand/kojalytics-mark.svg`; also the hicolor scalable icon.
- `artcraft-launcher-1024.png`: 1024 px render, with extra transparent margin for Apple's icon grid.
- `artcraft-launcher.icns`: macOS bundle icon (`CFBundleIconFile`).
- `artcraft-launcher.ico`: Windows icon, 16–256 px, embedded in the `.exe` by
  `apps/artcraft-launcher/build.rs` and used by the MSI.
- `hicolor/<size>/apps/com.kojalytics.artcraft-launcher.png` (16–512) and
  `hicolor/scalable/apps/com.kojalytics.artcraft-launcher.svg`: Linux icon theme.
- The tray / menu-bar icon is the same mark, rendered into `docs/brand/kojalytics-mark-black-44.png`
  (macOS template, 22 pt @2x) and `docs/brand/kojalytics-mark-64.png` (Windows, Linux, FreeBSD).

## Regenerate

Run `packaging/icons.sh` on any OS. It needs `resvg`; the `.ico` and `.icns` are packed by
`cargo xtask ico`. Never edit these files by hand.
