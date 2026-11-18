import sys, struct, pefile, capstone
from collections import defaultdict

EXE = sys.argv[1]
pe = pefile.PE(EXE)
base = pe.OPTIONAL_HEADER.ImageBase
data = pe.get_memory_mapped_image()
text = [s for s in pe.sections if s.Name.startswith(b'.text')][0]
tstart = base + text.VirtualAddress
tcode = data[text.VirtualAddress:text.VirtualAddress + text.Misc_VirtualSize]

# Imports of interest
iat = {}
for e in pe.DIRECTORY_ENTRY_IMPORT:
    for imp in e.imports:
        if imp.name:
            iat[imp.address] = (e.dll.decode(), imp.name.decode())
want = ['Sleep', 'QueryPerformanceCounter', 'QueryPerformanceFrequency', 'timeBeginPeriod',
        'timeGetTime', 'GetTickCount', 'Direct3DCreate9', 'SleepEx', 'WaitForSingleObject']
iat_want = {a: n for a, (d, n) in iat.items() if n in want}
print('IAT:', {hex(a): n for a, n in iat_want.items()})

# constants
consts = {
    'f32 60.0': struct.pack('<f', 60.0),
    'f32 1/60': struct.pack('<f', 1/60),
    'f32 16.666': struct.pack('<f', 1000/60),
    'f64 1/60': struct.pack('<d', 1/60),
    'f64 60.0': struct.pack('<d', 60.0),
    'f64 16.666': struct.pack('<d', 1000/60),
    'f32 1/30': struct.pack('<f', 1/30),
    'f64 1/30': struct.pack('<d', 1/30),
}
for name, pat in consts.items():
    hits = []
    i = data.find(pat)
    while i != -1 and len(hits) < 40:
        hits.append(base + i)
        i = data.find(pat, i + 1)
    print(name, [hex(h) for h in hits])

md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.skipdata = True
calls = defaultdict(list)
refs = defaultdict(list)
watch = set(struct.unpack('<I', struct.pack('<I', a))[0] for a in iat_want)
for ins in md.disasm(tcode, tstart):
    s = ins.op_str
    for a in iat_want:
        if hex(a) in s:
            calls[iat_want[a]].append(ins.address)
print({k: [hex(x) for x in v[:30]] + ([f'...{len(v)}'] if len(v) > 30 else []) for k, v in calls.items()})
