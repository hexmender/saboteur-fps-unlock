import sys, pefile, re
pe = pefile.PE(sys.argv[1], fast_load=True); img = pe.get_memory_mapped_image(); base = pe.OPTIONAL_HEADER.ImageBase
pat = re.compile(sys.argv[2].encode(), re.I)
seen = set()
for m in re.finditer(rb'[\x20-\x7e]{4,}', img):
    s = m.group()
    if pat.search(s) and s not in seen:
        seen.add(s); print(f'{base + m.start():#x}: {s.decode()}')
