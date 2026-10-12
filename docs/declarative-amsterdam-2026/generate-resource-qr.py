"""Generate the resource QR with a locally bundled ReportLab QR encoder."""
import importlib.util
import os
from pathlib import Path
import struct
import zlib

root = Path(__file__).resolve().parents[2]
encoder = os.environ.get('CEM_QR_ENCODER', '/mnt/c/Users/suns/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/Lib/site-packages/reportlab/graphics/barcode/qrencoder.py')
spec = importlib.util.spec_from_file_location('qrencoder', encoder)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
qr = module.QRCode(None, module.QRErrorCorrectLevel.M)
qr.addData('https://github.com/EPA-WG/cem/discussions/24')
qr.make()
scale, quiet = 12, 4
size = (len(qr.modules) + 2 * quiet) * scale
scanlines = bytearray()
for y in range(size):
    scanlines.append(0)
    for x in range(size):
        row, col = y // scale - quiet, x // scale - quiet
        dark = 0 <= row < len(qr.modules) and 0 <= col < len(qr.modules) and qr.modules[row][col]
        scanlines.extend((22, 52, 59) if dark else (255, 255, 255))

def chunk(kind, data):
    return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))

png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!2I5B', size, size, 8, 2, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(scanlines)) + chunk(b'IEND', b'')
target = root / 'dist/declarative-amsterdam-2026/slides/resources-qr.png'
target.parent.mkdir(parents=True, exist_ok=True)
target.write_bytes(png)
print(target)
