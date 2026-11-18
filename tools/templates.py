"""Parse GameTemplates.wsd (AULB) embedded in France/loosefiles_BinPC.pack and dump camera-related props."""
import sys, struct

def ph(s):
    h = 0x811C9DC5
    for c in s.encode():
        h = (((c | 0x20) ^ h) * 0x01000193) & 0xFFFFFFFF
    return ((h ^ 0x2A) * 0x01000193) & 0xFFFFFFFF
assert ph("ANY") == 0xED057225

def parse(buf, base):
    assert buf[base:base+4] == b'AULB'
    n = struct.unpack_from('<I', buf, base + 4)[0]
    p = base + 8; out = []
    for _ in range(n):
        if buf[p:p+12] == b'\x08' + b'\0' * 11:
            p += 12; continue
        total = struct.unpack_from('<I', buf, p)[0]; end = p + 4 + total
        q = p + 12
        nl = struct.unpack_from('<I', buf, q)[0]; name = buf[q+4:q+4+nl-1].decode('latin1'); q += 4 + nl
        tl = struct.unpack_from('<I', buf, q)[0]; typ = buf[q+4:q+4+tl-1].decode('latin1'); q += 4 + tl
        pc = struct.unpack_from('<I', buf, q)[0]; q += 4
        pairs = []
        for _ in range(pc):
            h, sz = struct.unpack_from('<II', buf, q)
            pairs.append((h, q + 8, buf[q+8:q+8+sz])); q += 8 + sz
        assert q == end, (name, q, end)
        out.append((name, typ, pairs)); p = end
    return out, p

if __name__ == '__main__':
    buf = open(sys.argv[1], 'rb').read()
    base = buf.find(b'AULB')
    tpls, endp = parse(buf, base)
    print('templates', len(tpls), 'AULB at', hex(base), 'end', hex(endp))
    names = ['CameraMoveTension', 'CameraAimTension', 'RotSpeed', 'FocusRotSpeed', 'PitchSpeed', 'Transition',
             'CameraTrackOffset', 'CameraTrackCenter', 'WallPushLength', 'fov', 'RestPitchWhenStill', 'RestingPitch',
             'FocusPitchSpeed', 'YawSpeed', 'Pitch1', 'Pitch2']
    hm = {ph(n): n for n in names}
    want = set(sys.argv[2:]) or {'CameraSettings', 'GroupCameraSettings'}
    for name, typ, pairs in tpls:
        if typ not in want: continue
        vals = []
        for h, off, d in pairs:
            if h in hm:
                if len(d) % 4 == 0 and len(d) <= 12:
                    v = struct.unpack('<%df' % (len(d) // 4), d)
                    vals.append(f"{hm[h]}={tuple(round(x, 3) for x in v)}")
                else:
                    vals.append(f"{hm[h]}={d.hex()}")
        print(f'[{typ}] {name}: ' + '; '.join(vals))
