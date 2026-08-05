import struct, sys
P = r"C:/Program Files (x86)/Steam/steamapps/common/Dwarf Fortress/Dwarf Fortress.exe"
data = open(P, "rb").read()
pe = struct.unpack_from("<I", data, 0x3C)[0]
opt = pe + 24
nsec = struct.unpack_from("<H", data, pe+6)[0]
opt_size = struct.unpack_from("<H", data, pe+20)[0]
sec_off = opt + opt_size
image_base = 0x140000000
secs = {}
for i in range(nsec):
    s = sec_off + i*40
    name = data[s:s+8].rstrip(b"\0").decode()
    vsize, vaddr, rawsize, rawptr = struct.unpack_from("<IIII", data, s+8)
    secs[name] = (vaddr, vsize, rawptr, rawsize)
tvaddr, tvsize, trawptr, trawsize = secs[".text"]
pd = secs.get(".pdata")
# 函数范围
funcs = []
if pd:
    for off in range(pd[2], pd[2]+pd[3], 12):
        b, e, u = struct.unpack_from("<III", data, off)
        if b: funcs.append((b, e))
def enclosing(va):
    rva = va - image_base
    for b, e in funcs:
        if b <= rva < e: return b, e
    return None, None

target = int(sys.argv[1], 16)
tdata = data[trawptr:trawptr+trawsize]
callers = []
i = 0
n = len(tdata)
while i < n - 5:
    b = tdata[i]
    if b in (0xE8, 0xE9):
        rel = struct.unpack_from("<i", tdata, i+1)[0]
        insn_va = image_base + tvaddr + i
        tgt = insn_va + 5 + rel
        if tgt == target:
            f = enclosing(insn_va)
            callers.append((insn_va, "call" if b == 0xE8 else "jmp", f))
        i += 5
    else:
        i += 1
print(f"0x{target:x} 的调用者 ({len(callers)}):")
for va, kind, f in callers:
    fr = f"函数 0x{f[0]:x}-0x{f[1]:x}" if f[0] else "?"
    print(f"  {kind} @ 0x{va:x}  ({fr})")
