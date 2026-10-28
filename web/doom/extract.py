"""The Doom data the 3d view uses (fps.js and doom/), taken from Freedoom's IWAD into a small WAD of its own:
freedoom.wad here. Freedoom (freedoom.github.io) is a free replacement for Doom's data, under the 3-clause BSD
license (COPYING.txt beside this file). The view reads the same lumps from id's own DOOM.WAD, DOOM1.WAD or DOOM2.WAD
when one is put in ~/.config/burrow/ (server.py serves it as /doom.wad), so the names are Doom's.

    python3 web/doom/extract.py freedoom1.wad web/doom/freedoom.wad

What it keeps: the palette and the colormaps; the status bar, its numbers, the arms panel, the keys, the faces;
the shotgun and its flash; the HUD font; the sound effects the game and the office play (fps.js, level.js); the first level (E1M1) with every texture, flat and sprite it needs (and
the frames they animate through, the switches' other side, the sky); the office's walls, floor, ceiling and its
mysterious door. Wall textures are recomposed into one TEXTURE1 over a PNAMES of only the patches they use."""
import struct, sys

MAP = "E1M1"
GRAPHICS = (["PLAYPAL", "COLORMAP", "STBAR", "STARMS", "STTPRCNT", "STTMINUS"]
            + [f"STTNUM{i}" for i in range(10)] + [f"STYSNUM{i}" for i in range(10)] + [f"STGNUM{i}" for i in range(10)]
            + [f"STKEYS{i}" for i in range(6)]
            + [f"STFST{p}{k}" for p in range(5) for k in range(3)] + [f"STFTL{p}0" for p in range(5)] + [f"STFTR{p}0" for p in range(5)]
            + [f"STFOUCH{p}" for p in range(5)] + [f"STFEVL{p}" for p in range(5)] + [f"STFKILL{p}" for p in range(5)] + ["STFGOD0", "STFDEAD0"]
            + [f"STCFN{c:03d}" for c in range(33, 96)])
# the sound effects (Doom's DMX format: 8-bit samples at their rate), what fps.js and level.js play
SOUNDS = ["DSPISTOL", "DSSHOTGN", "DSPLPAIN", "DSPLDETH", "DSOOF", "DSNOWAY", "DSITEMUP", "DSWPNUP", "DSGETPOW", "DSTELEPT",
          "DSDOROPN", "DSDORCLS", "DSBDOPN", "DSBDCLS", "DSSWTCHN", "DSSWTCHX", "DSPSTART", "DSPSTOP", "DSSTNMOV",
          "DSBAREXP", "DSFIRSHT", "DSFIRXPL", "DSCLAW", "DSSGTATK", "DSSKLATK", "DSSLOP", "DSPUNCH",
          "DSPOSIT1", "DSPOSIT2", "DSPOSIT3", "DSPOSACT", "DSPOPAIN", "DSPODTH1", "DSPODTH2", "DSPODTH3",
          "DSBGSIT1", "DSBGSIT2", "DSBGACT", "DSBGDTH1", "DSBGDTH2", "DSSGTSIT", "DSSGTDTH", "DSDMPAIN", "DSDMACT",
          "DSCACSIT", "DSCACDTH", "DSBRSSIT", "DSBRSDTH", "DSSKLDTH"]
# the sprites (by their four letters): the shotgun, its flash, puffs and blood, and every thing level.js knows (keep in step with its T)
SPRITES = ["SHTG", "SHTF", "PUFF", "BLUD", "BEXP", "BAL1", "BAL2", "BAL7", "TFOG", "IFOG",
           "POSS", "SPOS", "TROO", "SARG", "SKUL", "HEAD", "BOSS", "BAR1",
           "STIM", "MEDI", "BON1", "BON2", "ARM1", "ARM2", "SOUL", "SHEL", "SBOX", "SHOT", "BPAK", "BKEY", "YKEY", "RKEY", "BSKU", "YSKU", "RSKU",
           "CLIP", "AMMO", "ROCK", "BROK", "CELL", "CELP", "MGUN", "LAUN", "PLAS", "CSAW", "BFUG", "PSTR", "PINV", "PINS", "SUIT", "PMAP", "PVIS",
           "COLU", "ELEC", "TRE1", "TRE2", "SMIT", "COL1", "COL2", "COL3", "COL4", "COL5", "COL6", "CEYE", "FSKU", "TBLU", "TGRN", "TRED",
           "SMBT", "SMGT", "SMRT", "CAND", "CBRA", "FCAN", "GOR1", "GOR2", "GOR3", "GOR4", "GOR5", "POL1", "POL2", "POL3", "POL4", "POL5", "POL6", "PLAY"]
