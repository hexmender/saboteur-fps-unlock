import sys, pefile, capstone, re
pe = pefile.PE(sys.argv[1], fast_load=True); img = pe.get_memory_mapped_image(); base = pe.OPTIONAL_HEADER.ImageBase
lo, hi = int(sys.argv[2], 16), int(sys.argv[3], 16)
offs = [int(x, 16) for x in sys.argv[4:]]
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32); md.skipdata = True
pats = [re.compile(r'\[e[a-z]{2} \+ ' + hex(o) + r'\]') for o in offs]
seen = set()
for c in range(lo, hi, 0x8000):
    for ins in md.disasm(img[c - base:min(c + 0x8040, hi) - base], c):
        if ins.address >= c + 0x8000: break
        s = ins.op_str
        if ins.address not in seen and any(p.search(s) for p in pats):
            seen.add(ins.address); print(f'{ins.address:#x}: {ins.mnemonic} {s}')

