import struct, sys, zlib
from pathlib import Path

def decode(path):
    data = Path(path).read_bytes()
    pos, idat, plte, trns = 8, b"", None, None
    while pos < len(data):
        n, kind = struct.unpack(">I4s", data[pos:pos + 8])
        body = data[pos + 8:pos + 8 + n]
        pos += 12 + n
        if kind == b"IHDR":
            w, h, depth, ctype, _, _, interlace = struct.unpack(">IIBBBBB", body)
        elif kind == b"PLTE":
            plte = body
        elif kind == b"tRNS":
            trns = body
        elif kind == b"IDAT":
            idat += body
    if depth != 8 or interlace:
        sys.exit(f"{path}: unsupported depth {depth} / interlace {interlace}")
    channels = {0: 1, 2: 3, 3: 1, 4: 2, 6: 4}[ctype]
    raw = zlib.decompress(idat)
    stride = w * channels
    rows, prev, i = [], bytearray(stride), 0
    for _ in range(h):
        f = raw[i]; line = bytearray(raw[i + 1:i + 1 + stride]); i += 1 + stride
        for x in range(stride):
            a = line[x - channels] if x >= channels else 0
            b = prev[x]
            c = prev[x - channels] if x >= channels else 0
            if f == 1: line[x] = (line[x] + a) & 255
            elif f == 2: line[x] = (line[x] + b) & 255
            elif f == 3: line[x] = (line[x] + (a + b) // 2) & 255
            elif f == 4:
                p = a + b - c; pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                line[x] = (line[x] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        rows.append(line); prev = line
    out = bytearray()
    for line in rows:
        for x in range(w):
            px = line[x * channels:(x + 1) * channels]
            if ctype == 6: out += px
            elif ctype == 2: out += px + b"\xff"
            elif ctype == 0: out += bytes([px[0]] * 3) + b"\xff"
            elif ctype == 4: out += bytes([px[0]] * 3) + px[1:2]
            else:
                k = px[0]; out += plte[k * 3:k * 3 + 3] + bytes([trns[k] if trns and k < len(trns) else 255])
    return w, h, out

src, dst = Path(sys.argv[1]), Path(sys.argv[2])
cell = 128
atlas = bytearray(cell * 4 * cell * 4)
for slot, name in enumerate(["cat_paw", "particle_glow", "particle_heart"]):
    w, h, px = decode(src / f"{name}.png")
    assert (w, h) == (cell, cell), name
    for y in range(cell):
        row = (y * cell * 4 + slot * cell) * 4
        atlas[row:row + cell * 4] = px[y * cell * 4:(y + 1) * cell * 4]
for y in range(cell):
    row = (y * cell * 4 + 3 * cell) * 4
    atlas[row:row + cell * 4] = b"\xff" * (cell * 4)
dst.parent.mkdir(parents=True, exist_ok=True)
dst.write_bytes(atlas)
print(f"wrote {dst} ({len(atlas)} bytes, {cell * 4}x{cell})")
