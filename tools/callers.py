import sys, struct, pefile
pe = pefile.PE(sys.argv[1], fast_load=True); base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
for tgt in [int(x, 16) for x in sys.argv[2:]]:
    hits = []
    for i in range(0x1000, 0xB71000 - 5):
        if img[i] in (0xE8, 0xE9):
            rel = struct.unpack_from('<i', img, i + 1)[0]
            if base + i + 5 + rel == tgt:
                hits.append(base + i)
    # also pointer refs (vtables)
    pat = struct.pack('<I', tgt); j = img.find(pat); ptrs = []
    while j != -1:
        ptrs.append(base + j); j = img.find(pat, j + 1)
    print(hex(tgt), 'calls:', [hex(h) for h in hits], 'ptrs:', [hex(p) for p in ptrs])
