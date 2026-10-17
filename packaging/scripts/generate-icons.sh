#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

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

echo "==> Rendering icon resolutions from ${SOURCE_ICON}..."
SIZES=(16 24 32 48 64 128 256 512 1024)

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

# =============================================================================
# 2. macOS Assets (packaging/macos/assets)
# =============================================================================
echo "==> Generating macOS assets in packaging/macos/assets/..."
mkdir -p "${MACOS_ASSETS}"
python3 "${SCRIPT_DIR}/make_icns.py" "${MACOS_ASSETS}/mesh.icns" \
  "${TMP_DIR}/icon-16.png" \
  "${TMP_DIR}/icon-32.png" \
  "${TMP_DIR}/icon-64.png" \
  "${TMP_DIR}/icon-128.png" \
  "${TMP_DIR}/icon-256.png" \
  "${TMP_DIR}/icon-512.png" \
  "${TMP_DIR}/icon-1024.png"

# =============================================================================
# 3. Windows Assets (packaging/windows/assets)
# =============================================================================
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

echo "==> All OS-specific assets successfully generated in packaging/<os>/assets/!"
