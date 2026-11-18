"""In-place patch: raise CameraMoveTension / CameraAimTension components below MIN to MIN in all camera templates."""
import sys, struct
sys.path.insert(0, __import__('os').path.dirname(__file__))
from templates import parse, ph

path = sys.argv[1]; MIN = float(sys.argv[2]) if len(sys.argv) > 2 else 1.0
buf = bytearray(open(path, 'rb').read())
base = buf.find(b'AULB'); assert base > 0 and buf.find(b'AULB', base + 1) == -1
tpls, _ = parse(bytes(buf), base)
keys = {ph('CameraMoveTension'): 'Move', ph('CameraAimTension'): 'Aim'}
n = 0
for name, typ, pairs in tpls:
    if 'CameraSettings' not in typ: continue
    for h, off, d in pairs:
        if h in keys and len(d) == 12:
            v = struct.unpack('<3f', d)
            nv = tuple(max(x, MIN) for x in v)
            if nv != v:
                struct.pack_into('<3f', buf, off, *nv); n += 1
                print(f'{name:40} {keys[h]:4} {tuple(round(x,3) for x in v)} -> {nv}')
open(path, 'wb').write(buf)
print('patched pairs:', n)
