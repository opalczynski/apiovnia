# Apiovnia icon set

Single-source: `icons/master/apiovnia-1024.png` (raster) + `apiovnia.svg` (vector).

────────────────────────────────────────────────────────────
⚠️  Rename @2x files before building
────────────────────────────────────────────────────────────
The export filesystem replaced "@" with "-" in filenames. Before
running iconutil or pointing Tauri at the iconset, rename them back:

  cd icons/macos/apiovnia.iconset
  for f in *-2x.png; do mv "$f" "${f/-2x/@2x}"; done

  cd icons/tauri
  mv 128x128-2x.png 128x128@2x.png

────────────────────────────────────────────────────────────
macOS · build apiovnia.icns
────────────────────────────────────────────────────────────
  cd icons/macos
  iconutil -c icns apiovnia.iconset
  # → apiovnia.icns

────────────────────────────────────────────────────────────
Windows · build apiovnia.ico
────────────────────────────────────────────────────────────
  cd icons/windows
  magick convert 16.png 24.png 32.png 48.png 64.png 128.png 256.png apiovnia.ico

  # Square*Logo.png + StoreLogo.png — only if you ship MSIX to Microsoft Store.

────────────────────────────────────────────────────────────
Linux · install
────────────────────────────────────────────────────────────
  sudo cp -r icons/linux/hicolor/* /usr/share/icons/hicolor/
  sudo cp icons/linux/apiovnia.desktop /usr/share/applications/
  sudo gtk-update-icon-cache /usr/share/icons/hicolor

────────────────────────────────────────────────────────────
Tauri · the easy path (recommended)
────────────────────────────────────────────────────────────
Skip the manual generation entirely:

  cargo tauri icon icons/master/apiovnia-1024.png

That CLI command regenerates every required PNG, .icns and .ico
from a single source PNG into src-tauri/icons/.

────────────────────────────────────────────────────────────
Color spec
────────────────────────────────────────────────────────────
  Background:  #1F1714 → #0B0908  (linear gradient, top → bottom)
  Accent:      #F59E0B            (amber)
  Highlight:   #FBBF24            (drop glint, ≥128 px only)
  Outer ring:  #2A2A2F            (subtle, only ≥64 px)
  Squircle:    22.5% corner radius

────────────────────────────────────────────────────────────
Size-dependent rendering
────────────────────────────────────────────────────────────
  ≤ 24 px   filled solid hexagon (no outlines)
  32–48 px  amber outline + center drop
  ≥ 64 px   outer ring + outline + drop
  ≥ 128 px  + drop glint
