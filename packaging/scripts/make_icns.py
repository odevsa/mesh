#!/usr/bin/env python3
"""
Convert PNG images to Apple ICNS format without external dependencies.
"""
import struct
import sys
from pathlib import Path

# Mapping of dimensions to Apple ICNS chunk types
# Using PNG embedded format (supported in macOS 10.7+)
ICNS_TYPES = {
    16: b'icp4',
    32: b'icp5',
    64: b'icp6',
    128: b'ic07',
    256: b'ic08',
    512: b'ic09',
    1024: b'ic10',
}

def create_icns(png_files, output_path):
    chunks = []
    for png_path in png_files:
        p = Path(png_path)
        if not p.exists():
            continue
        data = p.read_bytes()
        # Parse PNG dimensions from IHDR chunk
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
        print("Usage: make_icns.py <output.icns> <input1.png> [input2.png ...]")
        sys.exit(1)
    create_icns(sys.argv[2:], sys.argv[1])
