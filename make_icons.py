import struct
import zlib
import os

def make_png(r, g, b, size=16):
    """Create a minimal valid PNG: size x size solid color."""
    # PNG signature
    sig = b'\x89PNG\r\n\x1a\n'

    # IHDR chunk: width, height, bit_depth=8, color_type=2 (RGB), compression=0, filter=0, interlace=0
    ihdr_data = struct.pack('>IIBBBBB', size, size, 8, 2, 0, 0, 0)
    ihdr = _chunk(b'IHDR', ihdr_data)

    # IDAT: raw image data compressed with zlib
    # Each row: filter byte (0) + RGB pixels
    row = bytes([0]) + bytes([r, g, b] * size)
    raw = row * size
    idat = _chunk(b'IDAT', zlib.compress(raw, 9))

    # IEND
    iend = _chunk(b'IEND', b'')

    return sig + ihdr + idat + iend

def _chunk(chunk_type, data):
    length = struct.pack('>I', len(data))
    crc = struct.pack('>I', zlib.crc32(chunk_type + data) & 0xffffffff)
    return length + chunk_type + data + crc

icons_dir = r'C:\Users\kkosiak.TITANPC\Desktop\Code\voice-to-text\src-tauri\icons'

# tray-idle.png: neutral grey
with open(os.path.join(icons_dir, 'tray-idle.png'), 'wb') as f:
    f.write(make_png(128, 128, 128))
print('Created tray-idle.png (grey)')

# tray-recording.png: solid red
with open(os.path.join(icons_dir, 'tray-recording.png'), 'wb') as f:
    f.write(make_png(220, 0, 0))
print('Created tray-recording.png (red)')

# tray-processing.png: solid orange/amber
with open(os.path.join(icons_dir, 'tray-processing.png'), 'wb') as f:
    f.write(make_png(255, 165, 0))
print('Created tray-processing.png (orange)')

print('All tray icons created successfully.')
