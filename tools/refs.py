import sys, struct, pefile, capstone
pe = pefile.PE(sys.argv[1]); base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
targets = {int(x, 16): x for x in sys.argv[2:]}
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
for t in targets:
    pat = struct.pack('<I', t)
    i = img.find(pat, 0x1000, 0xB71000)
    out = []
    while i != -1:
        # find instruction containing this operand: try starts 1..6 bytes before
        cands = []
        for back in range(1, 9):
            st = i - back
            ins = next(md.disasm(img[st:st + 16], base + st, 1), None)
            if ins and ins.size >= back + 4 and hex(t) in ins.op_str:
                cands.append((('ptr [' in ins.op_str), back, ins))
        if cands:
            cands.sort(key=lambda c: (c[0], -c[1]), reverse=True)
            ins = cands[0][2]
            out.append(f'{ins.address:#x}: {ins.mnemonic} {ins.op_str}')
        i = img.find(pat, i + 1, 0xB71000)
    print(hex(t), len(out))
    for o in out[:60]:
        print('  ', o)

