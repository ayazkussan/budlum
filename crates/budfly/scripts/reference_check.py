#!/usr/bin/env python3
"""BudFly reference implementation & provenance validator.

This is the Python mirror that *froze* generator v1.0, the dynamics
constants, and the golden anchors used by `tests/goldens.rs`. The Rust crate
is a line-by-line port; this script is its oracle. Run:

    python3 scripts/reference_check.py

Expect: scenario prints and `REFERENCE CHECK PASS` — it recomputes everything
and asserts the goldens committed in the Rust test suite. Generator v1.0 is
frozen: edit this file only when cutting a new generator version.
"""
#!/usr/bin/env python3
"""BudFly reference validator: mirrors the Rust crate's numeric semantics 1:1.

Goal: freeze constants + dynamics that the Rust port will copy verbatim.
Everything integer-only, Q4.12 fixed point (SCALE=4096), saturating.
"""
import hashlib, sys
from collections import defaultdict

SCALE = 4096          # Q4.12
V_TH  = SCALE         # spike threshold = 1.0 (Q4.12 4096)
V_MAX = SCALE * 16    # clamp ceiling
V_MIN = -SCALE * 8    # clamp floor
REFRAC = 1            # refractory ticks
W_SYN  = 512          # weight quantum per synapse count (1/8, Q4.12)
I_STIM = SCALE * 4    # injected stimulus current (4.0)
LEAK_SHIFT = 4        # v -= v >> 4  (x15/16)

