def vn(val):
    if val < 0:
        raise ValueError
    buf = b""
    while val >> 7:
        m = val & 0xFF | 0x80
        buf += m.to_bytes(1, "big")
        val >>= 7
    buf += val.to_bytes(1, "big")
    return buf


def tp(a, b, ary):
    return vn((b << 3) | a) + ary


def rs(a, ary):
    if isinstance(ary, str):
        ary = ary.encode()
    return tp(2, a, vn(len(ary)) + ary)


def nm(a, ary):
    return tp(0, a, vn(ary))
