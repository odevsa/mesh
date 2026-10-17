#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Parse targets
GEN_LINUX=false
GEN_MACOS=false
GEN_WINDOWS=false

if [ "$#" -eq 0 ]; then
  GEN_LINUX=true
  GEN_MACOS=true
  GEN_WINDOWS=true
else
  for arg in "$@"; do
    case "$arg" in
      --linux|-l)
        GEN_LINUX=true
        ;;
      --macos|--mac|-m)
        GEN_MACOS=true
        ;;
      --windows|--win|-w)
        GEN_WINDOWS=true
        ;;
      --all|-a)
        GEN_LINUX=true
        GEN_MACOS=true
        GEN_WINDOWS=true
        ;;
      -h|--help)
        echo "Usage: $(basename "$0") [--linux] [--macos] [--windows] [--all]"
        exit 0
        ;;
      *)
        echo "Unknown option: $arg" >&2
        echo "Usage: $(basename "$0") [--linux] [--macos] [--windows] [--all]" >&2
        exit 1
        ;;
    esac
  done
fi

# Find best source icon (SVG preferred, fallback to 512x512 PNG)
SOURCE_ICON=""
if [ -f "${ROOT_DIR}/assets/icon.svg" ]; then
  SOURCE_ICON="${ROOT_DIR}/assets/icon.svg"
elif [ -f "${ROOT_DIR}/assets/mesh.svg" ]; then
  SOURCE_ICON="${ROOT_DIR}/assets/mesh.svg"
elif [ -f "${ROOT_DIR}/assets/icons/512x512/apps/mesh.png" ]; then
  SOURCE_ICON="${ROOT_DIR}/assets/icons/512x512/apps/mesh.png"
fi

if [ -z "${SOURCE_ICON}" ] || [ ! -f "${SOURCE_ICON}" ]; then
  echo "Error: No source icon found in assets/" >&2
  exit 1
fi

# Target OS-specific asset directories inside packaging/
LINUX_ASSETS="${ROOT_DIR}/packaging/linux/assets"
MACOS_ASSETS="${ROOT_DIR}/packaging/macos/assets"
WINDOWS_ASSETS="${ROOT_DIR}/packaging/windows/assets"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

# Determine required icon sizes based on targets
SIZES=()
for s in 16 24 32 48 64 128 256 512 1024; do
  case "$s" in
    1024)
      if [ "$GEN_MACOS" = true ]; then SIZES+=("$s"); fi
      ;;
    512)
      if [ "$GEN_LINUX" = true ] || [ "$GEN_MACOS" = true ]; then SIZES+=("$s"); fi
      ;;
    24|48)
      if [ "$GEN_LINUX" = true ] || [ "$GEN_WINDOWS" = true ]; then SIZES+=("$s"); fi
      ;;
    *)
      SIZES+=("$s")
      ;;
  esac
done

echo "==> Rendering icon resolutions (${SIZES[*]}) from ${SOURCE_ICON}..."

for size in "${SIZES[@]}"; do
  out_png="${TMP_DIR}/icon-${size}.png"
  if command -v magick &>/dev/null; then
    magick -background none "${SOURCE_ICON}" -resize "${size}x${size}" "${out_png}"
  elif command -v convert &>/dev/null; then
    convert -background none "${SOURCE_ICON}" -resize "${size}x${size}" "${out_png}"
  elif command -v rsvg-convert &>/dev/null; then
    rsvg-convert -w "${size}" -h "${size}" -a -f png -o "${out_png}" "${SOURCE_ICON}"
  elif command -v inkscape &>/dev/null; then
    inkscape -w "${size}" -h "${size}" -o "${out_png}" "${SOURCE_ICON}"
  else
    echo "Error: No image rasterizer found (magick, convert, or rsvg-convert required)." >&2
    exit 1
  fi
done

