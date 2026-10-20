#!/usr/bin/env python3
"""Test-only execution of the retail VectorNormalSS instruction range."""
import hashlib
import struct
import sys
from pathlib import Path

exe = Path(sys.argv[1]).read_bytes()
assert hashlib.sha256(exe).hexdigest() == "0690a5c14ff3e975ebcb3de26e196a4dbb6afc992677d0de2cbcf86af9993558"
memory = bytearray(0x200000)
base = struct.unpack_from("<I", exe, 0x18)[0] & 0x1FFFFFFF
memory[base:base + len(exe) - 0x800] = exe[0x800:]


def signed(value, bits=32):
    value &= (1 << bits) - 1
    return value - (1 << bits) if value & (1 << (bits - 1)) else value


def run(vector):
    memory[0x1000:0x1006] = vector
    registers = [0] * 32
    gte = [0] * 32
    registers[4:6] = [0x80001000, 0x80001100]
    registers[31] = 0xFFFFFFFF
    pc, following = 0x80077B04, 0x80077B08
    for _ in range(160):
        if pc == 0xFFFFFFFF:
            return memory[0x1100:0x1106]
        assert 0x80077B04 <= pc < 0x80077BF4
        word = struct.unpack_from("<I", memory, pc & 0x1FFFFFFF)[0]
        op, rs, rt, rd = word >> 26, (word >> 21) & 31, (word >> 16) & 31, (word >> 11) & 31
        immediate = word & 0xFFFF
        displacement = signed(immediate, 16)
        after = (following + 4) & 0xFFFFFFFF
        if op == 0:
            function, shift = word & 63, (word >> 6) & 31
            if function == 0:
                registers[rd] = registers[rt] << shift
            elif function == 3:
                registers[rd] = signed(registers[rt]) >> shift
            elif function == 4:
                registers[rd] = registers[rt] << (registers[rs] & 31)
            elif function == 7:
                registers[rd] = signed(registers[rt]) >> (registers[rs] & 31)
            elif function == 8:
                after = registers[rs]
            elif function in (0x20, 0x21):
                registers[rd] = registers[rs] + registers[rt]
            elif function == 0x22:
                registers[rd] = registers[rs] - registers[rt]
            elif function == 0x24:
                registers[rd] = registers[rs] & registers[rt]
            else:
                raise AssertionError((hex(pc), hex(word)))
        elif op == 1 and rt == 0:
            if signed(registers[rs]) < 0:
                after = pc + 4 + (displacement << 2)
        elif op == 3:
            registers[31] = pc + 8
            after = ((pc + 4) & 0xF0000000) | ((word & 0x3FFFFFF) << 2)
        elif op == 4:
            if registers[rs] == registers[rt]:
                after = pc + 4 + (displacement << 2)
        elif op in (8, 9):
            registers[rt] = registers[rs] + displacement
        elif op == 15:
            registers[rt] = immediate << 16
        elif op == 18:
            if rs == 0:
                registers[rt] = gte[rd]
            elif rs == 4:
                value = registers[rt]
                if rd == 30:
                    magnitude = value ^ (0xFFFFFFFF if value & 0x80000000 else 0)
                    gte[31] = 32 - magnitude.bit_length()
                else:
                    assert rd in (8, 9, 10, 11)
                    gte[rd] = signed(value, 16)
            elif word == 0x4AA00428:
                for i in range(3):
                    gte[25 + i] = signed(gte[9 + i] * gte[9 + i])
            elif word == 0x4B90003D:
                for i in range(3):
                    gte[25 + i] = signed(gte[8] * gte[9 + i])
            else:
                raise AssertionError((hex(pc), hex(word)))
        elif op in (0x21, 0x29):
            address = (registers[rs] + displacement) & 0x1FFFFFFF
            assert address + 2 <= len(memory)
            if op == 0x29:
                struct.pack_into("<H", memory, address, registers[rt] & 0xFFFF)
            else:
                registers[rt] = struct.unpack_from("<h", memory, address)[0]
        else:
            raise AssertionError((hex(pc), hex(word)))
        registers = [r & 0xFFFFFFFF for r in registers]
        registers[0] = 0
        pc, following = following, after
    raise AssertionError("VectorNormalSS did not return")


data = sys.stdin.buffer.read()
assert len(data) % 6 == 0
for offset in range(0, len(data), 6):
    sys.stdout.buffer.write(run(data[offset:offset + 6]))