OFFICE_TEXTURES = ["STARTAN2", "STARTAN3", "DOOR3", "DOORTRAK", "DOORBLU", "DOORYEL", "DOORRED", "SW1STRTN", "SW2STRTN", "BIGDOOR7", "SKY1"]
OFFICE_FLATS = ["FLAT5_2", "CEIL3_5", "TLITE6_4", "TLITE6_5", "TLITE6_6", "FLOOR4_8", "F_SKY1"]
ANIMS = [["NUKAGE1", "NUKAGE2", "NUKAGE3"], ["FWATER1", "FWATER2", "FWATER3", "FWATER4"], ["SWATER1", "SWATER2", "SWATER3", "SWATER4"],
         ["LAVA1", "LAVA2", "LAVA3", "LAVA4"], ["BLOOD1", "BLOOD2", "BLOOD3"], ["RROCK05", "RROCK06", "RROCK07", "RROCK08"],
         ["SLIME01", "SLIME02", "SLIME03", "SLIME04"], ["SLIME05", "SLIME06", "SLIME07", "SLIME08"], ["SLIME09", "SLIME10", "SLIME11", "SLIME12"],
         ["BLODGR1", "BLODGR2", "BLODGR3", "BLODGR4"], ["SLADRIP1", "SLADRIP2", "SLADRIP3"], ["BLODRIP1", "BLODRIP2", "BLODRIP3", "BLODRIP4"],
         ["FIREBLU1", "FIREBLU2"], ["ROCKRED1", "ROCKRED2", "ROCKRED3"], ["BFALL1", "BFALL2", "BFALL3", "BFALL4"], ["SFALL1", "SFALL2", "SFALL3", "SFALL4"],
         ["WFALL1", "WFALL2", "WFALL3", "WFALL4"], ["DBRAIN1", "DBRAIN2", "DBRAIN3", "DBRAIN4"], ["GSTFONT1", "GSTFONT2", "GSTFONT3"],
         ["FIRELAVA", "FIRELAV2"], ["FIREMAG1", "FIREMAG2", "FIREMAG3"]]


def directory(path):
    d = open(path, "rb").read()
    _, n, off = struct.unpack_from("<4sii", d, 0)
    out = []
    for i in range(n):
        pos, size, name = struct.unpack_from("<ii8s", d, off + 16 * i)
        out.append((name.rstrip(b"\0").decode("ascii", "replace").upper(), d[pos:pos + size]))
    return out


def textures(L):
    pn = L["PNAMES"]
    names = [pn[4 + 8 * i:12 + 8 * i].rstrip(b"\0").decode("ascii").upper() for i in range(struct.unpack_from("<i", pn, 0)[0])]
    out = {}
    for lump in ("TEXTURE1", "TEXTURE2"):
        d = L.get(lump)
        if not d:
            continue
        for i in range(struct.unpack_from("<i", d, 0)[0]):
            o = struct.unpack_from("<i", d, 4 + 4 * i)[0]
            name = d[o:o + 8].rstrip(b"\0").decode("ascii").upper()
            w, h = struct.unpack_from("<hh", d, o + 12)
            ps = [struct.unpack_from("<hhh", d, o + 22 + 10 * k) for k in range(struct.unpack_from("<h", d, o + 20)[0])]
            out.setdefault(name, (w, h, [(ox, oy, names[pi]) for ox, oy, pi in ps]))
    return out


def with_partners(names, known):
    out = list(dict.fromkeys(names))
    for seq in ANIMS:
        if any(n in out for n in seq):
            out += [n for n in seq if n not in out and n in known]
    for n in list(out):
        for a, b in (("SW1", "SW2"), ("SW2", "SW1")):
            if n.startswith(a) and b + n[3:] in known and b + n[3:] not in out:
                out.append(b + n[3:])
    return out


