import sys, struct, pefile, capstone, bisect
pe = pefile.PE(sys.argv[1], fast_load=True); base = pe.OPTIONAL_HEADER.ImageBase
pe.parse_data_directories([pefile.DIRECTORY_ENTRY['IMAGE_DIRECTORY_ENTRY_IMPORT']])
img = pe.get_memory_mapped_image()
iat = {}
for e in pe.DIRECTORY_ENTRY_IMPORT:
    for imp in e.imports:
        if imp.name: iat[imp.address] = imp.name.decode()
names = {}
try:
    for line in open(sys.argv[4], encoding='utf-8'):
        p = line.rstrip('\n').split('\t')
        if p[0].startswith('0x'): names[int(p[0], 16)] = p[1]
except Exception: pass
a, b = int(sys.argv[2], 16), int(sys.argv[3], 16)
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32); md.skipdata = True
import re
for ins in md.disasm(img[a - base:b - base], a):
    s = f'{ins.address:#x}: {ins.mnemonic:6} {ins.op_str}'
    notes = []
    for m in re.findall(r'0x[0-9a-f]+', ins.op_str):
        v = int(m, 16)
        if v in iat: notes.append(iat[v])
        if v in names: notes.append(names[v])
        if 0xb71000 + base - 0x400000 <= v < 0x1100000 + 0x400000 and 'ptr' in ins.op_str and ins.mnemonic.startswith(('f', 'mov', 'cvt', 'com', 'ucom', 'mul', 'add', 'sub', 'div')):
            off = v - base
            if 0 <= off < len(img) - 8:
                f = struct.unpack('<f', img[off:off + 4])[0]
                d = struct.unpack('<d', img[off:off + 8])[0]
                notes.append(f'f={f:.6g} d={d:.6g}')
    if ins.address in names: print(f'---- {names[ins.address]}')
    print(s + ('   ; ' + ', '.join(notes) if notes else ''))
