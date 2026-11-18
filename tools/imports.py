import sys, pefile, re
pe = pefile.PE(sys.argv[1])
pat = re.compile(sys.argv[2], re.I)
for e in pe.DIRECTORY_ENTRY_IMPORT:
    for imp in e.imports:
        n = (imp.name or b'').decode()
        if pat.search(n) or pat.search(e.dll.decode()):
            print(f'{e.dll.decode():20} {n:40} IAT={imp.address:#x}')
img = pe.get_memory_mapped_image(); base = pe.OPTIONAL_HEADER.ImageBase
for s in sys.argv[3:]:
    for enc in ('ascii', 'utf-16-le'):
        b = s.encode(enc); i = img.find(b)
        while i != -1:
            print(f'str {s!r} {enc} @ {base + i:#x}'); i = img.find(b, i + 1)
