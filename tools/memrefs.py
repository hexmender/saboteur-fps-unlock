import sys, struct, pefile, capstone
pe = pefile.PE(sys.argv[1], fast_load=True); base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
for t in [int(x, 16) for x in sys.argv[2:]]:
    pat = struct.pack('<I', t); i = img.find(pat, 0x1000, 0xB71000); out = []
    while i != -1:
        best = None
        for back in range(8, 0, -1):
            st = i - back
            ins = next(md.disasm(img[st:st + 16], base + st, 1), None)
            if ins and st + ins.size >= i + 4 and hex(t) in ins.op_str and 'ptr' in ins.op_str:
                best = ins; break
        if best: out.append(f'{best.address:#x}: {best.mnemonic} {best.op_str}')
        else: out.append(f'raw@{base+i:#x}')
        i = img.find(pat, i + 1, 0xB71000)
    print(hex(t), out)
# float search
for name, v in [('1/240', 1/240), ('1/120', 1/120), ('1/144', 1/144)]:
    p = struct.pack('<f', v); j = img.find(p); print(name, hex(base + j) if j != -1 else None)
