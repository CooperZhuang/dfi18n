# 在巨型函数中搜索对 [rsp+0x7b20]/[rsp+0x6de0] 的引用及随后的绘制调用
import struct, sys
from capstone import Cs, CS_ARCH_X86, CS_MODE_64
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
def va_to_off(va):
    rva = va - image_base
    for vaddr, vsize, rawptr, rawsize in secs.values():
        if vaddr <= rva < vaddr + vsize:
            return rawptr + (rva - vaddr)
    return None
md = Cs(CS_ARCH_X86, CS_MODE_64)
md.detail = False

# 巨型函数 0x4988b0-0x4df174
FSTART = 0x1404988b0
FEND = 0x1404df174
off = va_to_off(FSTART)
code = data[off:off+(FEND-FSTART)]
insns = list(md.disasm(code, FSTART))
print("指令数:", len(insns))

# 找 lea rcx, [rsp+0x7b20] / [rsp+0x6de0] 之后的下一条 call
targets = {"[rsp + 0x7b20]": [], "[rsp + 0x6de0]": [], "[rsp + 0x6e00]": []}
for i, insn in enumerate(insns):
    for t in targets:
        if t in insn.op_str and insn.mnemonic in ("lea", "mov"):
            # 找之后5条内的 call
            for j in range(i+1, min(i+6, len(insns))):
                if insns[j].mnemonic == "call":
                    targets[t].append((insn.address, insns[j].address, insns[j].op_str))
                    break
for t, hits in targets.items():
    print(f"\n== {t} 引用后随的 call ({len(hits)}):")
    for a, ca, cop in hits[:15]:
        print(f"  lea@{a:x} -> call {ca:x} {cop}")