# =============================================================================
# 1. Linux Assets (packaging/linux/assets)
# =============================================================================
if [ "$GEN_LINUX" = true ]; then
  echo "==> Generating Linux assets in packaging/linux/assets/..."
  HICOLOR_DIR="${LINUX_ASSETS}/icons/hicolor"
  mkdir -p "${HICOLOR_DIR}"

  for size in 16 24 32 48 64 128 256 512; do
    hicolor_target="${HICOLOR_DIR}/${size}x${size}/apps"
    mkdir -p "${hicolor_target}"
    cp "${TMP_DIR}/icon-${size}.png" "${hicolor_target}/mesh.png"
  done

  mkdir -p "${HICOLOR_DIR}/scalable/apps"
  if [[ "${SOURCE_ICON}" == *.svg ]]; then
    cp "${SOURCE_ICON}" "${HICOLOR_DIR}/scalable/apps/mesh.svg"
  fi

  cp "${TMP_DIR}/icon-256.png" "${LINUX_ASSETS}/mesh.png"
fi

# =============================================================================
# 2. macOS Assets (packaging/macos/assets)
# =============================================================================
if [ "$GEN_MACOS" = true ]; then
  echo "==> Generating macOS assets in packaging/macos/assets/..."
  mkdir -p "${MACOS_ASSETS}"
  python3 - "${MACOS_ASSETS}/mesh.icns" \
    "${TMP_DIR}/icon-16.png" \
    "${TMP_DIR}/icon-32.png" \
    "${TMP_DIR}/icon-64.png" \
    "${TMP_DIR}/icon-128.png" \
    "${TMP_DIR}/icon-256.png" \
    "${TMP_DIR}/icon-512.png" \
    "${TMP_DIR}/icon-1024.png" << 'EOF'
import struct
import sys
from pathlib import Path

ICNS_TYPES = {
    16: b'icp4',
    32: b'icp5',
    64: b'icp6',
    128: b'ic07',
    256: b'ic08',
    512: b'ic09',
    1024: b'ic10',
}

def create_icns(output_path, png_files):
    chunks = []
    for png_path in png_files:
        p = Path(png_path)
        if not p.exists():
            continue
        data = p.read_bytes()
        if len(data) >= 24 and data[:8] == b'\x89PNG\r\n\x1a\n':
            width, height = struct.unpack('>II', data[16:24])
            if width == height and width in ICNS_TYPES:
                chunk_type = ICNS_TYPES[width]
                chunk_len = 8 + len(data)
                chunks.append(chunk_type + struct.pack('>I', chunk_len) + data)

    if not chunks:
        raise ValueError("No matching square PNG files found for ICNS generation")

    total_len = 8 + sum(len(c) for c in chunks)
    header = b'icns' + struct.pack('>I', total_len)

    with open(output_path, 'wb') as f:
        f.write(header)
        for chunk in chunks:
            f.write(chunk)

    print(f"Created ICNS: {output_path} ({len(chunks)} resolutions)")

if __name__ == '__main__':
    if len(sys.argv) < 3:
        sys.exit(1)
    create_icns(sys.argv[1], sys.argv[2:])
EOF
fi

# =============================================================================
# 3. Windows Assets (packaging/windows/assets)
# =============================================================================
if [ "$GEN_WINDOWS" = true ]; then
  echo "==> Generating Windows assets in packaging/windows/assets/..."
  mkdir -p "${WINDOWS_ASSETS}"
  if command -v magick &>/dev/null; then
    magick "${TMP_DIR}/icon-16.png" \
           "${TMP_DIR}/icon-24.png" \
           "${TMP_DIR}/icon-32.png" \
           "${TMP_DIR}/icon-48.png" \
           "${TMP_DIR}/icon-64.png" \
           "${TMP_DIR}/icon-128.png" \
           "${TMP_DIR}/icon-256.png" \
           "${WINDOWS_ASSETS}/mesh.ico"
  elif command -v convert &>/dev/null; then
    convert "${TMP_DIR}/icon-16.png" \
            "${TMP_DIR}/icon-24.png" \
            "${TMP_DIR}/icon-32.png" \
            "${TMP_DIR}/icon-48.png" \
            "${TMP_DIR}/icon-64.png" \
            "${TMP_DIR}/icon-128.png" \
            "${TMP_DIR}/icon-256.png" \
            "${WINDOWS_ASSETS}/mesh.ico"
  fi
fi

echo "==> Done generating requested assets!"