# ---------------- RNG: splitmix64 / xorshift (mirrored in Rust) -------------
class Rng:
    def __init__(self, seed): self.s = seed & 0xFFFFFFFFFFFFFFFF
    def next(self):  # splitmix64
        self.s = (self.s + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.s
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return z ^ (z >> 31)
    def below(self, n):  # uniform in [0, n)
        return self.next() % n

# ---------------- Connectome generator --------------------------------------
# Region ids
LAM_L, LAM_R, MED_L, MED_R, LOB_L, LOB_R = 0,1,2,3,4,5
CX_EB, CX_FB, CX_INH, MB_KC, MB_MBON, MB_APL, LH, DN_L, DN_R, MDN = range(6,16)
REGION_NAMES = {0:"lamina_L",1:"lamina_R",2:"medulla_L",3:"medulla_R",4:"lobula_L",
    5:"lobula_R",6:"central_complex_EB_ring",7:"central_complex_FB",8:"cx_inhibitory",
    9:"mushroom_body_KC",10:"mbon",11:"apl",12:"lateral_horn",13:"DNa02_like_L",
    14:"DNa02_like_R",15:"MDN_like"}
BASE = {LAM_L:512, LAM_R:512, MED_L:1024, MED_R:1024, LOB_L:512, LOB_R:512,
        CX_EB:64, CX_FB:128, CX_INH:8, MB_KC:2000, MB_MBON:10, MB_APL:1,
        LH:256, DN_L:8, DN_R:8, MDN:4}

def build_connectome(scale_num, scale_den, seed=0xB0DF17):
    # motif-scoped RNG streams: adding/tuning one motif never reshuffles the others
    rv = Rng(seed ^ 0xA11CE)   # visual pathway lamina->medulla->lobula
    rc = Rng(seed ^ 0xC0FFEE)  # central complex  FB/EB/INH compass
    rm = Rng(seed ^ 0xDEC0DE)  # mushroom body    KC/MBON/APL
    ro = Rng(seed ^ 0xF00D)    # output scaffold  LH/DN/MDN
    FLOOR = {CX_EB: 48, CX_FB: 64, CX_INH: 6, MB_MBON: 8, MB_APL: 1,
             DN_L: 6, DN_R: 6, MDN: 4, LH: 64}
    sizes, off, total = {}, {}, 0
    for r in range(16):
        n = max(FLOOR.get(r, 8), BASE[r] * scale_num // scale_den)
        sizes[r], off[r] = n, total
        total += n
    # edges: (pre_gid, post_gid, weight_q12)  weight<0 => inhibitory
    edges = []
    def add(pre_region, i, post_region, j, count, inhib=False):
        w = (max(1, count) * W_SYN) >> 0
        if inhib: w = -w
        edges.append((off[pre_region]+i, off[post_region]+j, w))
    for side, lam, med, lob in ((0, LAM_L, MED_L, LOB_L), (1, LAM_R, MED_R, LOB_R)):
        nl, nm, nb = sizes[lam], sizes[med], sizes[lob]
        for i in range(nl):  # lamina -> medulla: 2:1 + lateral jitter
            for t in range(2):
                j = (i*2 + t) % nm
                add(lam, i, med, j, 3 + rv.below(8))
            if rv.below(4) == 0:
                add(lam, i, med, (i*2 + 2 + rv.below(7)) % nm, 1 + rv.below(3))
        for jm in range(nm):  # medulla -> lobula: convergent 3->1, small cross-talk
            for _ in range(2):
                src = rv.below(nm)
                add(med, src, lob, (jm * nb // nm) % nb, 2 + rv.below(6))
                if rv.below(10) == 0:
                    add(med, src, lob, rv.below(nb), 1 + rv.below(3))
    # lobula -> LH, FB, and a visual PN role into mushroom body
    for side, lob in ((0, LOB_L), (1, LOB_R)):
        for i in range(sizes[lob]):
            # interleaved indexed coverage: every LH and FB neuron is reached
            add(lob, i, LH, ((i * sizes[LH]) // sizes[lob] + side) % sizes[LH], 2 + ro.below(5))
            add(lob, i, LH, ro.below(sizes[LH]), 1 + ro.below(4))
            add(lob, i, CX_FB, ((i * sizes[CX_FB]) // sizes[lob] + side) % sizes[CX_FB], 1 + rc.below(4))
            add(lob, i, CX_FB, rc.below(sizes[CX_FB]), 1 + rc.below(3))
            if rm.below(4) < 1:  # ~25% are projection neurons
                for _ in range(1 + rm.below(3)):
                    add(lob, i, MB_KC, rm.below(sizes[MB_KC]), 1 + rm.below(4))
    # FB -> EB ring (convergent bump seeding), EB ring local excitation, global inh
    for k in range(sizes[CX_FB]):
        for _ in range(2):
            add(CX_FB, k, CX_EB, rc.below(sizes[CX_EB]), 3 + rc.below(5))
    ne = sizes[CX_EB]
    for i in range(ne):  # ring: excite neighbours, weighted by proximity
        for d, c in ((1, 10), (2, 8), (3, 5), (4, 3)):
            add(CX_EB, i, CX_EB, (i+d) % ne, c)
            add(CX_EB, i, CX_EB, (i-d) % ne, c)
        add(CX_EB, i, CX_INH, i % sizes[CX_INH], 4)
    for i in range(sizes[CX_INH]):
        for j in range(ne):
            add(CX_INH, i, CX_EB, j, 3, inhib=True)
    # EB -> FB recurrence (keeps bump alive), sparse
    for i in range(ne):
        if rc.below(2) == 0:
            add(CX_EB, i, CX_FB, rc.below(sizes[CX_FB]), 2 + rc.below(4))
    # every KC receives at least 3 PN inputs (combinatorial code, no deaf KCs)
    pns = [i for i in range(sizes[LOB_L]) ] + [sizes[LOB_L] + i for i in range(sizes[LOB_R])]
    lob_span = sizes[LOB_L] + sizes[LOB_R]
    for k in range(sizes[MB_KC]):
        for _ in range(3):
            pick = pns[rm.below(lob_span)]
            lr = LOB_L if pick < sizes[LOB_L] else LOB_R
            add(lr, pick - (0 if lr == LOB_L else sizes[LOB_L]), MB_KC, k, 1 + rm.below(3))
    # MB: KC -> MBON convergent; KC -> APL; APL -> KC inhibitory
    for k in range(sizes[MB_KC]):
        add(MB_KC, k, MB_MBON, rm.below(sizes[MB_MBON]), 1 + rm.below(2))
        if rm.below(3) == 0:
            add(MB_KC, k, MB_APL, 0, 2)
    for k in range(0, sizes[MB_KC], 4):
        add(MB_APL, 0, MB_KC, k, 1, inhib=True)
    # FB compass state feeds the MDN veto channel (cross-channel reference)
    for k in range(sizes[CX_FB]):
        if rc.below(3) == 0:
            add(CX_FB, k, MDN, ro.below(sizes[MDN]), 2 + ro.below(3))
    # LH + MBON -> descending command pools (left/right lateralized)
    for i in range(sizes[LH]):
        tgt = DN_L if i % 2 == 0 else DN_R
        add(LH, i, tgt, ro.below(sizes[tgt]), 3 + ro.below(6))
    for i in range(sizes[MB_MBON]):
        add(MB_MBON, i, DN_L, ro.below(sizes[DN_L]), 2 + ro.below(4))
        add(MB_MBON, i, DN_R, ro.below(sizes[DN_R]), 2 + ro.below(4))
    # mutual inhibition L<->R, MDN as cross-channel veto
    for i in range(sizes[DN_L]): add(DN_L, i, DN_R, ro.below(sizes[DN_R]), 2, inhib=True)
    for i in range(sizes[DN_R]): add(DN_R, i, DN_L, ro.below(sizes[DN_L]), 2, inhib=True)
    for i in range(sizes[MDN]):
        add(MDN, i, DN_L, ro.below(sizes[DN_L]), 2, inhib=True)
        add(MDN, i, DN_R, ro.below(sizes[DN_R]), 2, inhib=True)
    return sizes, off, total, edges

# ---------------- Fabric: neuromorphic mesh model ---------------------------
class Chip:
    def __init__(self, n_cap=256, syn_sram_bytes=64*1024, sop_per_cycle=64):
        self.n_cap = n_cap
        self.syn_cap = syn_sram_bytes // 8      # 8B per synapse entry
        self.sop_per_cycle = sop_per_cycle
    def place(self, total, edges):
        ncore = (total + self.n_cap - 1) // self.n_cap
        core_of = [i // self.n_cap for i in range(total)]
        local = defaultdict(int)
        fanout = defaultdict(set)
        for p, q, w in edges:
            local[core_of[q]] += 1
            fanout[core_of[p]].add(core_of[q])
        overflow = [c for c, s in local.items() if s > self.syn_cap]
        return ncore, core_of, local, fanout, overflow

def mesh_dims(n):
    x = 1
    while x * x < n: x += 1
    return x, (n + x - 1) // x

E_HOP_PJ, E_SOP_PJ, E_NEURON_PJ = 26, 24, 900  # order-of-magnitude, documented

# ---------------- Simulation ------------------------------------------------
def run(sizes, off, total, edges, ticks, stimuli, chip=None, audit_rows=False):
    """stimuli: dict gid -> (start_tick, end_tick); inject I_STIM each tick in window.
    Returns (anchors, spikes_per_region, trace_rows)."""
    adj = [[] for _ in range(total)]
    for p, q, w in edges:
        adj[p].append((q, w))
    v = [0] * total
    refr = [0] * total
    anchor = b"\x00" * 32
    pend = [0] * total
    region_spikes = defaultdict(int)
    rows = defaultdict(list) if audit_rows else None
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        for gid in range(total):
            i_ext = I_STIM if (gid in stimuli and stimuli[gid][0] <= t < stimuli[gid][1]) else 0
            i_syn = 0
            # buffered input delivered this tick (computed from prev tick spikes)
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            spike = 0
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                leaked = vb - (vb >> LEAK_SHIFT)
                raw = leaked + i_ext + i_syn
                raw = max(V_MIN, min(V_MAX, raw))
                if raw >= V_TH:
                    spike = 1
                    v[gid] = 0
                    refr[gid] = REFRAC
                else:
                    v[gid] = raw
            if spike:
                spiking.append(gid)
                sbytes[gid] = 1
            if audit_rows:
                rows[gid].append((t, vb, i_ext, i_syn, v[gid], spike, rb))
        # deliver to next tick (1-tick synaptic delay => clean trace semantics)
        pend = [0] * total
        for gid in spiking:
            for q, w in adj[gid]:
                pend[q] = max(-SCALE*64, min(SCALE*64, pend[q] + w))
        region_of = {}
        for r, o in off.items():
            for i in range(sizes[r]): region_of[o+i] = r
        for gid in spiking: region_spikes[region_of[gid]] += 1
        sh = hashlib.sha256(bytes(sbytes)).digest()
        vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh).digest()
    return anchor.hex(), dict(region_spikes), rows

# ---------------- AIR-style constraint checker -------------------------------
def check_trace(rows):
    """The executable specification of the tick circuit (mirrors src/air.rs):
    C1 leak: leaked = v_before - (v_before>>4); C2 accum&clamp; C3 threshold/spike;
    C4 reset; C5 refractory hold; C6 refractory chain (strengthened rule, also
    enforced on the Rust side); C7 genesis state."""
    bad = 0
    for gid, rs in rows.items():
        for i, (t, vb, ie, isy, va, sp, rb) in enumerate(rs):
            if i == 0:                                    # C7
                if vb != 0 or rb != 0: bad += 1; continue
            else:                                         # C6
                p_t, p_vb, p_ie, p_isy, p_va, p_sp, p_rb = rs[i-1]
                expected = (p_rb - 1) if p_rb > 0 else (REFRAC if p_sp == 1 else 0)
                if rb != expected: bad += 1; continue
            if rb > 0:  # C5
                if va != 0 or sp != 0: bad += 1; continue
            else:
                leaked = vb - (vb >> LEAK_SHIFT)          # C1
                raw = max(V_MIN, min(V_MAX, leaked + ie + isy))  # C2
                expect_sp = 1 if raw >= V_TH else 0
                if sp != expect_sp: bad += 1; continue    # C3
                if expect_sp and va != 0: bad += 1; continue  # C4
                if not expect_sp and va != raw: bad += 1; continue  # C2
    return bad

# ---------------- Fabric cycle/energy model ---------------------------------
def fabric_report(total, edges, spike_counts_total, chip):
    ncore, core_of, local, fanout, overflow = chip.place(total, edges)
    mx, my = mesh_dims(ncore)
    # route stats: for each edge, hops between cores; packets collapse by (src,dst) per spike
    edge_hops = 0
    packets = set()
    for p, q, w in edges:
        a, b = core_of[p], core_of[q]
        ax, ay = a % mx, a // mx
        bx, by = b % mx, b // mx
        edge_hops += abs(ax-bx) + abs(ay-by)
        packets.add((a, b))
    syn_total = len(edges)
    per_spike_packets = len(packets) and (sum(len(v) for v in fanout.values()) / max(1, ncore))
    return dict(cores=ncore, mesh=(mx, my), synapses=syn_total,
                syn_per_core_max=max(local.values()) if local else 0,
                syn_per_core_avg=(syn_total // max(1, ncore)),
                sram_overflow_cores=len(overflow),
                avg_hops_per_edge=round(edge_hops / max(1, syn_total), 2),
                packets_per_spike_avg=round(per_spike_packets, 1),
                energy_per_tick_pJ=(spike_counts_total * E_SOP_PJ * 4 +
                                    total * E_NEURON_PJ // 1000))

# ---------------- scenarios ---------------------------------------------------

# ---------------- frozen goldens (must match crates/budfly tests) -----------
RING64_ANCHOR = "aedfd9426fd3d9f0e79afa1a889cb7bfb88e1e7a8cc0c33b3561718ea127aea3"
SENTINEL_GOLDENS = [
    (bytes(32), "Abstain", 0, 0, 0, "50f5f2ab0f7342fb412c7b781ea3b9dee5f45160a8a5a22ace3283b784d18173"),
    (bytes([0xFF]*32), "Affirm", 7, 6, 1, "398e126a88776b4f15f7da9bfb7bd927f47fc32e8a92128a354380ce8d429cc3"),
    (hashlib.sha256(b"budlum-genesis").digest(), "Affirm", 5, 3, 6, "ac4e8042233658c31ac154f7ee7fd7ee0707959373e057f60743e41b68ae0eb4"),
    (bytes(range(32)), "Reject", 1, 3, 0, "01b3ff076fa7850317ff1883e18b58f1e17d77dd2efaa12eb24b8db943699cb2"),
]

def sentinel(sizes, off, edges, digest: bytes, ticks=48):
    total = sum(sizes.values())
    rng = Rng(int.from_bytes(digest[:8], "little"))
    eb0, ne = off[CX_EB], sizes[CX_EB]
    sector_start = int.from_bytes(digest[8:10], "little") % ne
    width = max(4, ne // 4)
    stim = {}
    for i in range(width):
        stim[eb0 + (sector_start + i) % ne] = (0, 8)
    for side_region in (LOB_L, LOB_R):
        base, n = off[side_region], sizes[side_region]
        for b in range(32):
            if digest[16 + b // 8] >> (b % 8) & 1:
                g = base + rng.below(n)
                if g not in stim: stim[g] = (0, 4)
    anchor, spk, _ = run(sizes, off, total, edges, ticks, stim)
    dl, dr, mdn = spk.get(DN_L, 0), spk.get(DN_R, 0), spk.get(MDN, 0)
    if mdn > 0 and mdn > 3 * max(dl, dr): verdict = "Abstain"
    elif dl > dr: verdict = "Affirm"
    elif dr > dl: verdict = "Reject"
    else: verdict = "Abstain"
    return verdict, dl, dr, mdn, anchor

def main():
    scale = (1, 16)
    sizes, off, total, edges = build_connectome(*scale)
    assert (total, len(edges)) == (588, 2283), "generator v1.0 shape drifted"
    eb0, ne = off[CX_EB], sizes[CX_EB]
    stim = {eb0 + i: (0, 8) for i in range(ne // 4)}
    anchor, spk, rows = run(sizes, off, total, edges, 64, stim, audit_rows=True)
    assert anchor == RING64_ANCHOR, f"ring anchor drifted: {anchor}"

    import math
    counts = [0] * ne
    for g, rs in rows.items():
        if eb0 <= g < eb0 + ne:
            counts[g - eb0] = sum(r[5] for r in rs[32:])
    sx = sum(counts[i] * math.cos(2*math.pi*i/ne) for i in range(ne))
    sy = sum(counts[i] * math.sin(2*math.pi*i/ne) for i in range(ne))
    ang = (math.degrees(math.atan2(sy, sx)) + 360) % 360
    target = 360 * ((ne/8) / ne)
    err = min((ang - target) % 360, (target - ang) % 360)
    assert err < 45 and sum(counts) > 0, f"ring attractor lost: err={err}"
    print(f"[ring] bump angle={ang:.1f} target={target:.1f} err={err:.1f} eb_spikes={sum(counts)}")

    assert check_trace(rows) == 0, "honest trace must satisfy all constraints"
    gid0 = next(iter(rows))
    t, vb, ie, isy, va, sp, rb = rows[gid0][1]
    rows[gid0][1] = (t, vb, ie, isy, va, 1 - sp, rb)
    assert check_trace(rows) > 0, "tamper must be caught"
    print("[air] honest=0 violations, tampered caught")

    for digest, verdict, dl, dr, mdn, want in SENTINEL_GOLDENS:
        got = sentinel(sizes, off, edges, digest)
        assert got == (verdict, dl, dr, mdn, want), f"sentinel drift: {got}"
    print(f"[oracle] {len(SENTINEL_GOLDENS)} goldens reproduce")

    chip = Chip()
    rep16 = fabric_report(total, edges, 500, chip)
    assert (rep16["cores"], rep16["mesh"], rep16["sram_overflow_cores"]) == (3, (2, 2), 0)
    print(f"[fabric] 1/16 placement {rep16}")
    print("REFERENCE CHECK PASS")

if __name__ == "__main__":
    main()