def main(src, dst):
    D = directory(src)
    L = {}
    for name, data in D:
        if data and name not in L:
            L[name] = data
    i = [k for k, (n, _) in enumerate(D) if n == MAP][0]
    maplumps = D[i:i + 11]
    side = dict(maplumps)["SIDEDEFS"]
    sectors = dict(maplumps)["SECTORS"]
    used_t = set()
    for k in range(len(side) // 30):
        for o in (4, 12, 20):
            t = side[30 * k + o:30 * k + o + 8].rstrip(b"\0").decode().upper()
            if t != "-":
                used_t.add(t)
    used_f = set()
    for k in range(len(sectors) // 26):
        for o in (4, 12):
            used_f.add(sectors[26 * k + o:26 * k + o + 8].rstrip(b"\0").decode().upper())
    tex = textures(L)
    keep_t = [t for t in with_partners(sorted(used_t) + OFFICE_TEXTURES, tex) if t in tex]
    flat_names = []
    on = False
    for name, data in D:
        if name in ("F_START", "FF_START"):
            on = True
        elif name in ("F_END", "FF_END"):
            on = False
        elif on and data:
            flat_names.append(name)
    keep_f = [f for f in with_partners(sorted(used_f) + OFFICE_FLATS, set(flat_names)) if f in L or f == "F_SKY1"]
    sprite_lumps = []
    on = False
    for name, data in D:
        if name in ("S_START", "SS_START"):
            on = True
        elif name in ("S_END", "SS_END"):
            on = False
        elif on and data and name[:4] in SPRITES and name not in sprite_lumps:
            sprite_lumps.append(name)
    patches = sorted({p for t in keep_t for _, _, p in tex[t][2] if p in L})
    pnames = struct.pack("<i", len(patches)) + b"".join(p.encode().ljust(8, b"\0") for p in patches)
    defs = []
    for t in keep_t:                                 # Doom's maptexture: name, masked, width, height, columndirectory, patchcount, patches
        w, h, ps = tex[t]
        ps = [(ox, oy, p) for ox, oy, p in ps if p in L]
        defs.append(t.encode().ljust(8, b"\0") + struct.pack("<ihhih", 0, w, h, 0, len(ps))
                    + b"".join(struct.pack("<hhhhh", ox, oy, patches.index(p), 1, 0) for ox, oy, p in ps))
    offs, pos = [], 4 + 4 * len(defs)
    for dd in defs:
        offs.append(pos)
        pos += len(dd)
    texture1 = struct.pack("<i", len(defs)) + b"".join(struct.pack("<i", o) for o in offs) + b"".join(defs)
    out = [(n, L[n]) for n in GRAPHICS + SOUNDS if n in L]
    out += [(n, d) for n, d in maplumps]
    out += [("PNAMES", pnames), ("TEXTURE1", texture1)]
    out += [("P_START", b"")] + [(p, L[p]) for p in patches] + [("P_END", b"")]
    out += [("S_START", b"")] + [(s, L[s]) for s in sprite_lumps] + [("S_END", b"")]
    out += [("F_START", b"")] + [(f, L.get(f, b"\0" * 4096)) for f in keep_f] + [("F_END", b"")]
    missing = [n for n in GRAPHICS + SOUNDS if n not in L] + [t for t in OFFICE_TEXTURES if t not in tex] + [t for t in used_t if t not in tex]
    body, dirs, off = b"", [], 12
    for name, data in out:
        dirs.append(struct.pack("<ii8s", off, len(data), name.encode()[:8].ljust(8, b"\0")))
        body += data
        off += len(data)
    open(dst, "wb").write(struct.pack("<4sii", b"PWAD", len(out), off) + body + b"".join(dirs))
    print(f"{dst}: {len(out)} lumps, {off + 16 * len(out)} bytes; {MAP}; textures {len(keep_t)}, patches {len(patches)}, flats {len(keep_f)}, "
          f"sprite lumps {len(sprite_lumps)}; missing {missing}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
