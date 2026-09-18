#!/usr/bin/env python3
"""BudFly expansion probes — Python reference that FREEZES the numbers the
Rust expansion modules (analysis/lesion/compass/learn/fault) must reproduce.

Same discipline as reference_check.py: every PIN line below is recomputed
here from the frozen dynamics AND cross-checked against goldens.anchor.toml
(one table, twice). Run from crates/budfly/scripts:

    python3 expansion_check.py

Five probes over connectome v1.0:
  [census]  graph census: degree sums + BFS geodesics sensory->command
  [lesion]  ablation battery: silence a region, pin the behavioral deficit
  [compass] bump wander: the ring's bump does NOT lock (pinned negative)
  [learn]   mushroom-body trace-gated conditioned suppression (works)
  [fault]   analytic many-core fault table at MaleCNS scale
"""
import hashlib
from collections import defaultdict, deque
from pathlib import Path
import tomllib

from reference_check import (
    SCALE, V_TH, V_MAX, V_MIN, REFRAC, W_SYN, I_STIM, LEAK_SHIFT,
    LAM_L, LAM_R, LOB_L, LOB_R, CX_EB, CX_FB, MB_KC, MB_MBON, MB_APL,
    DN_L, DN_R, MDN,
    Rng, build_connectome, run, sentinel,
)

# Mirrors Rust sim.rs (per-add progressive clamp during delivery). The
# reference runner omits the clamp because exercised workloads never reach
# it; the expansion runners clamp exactly like Rust so new probes that
# raise fan-in stay bit-identical across languages.
FAN_IN_MAX = SCALE * 64

RESULTS = {}
def pin(tag, *vals):
    RESULTS[tag] = " ".join(str(v) for v in vals)
    print(f"PIN[{tag}] {RESULTS[tag]}")

sizes, off, total, edges = build_connectome(1, 16)
assert (total, len(edges)) == (588, 2283)
region_of = [0] * total
for r in range(16):
    for g in range(off[r], off[r] + sizes[r]):
        region_of[g] = r

# --------------------------------------------------------------- [census]
adj_out = [[] for _ in range(total)]
for p, q, _w in edges:
    adj_out[p].append(q)
out_deg = {r: 0 for r in range(16)}
in_deg = {r: 0 for r in range(16)}
for p, q, _w in edges:
    out_deg[region_of[p]] += 1
    in_deg[region_of[q]] += 1

def bfs_dist(src):
    d = [-1] * total
    dq = deque()
    for g in range(off[src], off[src] + sizes[src]):
        d[g] = 0
        dq.append(g)
    while dq:
        g = dq.popleft()
        for q in adj_out[g]:
            if d[q] < 0:
                d[q] = d[g] + 1
                dq.append(q)
    return d

def pool_stats(d, region):
    ds = [d[g] for g in range(off[region], off[region] + sizes[region]) if d[g] >= 0]
    return (len(ds), min(ds), sum(ds) * 10 // len(ds), max(ds))

dL = bfs_dist(LAM_L)
census_str = "|".join(f"{r}:{out_deg[r]}:{in_deg[r]}" for r in range(16))
pin("census.reached_from_laml", sum(1 for x in dL if x >= 0))
pin("census.dnl", *pool_stats(dL, DN_L))
pin("census.dnr", *pool_stats(dL, DN_R))
pin("census.mdn", *pool_stats(dL, MDN))
pin("census.degsum_sha256", hashlib.sha256(census_str.encode()).hexdigest())

# --------------------------------------------------------------- [lesion]
def run_masked(ticks, stimuli, dead, audit_rows=False):
    """Identical dynamics to reference run(); `dead` gids never update nor
    fire, and no current flows in or out of them. Anchor shape unchanged."""
    adj = [[] for _ in range(total)]
    for p, q, w in edges:
        if not dead[p] and not dead[q]:
            adj[p].append((q, w))
    v = [0] * total
    refr = [0] * total
    pend = [0] * total
    anchor = b"\x00" * 32
    region_spikes = defaultdict(int)
    rows = defaultdict(list) if audit_rows else None
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        for gid in range(total):
            if dead[gid]:
                continue
            i_ext = I_STIM if (gid in stimuli and stimuli[gid][0] <= t < stimuli[gid][1]) else 0
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            spike = 0
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                raw = max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + i_ext + i_syn))
                if raw >= V_TH:
                    v[gid] = 0
                    refr[gid] = REFRAC
                    spike = 1
                    spiking.append(gid)
                    sbytes[gid] = 1
                else:
                    v[gid] = raw
            if audit_rows:
                rows[gid].append((t, vb, i_ext, i_syn, v[gid], spike, rb))
        nxt = [0] * total
        for p in spiking:
            for q, w in adj[p]:
                nxt[q] = max(-FAN_IN_MAX, min(FAN_IN_MAX, nxt[q] + w))
        for g in spiking:
            region_spikes[region_of[g]] += 1
        pend = nxt
        sh = hashlib.sha256(bytes(sbytes)).digest()
        vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh).digest()
    return anchor.hex(), region_spikes, rows

def odor_stim():
    """Canonical odor probe: left lamina bump ticks 0..8, fixed PN pattern
    into every other KC ticks 8..16 (the 'smell')."""
    st = {}
    for i in range(sizes[LAM_L] // 4):
        st[off[LAM_L] + i] = (0, 8)
    for i in range(sizes[MB_KC] // 2):
        st[off[MB_KC] + i * 2] = (8, 16)
    return st

def dead_region(r):
    dm = [False] * total
    for g in range(off[r], off[r] + sizes[r]):
        dm[g] = True
    return dm

def spk_row(spk):
    return ",".join(str(spk.get(r, 0)) for r in range(16))

dead_none = [False] * total
ba, bs, _ = run_masked(48, odor_stim(), dead_none)
pin("lesion.base.anchor", ba)
pin("lesion.base.spikes", spk_row(bs))
for name, r in (("no_apl", MB_APL), ("no_eb", CX_EB), ("no_mdn", MDN)):
    a, s, _ = run_masked(48, odor_stim(), dead_region(r))
    pin(f"lesion.{name}.spikes", spk_row(s))
    pin(f"lesion.{name}.anchor", a)
    pin(f"lesion.{name}.mbon_delta", s.get(MB_MBON, 0) - bs.get(MB_MBON, 0))

# --------------------------------------------------------------- [compass]
# v4 FROZEN PROTOCOL (honest negative pinned): 6-wide bump at sector 0 for
# ticks 0..8, then 64 ticks of silence. Question: does the bump hold its
# sector per tick? Measured answer: NO (drift/wander), pinned quantitatively,
# matching the live histogram dump; the summed-angle golden (reference_check)
# remains the coarse statistic that historically concentrated.
def compass_wander(bump_width=6, ticks=72):
    stim = {off[CX_EB] + i: (0, 8) for i in range(bump_width)}
    anchor, spk, rows = run_masked(ticks, stim, dead_none, audit_rows=True)
    ne = sizes[CX_EB]
    sw = ne // 16
    per = defaultdict(lambda: defaultdict(int))
    for g, rs in rows.items():
        if off[CX_EB] <= g < off[CX_EB] + ne:
            for r in rs:
                if r[5] == 1:
                    per[r[0]][(g - off[CX_EB]) // sw] += 1
    dec = []
    for t in range(ticks):
        bk = per.get(t)
        if bk and max(bk.values()) > 0:
            # ties -> lowest sector index (mirrors python max with (count, -b))
            best_c = max(bk.values())
            dec.append(min(b for b, c in bk.items() if c == best_c))
        else:
            dec.append(-1)
    after = range(8, ticks)
    hold0 = sum(1 for t in after if dec[t] == 0)
    anysp = sum(1 for t in after if dec[t] >= 0)
    t40 = sorted(per[40].items())
    hist = ",".join(f"{b}:{c}" for b, c in t40)
    pin("compass.v4.anchor", anchor)
    pin("compass.v4.hold_sector0_of64", hold0)
    pin("compass.v4.any_spike_of64", anysp)
    pin("compass.v4.t40_histogram", hist)
    pin("compass.v4.verdict", "bump wanders; single-sector lock absent")

compass_wander()

# ----------------------------------------------------------------- [learn]
# v3 FROZEN PROTOCOL (works): CS = every even KC, ticks 8..24; US = MDN pool,
# ticks 12..20. Three-factor trace gate: MDN spiked at t or t-1 (1-tick US
# trace), pre spiked at t-1, post spiked at t; KC->MBON decremented
# delta=128 down to floor=128, edge order = generation order, delivery for
# t+1 uses UPDATED weights.
def cond_run(cs_ticks=(8, 24), us_ticks=(12, 20), ticks=32, delta=SCALE // 32,
             floor_w=W_SYN // 4):
    stim = {off[MB_KC] + i: cs_ticks for i in range(0, sizes[MB_KC], 2)}
    for i in range(sizes[MDN]):
        stim[off[MDN] + i] = us_ticks
    adj_idx = [[] for _ in range(total)]
    W = [w for (_p, _q, w) in edges]
    for i, (p, q, _w) in enumerate(edges):
        adj_idx[p].append((q, i))
    v = [0] * total
    refr = [0] * total
    pend = [0] * total
    anchor = b"\x00" * 32
    mbon = []
    prev = set()
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        for gid in range(total):
            i_ext = I_STIM if (gid in stim and stim[gid][0] <= t < stim[gid][1]) else 0
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                raw = max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + i_ext + i_syn))
                if raw >= V_TH:
                    v[gid] = 0
                    refr[gid] = REFRAC
                    spiking.append(gid)
                    sbytes[gid] = 1
                else:
                    v[gid] = raw
        ss = set(spiking)
        mbon.append(sum(1 for g in spiking if region_of[g] == MB_MBON))
        gate = any(region_of[g] == MDN for g in ss) or any(region_of[g] == MDN for g in prev)
        if gate:
            for i, (p, q, _w) in enumerate(edges):
                if region_of[p] == MB_KC and region_of[q] == MB_MBON \
                        and p in prev and q in ss:
                    W[i] = max(floor_w, W[i] - delta)
        nxt = [0] * total
        for p in spiking:
            for q, i in adj_idx[p]:
                nxt[q] = max(-FAN_IN_MAX, min(FAN_IN_MAX, nxt[q] + W[i]))
        pend = nxt
        prev = ss
        sh = hashlib.sha256(bytes(sbytes)).digest()
        vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh).digest()
    # canonical class binding (learn-v2): KC->MBON edges only, generation
    # order, (pre_offset, post_offset, final_w) LE triples - robust against
    # whole-list layout questions, still generation-ordered by construction.
    cw = [(p - off[MB_KC], q - off[MB_MBON], W[i])
          for i, (p, q, _w) in enumerate(edges)
          if region_of[p] == MB_KC and region_of[q] == MB_MBON]
    learn_anchor = hashlib.sha256(
        b"learn-v2" + b"".join(
            p.to_bytes(4, "little") + q.to_bytes(4, "little")
            + w.to_bytes(4, "little", signed=True)
            for p, q, w in cw)).hexdigest()
    wc = [W[i] for i, (p, q, _w) in enumerate(edges)
          if region_of[p] == MB_KC and region_of[q] == MB_MBON]
    changed = sum(1 for i, (p, q, w) in enumerate(edges)
                  if region_of[p] == MB_KC and region_of[q] == MB_MBON and W[i] != w)
    pin("learn.v3.mbon_per_tick", ",".join(map(str, mbon)))
    pin("learn.v3.changed", changed)
    pin("learn.v3.floor_min", min(wc))
    pin("learn.v3.ceil_max", max(wc))
    pin("learn.v3.pre_mbon_8_12", sum(mbon[8:12]))
    pin("learn.v3.post_mbon_20_24", sum(mbon[20:24]))
    pin("learn.v3.run_anchor", anchor.hex())
    pin("learn.v3.learn_anchor", learn_anchor)

cond_run()

# ----------------------------------------------------------------- [fault]
def fault_table():
    MALE_N, MALE_C, ACTIVE_PCT = 166_700, 25_582_938, 2
    N_CAP, SRAM, SOP_CYC, CLK = 256, 64 * 1024, 64, 1_000_000_000
    E_SOP, E_HOP, E_UPD = 24, 26, 900
    cores_by_n = -(-MALE_N // N_CAP)
    cores_by_s = -(-(MALE_C * 8) // SRAM)
    cores = max(cores_by_n, cores_by_s)
    active = MALE_N * ACTIVE_PCT // 100
    fan = MALE_C // MALE_N
    sops = active * fan
    energy = active * E_SOP + active * 4 * E_HOP + MALE_N * E_UPD // 1000
    for k in (0, 1, 8, 64, 256, 1024):
        ok = cores - k
        per_core = -(-sops // ok)
        cyc = -(-per_core // SOP_CYC)
        tps = CLK // cyc if cyc else 0
        pin(f"fault.k{k}", k, ok, cyc, tps, energy)

fault_table()

# ---------------------------------------------------------------- [dispute]
# Anchor-bisection dispute game: an executor submits per-tick chain heads;
# the validator reruns honestly, locates the FIRST divergent tick, and the
# C1-C7 transition constraints arbitrate that single tick. The dishonest
# fixture (frozen recipe): at tick 23 the liar flips the spike bit of
# (CxEb offset 3) and, being lazy, reports all-zero membrane bytes at that
# tick; the chain is folded onwards normally.
def anchor_log_run(ticks, stim, tamper=None):
    adj = [[] for _ in range(total)]
    for p, q, w in edges:
        adj[p].append((q, w))
    v = [0] * total
    refr = [0] * total
    pend = [0] * total
    anchor = b"\x00" * 32
    log = []
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        for gid in range(total):
            i_ext = I_STIM if (gid in stim and stim[gid][0] <= t < stim[gid][1]) else 0
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                raw = max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + i_ext + i_syn))
                if raw >= V_TH:
                    v[gid] = 0
                    refr[gid] = REFRAC
                    spiking.append(gid)
                    sbytes[gid] = 1
                else:
                    v[gid] = raw
        nxt = [0] * total
        for p in spiking:
            for q, w in adj[p]:
                nxt[q] = max(-FAN_IN_MAX, min(FAN_IN_MAX, nxt[q] + w))
        pend = nxt
        if tamper is not None and t == tamper:
            gid = off[CX_EB] + 3
            sbytes[gid] = 1 - sbytes[gid]
            vh = hashlib.sha256(b"\x00" * (total * 4)).digest()
        else:
            vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        sh = hashlib.sha256(bytes(sbytes)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh).digest()
        log.append(anchor.hex())
    return log

def odor_stim_local():
    st = {}
    for i in range(sizes[LAM_L] // 4):
        st[off[LAM_L] + i] = (0, 8)
    for i in range(sizes[MB_KC] // 2):
        st[off[MB_KC] + i * 2] = (8, 16)
    return st

H = anchor_log_run(48, odor_stim_local())
F = anchor_log_run(48, odor_stim_local(), tamper=23)
div = next((t for t in range(48) if H[t] != F[t]), None)
pin("dispute.honest_final", H[-1])
pin("dispute.dishonest_final", F[-1])
pin("dispute.divergence_tick", div)
import math
pin("dispute.bisect_queries_max", math.ceil(math.log2(48)))

# ---------------------------------------------------------------- [reflex]
stim_l = {off[LAM_L] + i: (0, 8) for i in range(sizes[LAM_L] // 4)}
_a, _s, rows_l = run_masked(48, stim_l, dead_none, audit_rows=True)
RN = ["LamL", "LamR", "MedL", "MedR", "LobL", "LobR", "CxEb", "CxFb",
      "CxInh", "MbKc", "MbMbon", "MbApl", "Lh", "DnL", "DnR", "Mdn"]
first = {}
for r in range(16):
    ts = [rr[0] for g in range(off[r], off[r] + sizes[r]) for rr in rows_l[g] if rr[5] == 1]
    first[r] = min(ts) if ts else -1
pin("reflex.first_spike_tick", ",".join(f"{RN[r]}:{first[r]}" for r in range(16)))
pin("reflex.latency_dnl", first[DN_L])
pin("reflex.latency_mdn", first[MDN])
pin("reflex.dnr_silent", first[DN_R] == -1 and first[MB_MBON] == -1)

# ---------------------------------------------------------------- [canary]
# 1-tick challenge-response canary: cheap integrity heartbeat pinned to the
# expensive contract through the SAME log path (self-consistency asserted).
vg, dl, dr, mdn, sent_anchor = sentinel(sizes, off, edges, bytes([0xFF] * 32))
rng_stim_check = bytes([0xFF] * 32)
# sentinel all-ones stimulus reproduced via the same builder on this side:
s_stim = {}
_r = Rng(int.from_bytes(rng_stim_check[0:8], "little"))
_eb0, _ne = off[CX_EB], sizes[CX_EB]
_sector = int.from_bytes(rng_stim_check[8:10], "little") % _ne
_width = max(4, _ne // 4)
for _i in range(_width):
    s_stim[_eb0 + (_sector + _i) % _ne] = (0, 8)
for _lob in (LOB_L, LOB_R):
    _base, _n = off[_lob], sizes[_lob]
    for _b in range(32):
        if (rng_stim_check[16 + _b // 8] >> (_b % 8)) & 1:
            _gid = _base + _r.below(_n)
            if _gid not in s_stim:
                s_stim[_gid] = (0, 4)

def anchor_log_generic(ticks, stim):
    adj = [[] for _ in range(total)]
    for p, q, w in edges:
        adj[p].append((q, w))
    v = [0] * total
    refr = [0] * total
    pend = [0] * total
    anchor = b"\x00" * 32
    log = []
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        for gid in range(total):
            i_ext = I_STIM if (gid in stim and stim[gid][0] <= t < stim[gid][1]) else 0
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                raw = max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + i_ext + i_syn))
                if raw >= V_TH:
                    v[gid] = 0
                    refr[gid] = REFRAC
                    spiking.append(gid)
                    sbytes[gid] = 1
                else:
                    v[gid] = raw
        nxt = [0] * total
        for p in spiking:
            for q, w in adj[p]:
                nxt[q] = max(-FAN_IN_MAX, min(FAN_IN_MAX, nxt[q] + w))
        pend = nxt
        sh = hashlib.sha256(bytes(sbytes)).digest()
        vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh).digest()
        log.append(anchor.hex())
    return log

clog = anchor_log_generic(48, s_stim)
assert clog[-1] == sent_anchor, "canary log path must land on the sentinel golden"
pin("canary.allones.tick1", clog[0])
pin("canary.allones.chain_end", clog[-1])
pin("canary.self_consistent", clog[-1] == sent_anchor)


def sentinel_stim(digest):
    """The one builder (draw order frozen): compass sector, then LobL, LobR."""
    st = {}
    r = Rng(int.from_bytes(digest[0:8], "little"))
    eb0, ne = off[CX_EB], sizes[CX_EB]
    sector = int.from_bytes(digest[8:10], "little") % ne
    width = max(4, ne // 4)
    for i in range(width):
        st[eb0 + (sector + i) % ne] = (0, 8)
    for lob in (LOB_L, LOB_R):
        base, n = off[lob], sizes[lob]
        for b in range(32):
            if (digest[16 + b // 8] >> (b % 8)) & 1:
                g = base + r.below(n)
                if g not in st:
                    st[g] = (0, 4)
    return st

def verdict_of(dl, dr, mdn):
    if mdn > 0 and mdn > 3 * max(dl, dr):
        return "Abstain"
    if dl > dr:
        return "Affirm"
    if dr > dl:
        return "Reject"
    return "Abstain"

# ---------------------------------------------------------------- [replay]
# Sealed replay certificate: the base chain is published; a counterfactual
# branch forks the STIMULUS at FORK while dynamics stay deterministic. The
# certificate's spine: branch prefix must byte-match the published prefix —
# cheap on-chain check (no re-execution), expensive fraud (re-execution of
# one branch only). Frozen question: "what if MDN fired at tick 10?".
FORK = 10
base_log = anchor_log_generic(48, sentinel_stim(bytes([0xFF] * 32)))
vB, Bdl, Bdr, Bmdn, sent_anchor2 = sentinel(sizes, off, edges, bytes([0xFF] * 32))
assert base_log[-1] == sent_anchor2, "log end must seal the sentinel verdict"

cf = dict(sentinel_stim(bytes([0xFF] * 32)))
for i in range(sizes[MDN]):
    cf[off[MDN] + i] = (FORK, FORK + 2)  # MDN veto arrives at tick 10
branch_log = anchor_log_generic(48, cf)
assert branch_log[:FORK] == base_log[:FORK], "prefix must bind before the fork"
cf_anchor, cf_spk, _ = run(sizes, off, total, edges, 48, cf)
assert cf_anchor == branch_log[-1], "run() final anchor must seal the log end"
Cdl, Cdr, Cmdn = cf_spk.get(DN_L, 0), cf_spk.get(DN_R, 0), cf_spk.get(MDN, 0)
pin("replay.fork_tick", FORK)
pin("replay.base_anchor_at_fork", base_log[FORK - 1])
pin("replay.prefix_bound", branch_log[:FORK] == base_log[:FORK])
pin("replay.base_counters", f"{Bdl},{Bdr},{Bmdn}")
pin("replay.base_verdict", verdict_of(Bdl, Bdr, Bmdn))
pin("replay.mdn10_branch_final", branch_log[-1])
pin("replay.mdn10_counters", f"{Cdl},{Cdr},{Cmdn}")
pin("replay.mdn10_verdict", verdict_of(Cdl, Cdr, Cmdn))

# ----------------------------------------------------------------- [divan]
# The Fly Court: three jurors = three seat-salted probes of ONE fact digest.
# Draw-order discipline: one builder per (builder, seat) — the seat salt is
# folded into a fresh digest, never into a second draw of the base stream.
# Council rule (fails closed): anything but unanimity settles as Abstain.
D0 = bytes([0x42] * 32)
def seat_digest(d, seat):
    return hashlib.sha256(d + bytes([seat])).digest()
seat_v, seat_a = [], []
for s in range(3):
    v, dl, dr, mdn, a = sentinel(sizes, off, edges, seat_digest(D0, s))
    seat_v.append(v)
    seat_a.append(a)
    pin(f"divan.seat{s}_verdict", v)
    pin(f"divan.seat{s}_anchor", a)
unanimous = seat_v[0] == seat_v[1] == seat_v[2]
pin("divan.unanimous", unanimous)
pin("divan.council", seat_v[0] if unanimous else "Abstain")
# Dishonest-juror expulsion: seat 2 submits a chain that is lazy at tick 23
# (spike-bit flip on CxEb+3, all-zero membrane bytes). The bisection game
# locates the lie; the seat is expelled. An honest rerun arbitrates.
seat2_log_honest = anchor_log_generic(48, sentinel_stim(seat_digest(D0, 2)))
seat2_log_liar = anchor_log_run(48, sentinel_stim(seat_digest(D0, 2)), tamper=23)
expel = next(t for t in range(48) if seat2_log_honest[t] != seat2_log_liar[t])
pin("divan.liar_seat", 2)
pin("divan.liar_divergence", expel)
pin("divan.liar_final", seat2_log_liar[-1])
pin("divan.seat2_honest_final", seat2_log_honest[-1])

# ------------------------------------------------------------- [tournament]
# Two seeds, same ring, same panel: survival = decisiveness. The settlement
# layer pays for a council that can say something; an abstention machine is
# dead weight. Referee = the divan of each fly over a frozen 8-digest panel.
sizesB, offB, totalB, edgesB = build_connectome(1, 16, seed=0xB0DF18)
assert sizesB == sizes and offB == off and totalB == total, "same anatomy, new wiring"
panel = [hashlib.sha256(b"budfly-panel" + bytes([i])).digest() for i in range(8)]
def council_abstains(sizes_, off_, edges_, digest):
    vs = [sentinel(sizes_, off_, edges_, seat_digest(digest, s))[0] for s in range(3)]
    return not (vs[0] == vs[1] == vs[2])
abA = sum(council_abstains(sizes, off, edges, d) for d in panel)
abB = sum(council_abstains(sizesB, offB, edgesB, d) for d in panel)
pin("tournament.panel", 8)
pin("tournament.abstains_a", abA)
pin("tournament.abstains_b", abB)
pin("tournament.survivor", "A" if abA < abB else ("B" if abB < abA else "both"))
# Arbitration sanity: the deciding run is re-executable under dispute rules
# (bisection machinery is fly-agnostic; it sees chains, not champions).


# --------------------------------------------------------------- [envelope]
# BSE-1 "BudFly Settlement Envelope": the 136-byte court verdict — the one
# artifact a settlement layer stores instead of re-running anything.
# Layout (frozen):
#   "BSE1"            4 B
#   fact_digest      32 B
#   seat anchors   3*32 B
#   seat verdicts  3*1  B   (0=Abstain, 1=Affirm, 2=Reject)
#   council        1    B
# Cheap checks (no fly execution): magic, explicit length, council rule
# re-applied to the seat codes must reproduce the council byte.
VCODE = {"Abstain": 0, "Affirm": 1, "Reject": 2}
def encode_bse1(digest, seat_anchors, seat_verdicts, council):
    b = b"BSE1" + digest + b"".join(bytes.fromhex(a) for a in seat_anchors)
    b += bytes([VCODE[v] for v in seat_verdicts]) + bytes([VCODE[council]])
    assert len(b) == 136
    return b
def decode_bse1(b):
    assert len(b) == 136 and b[:4] == b"BSE1"
    return {"digest": b[4:36], "anchors": [b[36+i*32:68+i*32].hex() for i in range(3)],
            "verdicts": list(b[132:135]), "council": b[135]}
env = encode_bse1(D0, seat_a, seat_v, "Abstain")
dec = decode_bse1(env)
unanimous_codes = dec["verdicts"][0] == dec["verdicts"][1] == dec["verdicts"][2]
dec_council = dec["verdicts"][0] if unanimous_codes else 0
pin("envelope.len", len(env))
pin("envelope.hex", env.hex())
pin("envelope.cheap_consistent", dec_council == dec["council"])
pin("envelope.selfcheck", dec["anchors"] == seat_a and dec["verdicts"] == [VCODE[v] for v in seat_v])


# ----------------------------------------------------------- [envelope bse2]
# BSE-2: the verdict PLUS its counterfactual card, 204 bytes.
# Layout (frozen):
#   "BSE2"            4 B
#   fact_digest      32 B
#   seat anchors   3*32 B
#   seat verdicts  3*1  B
#   council        1    B
#   fork_tick      4    B (little-endian u32; 0 = no card)
#   base_head@fork-1 32 B (zero bytes when fork == 0)
#   branch_final     32 B
# Cheap native check: fork > 0  =>  the card exists; the verifier's cheap
# replay check is one 32-byte compare against the PUBLISHED base head at
# fork-1 (wherever that base is committed: chain, manifest, or the BSE-1
# lineage of the same fact). No execution, no hashing.
FF = bytes([0xFF] * 32)
ff_seat_v, ff_seat_a = [], []
for s in range(3):
    v, dl, dr, mdn, a = sentinel(sizes, off, edges, seat_digest(FF, s))
    ff_seat_v.append(v)
    ff_seat_a.append(a)
ff_unan = ff_seat_v[0] == ff_seat_v[1] == ff_seat_v[2]
ff_council = ff_seat_v[0] if ff_unan else "Abstain"
pin("envelope2.ff_seat_verdicts", ",".join(ff_seat_v))
pin("envelope2.ff_unanimous", ff_unan)
pin("envelope2.ff_council", ff_council)

def encode_bse2(digest, seat_anchors, seat_verdicts, council, fork, base_head_hex, branch_hex):
    card = fork.to_bytes(4, "little")
    card += (bytes(32) if fork == 0 else bytes.fromhex(base_head_hex))
    card += (bytes(32) if fork == 0 else bytes.fromhex(branch_hex))
    b = b"BSE2" + digest + b"".join(bytes.fromhex(a) for a in seat_anchors)
    b += bytes([VCODE[v] for v in seat_verdicts]) + bytes([VCODE[council]]) + card
    assert len(b) == 204
    return b
b2 = encode_bse2(FF, ff_seat_a, ff_seat_v, ff_council, FORK,
                 base_log[FORK - 1], branch_log[-1])
pin("envelope2.len", len(b2))
pin("envelope2.hex", b2.hex())
pin("envelope2.fork", FORK)
pin("envelope2.cheap_replay", b2[136:140] == FORK.to_bytes(4, "little")
    and b2[140:172].hex() == base_log[FORK - 1]
    and b2[172:204].hex() == branch_log[-1])
# BSE-1 stays a strict sub-case: fork == 0 => card bytes are zeros.
b2_nocard = encode_bse2(FF, ff_seat_a, ff_seat_v, ff_council, 0, "00" * 32, "00" * 32)
pin("envelope2.nocard_tail", b2_nocard[136:].hex())


# ------------------------------------------------------------------- [act]
# ACT-1 "Arbitration Check Tape": the one-tick fraud-proof, verifiable
# WITHOUT the connectome. A bisection lands on a disputed tick T; the honest
# executor answers with rows for T-1 and T (the two-tick window the C1-C7
# constraint catalogue needs: C6 glues T-1 to T, genesis only if T<=1).
#
# Layout (frozen):
#   "ACT1"        4 B
#   tick_t       u32 LE 4 B
#   row_count    u32 LE 4 B
#   rows: for tick in (T-1, T), for gid in 0..total:
#         v_before i32 LE | i_ext i32 LE | i_syn i32 LE |
#         v_after  i32 LE | r_before u32 LE | spike u8     (21 B/row)
#
# Two independent kill switches, checked without re-running anything:
#   (i)  constraint kill: evaluate C1-C7 over the window rows (air mirror)
#   (ii) fold kill: recompute head(T) = sha256(prev_head | T be8 |
#        sha256(spike_bytes) | sha256(v_after LE)) and compare to the
#        published chain head. Forgery fails (i), (ii), or both.
T = 23
ff_stim = sentinel_stim(bytes([0xFF] * 32))
A, spkA, rowsA = run(sizes, off, total, edges, 48, ff_stim, audit_rows=True)
ff_log = anchor_log_generic(48, ff_stim)

def act_rows_bytes(rows_map, ticks_sel):
    buf = bytearray()
    n = 0
    for t in ticks_sel:
        for gid in range(total):
            _t, vb, i_ext, i_syn, va, sp, rb = rows_map[gid][t]
            buf += vb.to_bytes(4, "little", signed=True)
            buf += i_ext.to_bytes(4, "little", signed=True)
            buf += i_syn.to_bytes(4, "little", signed=True)
            buf += va.to_bytes(4, "little", signed=True)
            buf += rb.to_bytes(4, "little", signed=False)
            buf += bytes([sp])
            n += 1
    return bytes(buf), n

def act_encode(rows_map, t):
    sel = (t - 1, t) if t > 0 else (0,)
    body, n = act_rows_bytes(rows_map, sel)
    return b"ACT1" + t.to_bytes(4, "little") + n.to_bytes(4, "little") + body

def act_c_violations(rows_map, sel):
    # mirror of air::count_violations over a window, per neuron
    bad = 0
    for gid in range(total):
        for idx, t in enumerate(sel):
            _t, vb, i_ext, i_syn, va, sp, rb = rows_map[gid][t]
            if t == 0:
                if vb != 0 or rb != 0:
                    bad += 1
                continue
            _pt, pvb, pi_ext, pi_syn, pva, psp, prb = rows_map[gid][t - 1]
            exp_r = (prb - 1) if prb > 0 else (REFRAC if psp == 1 else 0)
            if rb != exp_r:
                bad += 1
                continue
            if rb > 0:
                if va != 0 or sp != 0:
                    bad += 1
                continue
            leaked = vb - (vb >> LEAK_SHIFT)
            raw_i = max(V_MIN, min(V_MAX, leaked + i_ext + i_syn))
            exp_sp = 1 if raw_i >= V_TH else 0
            if sp != exp_sp:
                bad += 1
                continue
            if sp == 1:
                if va != 0:
                    bad += 1
            else:
                if va != raw_i:
                    bad += 1
    return bad

def act_fold(rows_map, sel_last, prev_head):
    sh = hashlib.sha256(bytes(1 if rows_map[gid][sel_last][5] else 0
                              for gid in range(total))).digest()
    vh = hashlib.sha256(b"".join(rows_map[gid][sel_last][4].to_bytes(4, "little", signed=True)
                                 for gid in range(total))).digest()
    return hashlib.sha256(prev_head + sel_last.to_bytes(8, "big") + sh + vh).hexdigest()

tape = act_encode(rowsA, T)
sel = (T - 1, T)
hon_viol = act_c_violations(rowsA, sel)
hon_fold = act_fold(rowsA, T, bytes.fromhex(ff_log[T - 1]))
# forged twin: flip the CxEb+3 spike at T, zero the membrane bytes (lazy liar)
gid_tam = off[CX_EB] + 3
rowsF = {g: list(rowsA[g]) for g in rowsA}
t_, vb, ie, isy, va, sp, rb = rowsF[gid_tam][T]
rowsF[gid_tam][T] = (t_, vb, ie, isy, va, 1 - sp, rb)
# lazy liar: the fold's membrane bytes of tick T are all zeros => claim v_after == 0
for g in range(total):
    _t2, vb2, ie2, isy2, va2, sp2, rb2 = rowsF[g][T]
    rowsF[g][T] = (_t2, vb2, ie2, isy2, 0, sp2, rb2)
forg_viol = act_c_violations(rowsF, sel)
forg_fold = act_fold(rowsF, T, bytes.fromhex(ff_log[T - 1]))
tapeF = act_encode(rowsF, T)
pin("act.tick", T)
pin("act.rows", 2 * total)
pin("act.tape_len", len(tape))
pin("act.sha256", hashlib.sha256(tape).hexdigest())
pin("act.violations_honest", hon_viol)
pin("act.fold_honest_ok", hon_fold == ff_log[T])
pin("act.violations_forged", forg_viol)
pin("act.fold_forged_still_ok", forg_fold == ff_log[T])
pin("act.sha256_forged", hashlib.sha256(tapeF).hexdigest())


# ---------------------------------------------------------------- [attest]
# Neural identity handshake: proof-of-exact-build. A peer proves it runs
# THIS frozen connectome through THIS frozen code path by answering a
# (peer, epoch, nonce)-bound challenge. Three rungs of confidence:
#   L1 = 1-tick canary      (~32 B, heartbeat)
#   L2 = 48-tick chain end  (full dynamics)
#   L3 = court envelope     (verdict + anchor lineage)
# Honest scope: a behavioral build-fingerprint for closed validator sets —
# anyone holding the pinned map can replay it; that map IS the asset being
# attested. Not a TEE, not a VDF (see docs/BUDFLY_APPLICATIONS.md).
def att_digest(peer, epoch, nonce):
    return hashlib.sha256(b"BUDFLY-ATTEST1\x00" + peer +
                          epoch.to_bytes(8, "big") + nonce).digest()
peer = b"fly-peer-01"
ap_ch = att_digest(peer, 7, bytes([0xA5] * 32))
l1 = anchor_log_generic(1, sentinel_stim(ap_ch))[0]
l2log = anchor_log_generic(48, sentinel_stim(ap_ch))
l2 = l2log[-1]
av, adl, adr, amdn, ach = sentinel(sizes, off, edges, ap_ch)
pin("attest.peer", peer.decode())
pin("attest.epoch", 7)
pin("attest.challenge", ap_ch.hex())
pin("attest.l1_canary", l1)
pin("attest.l1_prefix_of_l2", l2log[0] == l1)
pin("attest.l2_chain_end", l2)
pin("attest.l3_verdict", av)
pin("attest.l3_counters", f"{adl},{adr},{amdn}")
# freshness proof: epoch 8 must move every rung
ap_ch8 = att_digest(peer, 8, bytes([0xA5] * 32))
pin("attest.epoch_moves_l2", anchor_log_generic(48, sentinel_stim(ap_ch8))[-1] != l2)


# ------------------------------------------------------------ [d1 fixtures]
# The bud-zero handoff: the ACT-1 arbitration tapes as canonical fixture
# files. scripts/dump_fixtures.py regenerates these byte-for-byte; the
# regeneration-equality pins below are what keeps the circuit's test
# vectors from quietly drifting while the zk pipeline learns to read them.
hon_tape = act_encode(rowsA, T)
gi = off[CX_EB] + 3
rowsL = {g: list(rowsA[g]) for g in rowsA}
t_, vb, ie, isy, va, sp, rb = rowsL[gi][T]
rowsL[gi][T] = (t_, vb, ie, isy, va, 1 - sp, rb)
for g in range(total):
    _t2, vb2, ie2, isy2, va2, sp2, rb2 = rowsL[g][T]
    rowsL[g][T] = (_t2, vb2, ie2, isy2, 0, sp2, rb2)
liar_tape = act_encode(rowsL, T)
pin("d1.honest_tape_sha256", hashlib.sha256(hon_tape).hexdigest())
pin("d1.liar_tape_sha256", hashlib.sha256(liar_tape).hexdigest())
pin("d1.honest_expect_violations", act_c_violations(rowsA, (T - 1, T)))
pin("d1.liar_expect_violations", act_c_violations(rowsL, (T - 1, T)))
pin("d1.fixtures_contract", "budfly-fixtures-v1")

# ---------------------------------------------------------------- [league]
# Season mode: FOUR seeds, 8-digest panel each, decisiveness standings.
# Standing rule (frozen): fewer council non-unanimities ranks higher;
# ties broken by seat-0 anchor of panel digest 0, lexicographic.
LEAGUE_SEEDS = [0xB0DF17, 0xB0DF18, 0xB0DF19, 0xB0DF1A]
flies = {}
for sd in LEAGUE_SEEDS:
    flies[sd] = build_connectome(1, 16, seed=sd)
rows_by_seed = {}
for sd in LEAGUE_SEEDS:
    sz, of, tot, ed = flies[sd]
    assert (sz, of, tot) == (sizes, off, total), "same anatomy across seeds"
    ab = 0
    for d in panel:
        vs = [sentinel(sz, of, ed, seat_digest(d, s))[0] for s in range(3)]
        if not (vs[0] == vs[1] == vs[2]):
            ab += 1
    tie_anchor = sentinel(sz, of, ed, seat_digest(panel[0], 0))[4]
    rows_by_seed[sd] = (ab, tie_anchor)
standing = sorted(LEAGUE_SEEDS, key=lambda sd: (rows_by_seed[sd][0], rows_by_seed[sd][1]))
pin("league.seeds", ",".join(hex(s) for s in LEAGUE_SEEDS))
pin("league.abstains", ",".join(str(rows_by_seed[sd][0]) for sd in LEAGUE_SEEDS))
pin("league.standing", ",".join(hex(s) for s in standing))
pin("league.champion", hex(standing[0]))

# --------------------------------------------------------------- [erasure]
# D2 experiment: the wandering prover. A blob of 32 chunks (deterministic
# toy bytes), merkle root as the committed fact; a verifier issues K = 8
# chunk-bound canary challenges; the prover answers each with the 1-tick
# L1 anchor. The pin pins the PROTOCOL, not a storage claim (see the
# honest-negative below and docs/BUDFLY_APPLICATIONS.md D2 row).
CHUNKS = [hashlib.sha256(b"budfly-erasure-chunk" + i.to_bytes(2, "little")).digest()
          for i in range(32)]
er_root = hashlib.sha256(b"".join(CHUNKS)).digest()
def chunk_challenge(root, idx, epoch):
    return hashlib.sha256(b"BUDFLY-ERASURE1\x00" + root +
                          idx.to_bytes(2, "little") + epoch.to_bytes(8, "big")).digest()
er_answers = [anchor_log_generic(1, sentinel_stim(chunk_challenge(er_root, i, 3)))[0]
              for i in [0, 5, 9, 13, 17, 21, 26, 31]]
pin("erasure.root", er_root.hex())
pin("erasure.k", 8)
pin("erasure.answers", ",".join(a[:16] for a in er_answers))
# avalanche property: flip ONE bit in one chunk -> root moves -> all
# challenges move -> every answer changes.
CHUNKS2 = list(CHUNKS)
CHUNKS2[13] = bytes([CHUNKS2[13][0] ^ 1]) + CHUNKS2[13][1:]
er_root2 = hashlib.sha256(b"".join(CHUNKS2)).digest()
er_answers2 = [anchor_log_generic(1, sentinel_stim(chunk_challenge(er_root2, i, 3)))[0]
               for i in [0, 5, 9, 13, 17, 21, 26, 31]]
pin("erasure.bitflip_moves_all", all(a != b for a, b in zip(er_answers, er_answers2)))
# HONEST NEGATIVE (pinned so nobody retrades it later): answering the
# canary proves running the pinned map on the challenge; it does NOT by
# itself prove the chunk bytes were ever read. Soundness requires the
# chunk bytes feeding the stimulus stream, which this v1 does NOT do.
pin("erasure.proves_possession", False)

# ------------------------------------------------------------- [hardening]
# The adversary sweep: not "does the honest tape pass" but "how much room
# does a forger actually have". Every question below is run, not argued:
#   tape    every single-bit flip of the honest tape + every truncation
#   erasure every chunk position avalanches all K answers
#   attest  every nonce byte avalanches L1; ladder stays self-consistent
#   league  replay/shuffle/tie invariants of the frozen season
#
# CLASSIFICATION PRECEDENCE (frozen): malformed -> viol_caught ->
# fold_caught -> escaped. The escape inventory is pinned OFFSET BY OFFSET
# (sha256 of the LE offset list): the forger's entire freedom for v1 is a
# published, hashed set of byte positions.
#
# WINDOW SEMANTICS — this evaluator mirrors tape.rs/air.rs EXACTLY: the C6
# refractory chain is checked only INSIDE the taped window (i > 0), genesis
# is tick-keyed, and v_before is an input to the constraints, not a chained
# commitment. That is what the shipped verifier enforces.
ROWB = 21
def tape_decode_rust(b):
    """Bit-exact mirror of tape::decode_tape. Returns (tick, rows) or None;
    rows are (vb, ie, isy, va, rb, sp) tuples with window tick labels."""
    if len(b) < 12 or b[:4] != b"ACT1":
        return None
    tick = int.from_bytes(b[4:8], "little")
    count = int.from_bytes(b[8:12], "little")
    if len(b) != 12 + count * ROWB:
        return None
    tib = count // total  # checked_div; total == 588 here
    if tib != (1 if tick == 0 else 2):
        return None
    base = tick - 1 if tick > 0 else 0
    rows = []
    for i in range(count):
        s = 12 + i * ROWB
        bb = b[s:s + ROWB]
        rows.append((
            int.from_bytes(bb[0:4], "little", signed=True),
            int.from_bytes(bb[4:8], "little", signed=True),
            int.from_bytes(bb[8:12], "little", signed=True),
            int.from_bytes(bb[12:16], "little", signed=True),
            int.from_bytes(bb[16:20], "little"),
            bb[20],
            base + i // total,
        ))
    return tick, rows

def air_window_violations(nrows):
    """Bit-exact mirror of air::count_violations for ONE neuron's window
    rows [(vb,ie,isy,va,rb,sp,tick), ...] in tape order."""
    bad = 0
    for i, (vb, ie, isy, va, rb, sp, tick) in enumerate(nrows):
        # EXACT air.rs tree: a FAILED genesis check counts and skips; an
        # OK genesis row FALLS THROUGH to the state gates (no continue).
        if tick == 0:
            if vb != 0 or rb != 0:
                bad += 1
                continue
        elif i > 0:
            pp = nrows[i - 1]
            exp_r = (pp[4] - 1) if pp[4] > 0 else (REFRAC if pp[5] == 1 else 0)
            if rb != exp_r:
                bad += 1
                continue
        if rb > 0:
            if va != 0 or sp != 0:
                bad += 1
            continue
        leaked = vb - (vb >> LEAK_SHIFT)
        raw = max(V_MIN, min(V_MAX, leaked + ie + isy))
        exp_sp = 1 if raw >= V_TH else 0
        if sp != exp_sp:
            bad += 1
            continue
        if exp_sp == 1:
            if va != 0:
                bad += 1
        elif va != raw:
            bad += 1
    return bad

def tape_fold_rust(tick, rows, prev_head):
    """Mirror of tape::recompute_fold over the decoded rows."""
    last = rows[len(rows) - total:]
    sh = hashlib.sha256(bytes(r[5] for r in last[:total])).digest()
    vh = hashlib.sha256(b"".join(r[3].to_bytes(4, "little", signed=True)
                                 for r in last[:total])).digest()
    return hashlib.sha256(prev_head + tick.to_bytes(8, "big") + sh + vh).digest()

TAPE_TICK, PUB_HEAD = T, bytes.fromhex(ff_log[T])
PREV_HEAD = bytes.fromhex(ff_log[T - 1])
mutations = malformed = viol_caught = fold_caught = 0
escapes = []
tick_flip_hits = []  # header mutant detail, for the report line
for o in range(len(tape)):
    if o < 12:
        mb = bytearray(tape)
        mb[o] ^= 1
        dec = tape_decode_rust(bytes(mb))
        if dec is None:
            malformed += 1
            continue
        tk, rows = dec
        by_neuron = [[] for _ in range(total)]
        for idx_r, r in enumerate(rows):
            by_neuron[idx_r % total].append(r)
        viol = sum(air_window_violations(nrs) for nrs in by_neuron)
        if viol > 0:
            viol_caught += 1
        elif tape_fold_rust(tk, rows, PREV_HEAD) != PUB_HEAD:
            fold_caught += 1
            tick_flip_hits.append(o)
        else:
            escapes.append(o)
        continue
    # body mutant: row i, neuron gid; only that neuron's 2-row window
    # can change verdict (baseline is zero violations everywhere).
    i = (o - 12) // ROWB
    gid = i % total
    tw = T - 1 + i // total
    rb21 = bytearray(tape[12 + i * ROWB: 12 + (i + 1) * ROWB])
    rb21[(o - 12) % ROWB] ^= 1
    patched = (
        int.from_bytes(rb21[0:4], "little", signed=True),
        int.from_bytes(rb21[4:8], "little", signed=True),
        int.from_bytes(rb21[8:12], "little", signed=True),
        int.from_bytes(rb21[12:16], "little", signed=True),
        int.from_bytes(rb21[16:20], "little"),
        rb21[20],
        tw,
    )
    honest = [tuple(rowsA[gid][t][j] for j in (1, 2, 3, 4, 6, 5)) + (t,)
              for t in (T - 1, T)]
    # note: rowsA tuples are (t, vb, ie, isy, va, sp, rb) -> reordered above
    # to (vb, ie, isy, va, rb, sp, tick) matching the air evaluator.
    win = [patched if t == tw else h for t, h in zip((T - 1, T), honest)]
    viol = air_window_violations(win)
    if viol > 0:
        viol_caught += 1
        continue
    # fold: patch only moves it when the byte lands in tick-T spike/v_after
    rows_last = None
    if tw == T and ((o - 12) % ROWB >= 12):
        rows_last = []
        for g2 in range(total):
            if g2 == gid:
                rows_last.append(patched)
            else:
                hx = rowsA[g2][T]
                rows_last.append((hx[1], hx[2], hx[3], hx[4], hx[6], hx[5], T))
        fd = tape_fold_rust(T, rows_last, PREV_HEAD)
    else:
        fd = tape_fold_rust(
            T,
            [(hx[1], hx[2], hx[3], hx[4], hx[6], hx[5], T)
             for hx in (rowsA[g][T] for g in range(total))],
            PREV_HEAD)
    if fd != PUB_HEAD:
        fold_caught += 1
    else:
        escapes.append(o)

mutations = len(tape)
trunc_decodable = sum(1 for L in range(len(tape))
                      if tape_decode_rust(tape[:L]) is not None)
append_decodable = 1 if tape_decode_rust(tape + b"\x00") is not None else 0
esc_hash = hashlib.sha256(b"".join(o.to_bytes(4, "little") for o in escapes)).hexdigest()
# escape CLASS inventory: (field_of_row(0..20), row_state) must be reported
# with the counts so a class shift breaks the pins even at equal totals.
def esc_class(o):
    if o < 12:
        return "header"
    f = (o - 12) % ROWB
    i = (o - 12) // ROWB
    gid = i % total
    hx = rowsA[gid][T - 1 + i // total]
    state = f"rb{hx[6]}sp{hx[5]}"  # r_before, spike of the honest row
    return f"f{f}:{state}"
from collections import Counter as _Counter
esc_inv = sorted(_Counter(esc_class(o) for o in escapes).items())
pin("hard.tape.mutations", mutations)
pin("hard.tape.malformed", malformed)
pin("hard.tape.viol_caught", viol_caught)
pin("hard.tape.fold_caught", fold_caught)
pin("hard.tape.escaped", len(escapes))
pin("hard.tape.escape_offsets_sha256", esc_hash)
pin("hard.tape.escape_classes", ";".join(f"{k}={v}" for k, v in esc_inv))
pin("hard.tape.trunc_decodable", trunc_decodable)
pin("hard.tape.append_decodable", append_decodable)

# erasure: full-position avalanche, not just chunk 13
era_chunk_ok = 0
for ci in range(32):
    cm = list(CHUNKS)
    cm[ci] = bytes([cm[ci][0] ^ 1]) + cm[ci][1:]
    r2 = hashlib.sha256(b"".join(cm)).digest()
    a2 = [anchor_log_generic(1, sentinel_stim(chunk_challenge(r2, ix, 3)))[0]
          for ix in [0, 5, 9, 13, 17, 21, 26, 31]]
    if all(x != y for x, y in zip(er_answers, a2)):
        era_chunk_ok += 1
era_epoch4 = [anchor_log_generic(1, sentinel_stim(chunk_challenge(er_root, ix, 4)))[0]
              for ix in [0, 5, 9, 13, 17, 21, 26, 31]]
pin("hard.erasure.chunk_avalanche", era_chunk_ok)
pin("hard.erasure.epoch_moves", sum(1 for x, y in zip(er_answers, era_epoch4) if x != y))

# attest: every nonce byte avalanches L1 and the ladder re-heads itself
base_l1 = l1  # from [attest]
nonce_av = ladder_ok = 0
for nb in range(32):
    nn = bytearray([0xA5] * 32)
    nn[nb] ^= 1
    chn = hashlib.sha256(b"BUDFLY-ATTEST1\\x00" + peer +
                         (7).to_bytes(8, "big") + bytes(nn)).digest()
    lg = anchor_log_generic(48, sentinel_stim(chn))
    if lg[0] != base_l1:
        nonce_av += 1
    if lg[0] == anchor_log_generic(1, sentinel_stim(chn))[0]:
        ladder_ok += 1
peer_flip_l2 = anchor_log_generic(48, sentinel_stim(
    hashlib.sha256(b"BUDFLY-ATTEST1\\x00" + b"fly-peer-02" +
                   (7).to_bytes(8, "big") + bytes([0xA5] * 32)).digest()))[-1]
pin("hard.attest.nonce_avalanche", nonce_av)
pin("hard.attest.ladder_robust", ladder_ok)
pin("hard.attest.peer_binding", peer_flip_l2 != l2)

# league: invariants of the frozen season table (reuses [league] rows)
ref_sizes, ref_off, ref_total, ref_edges = flies[0xB0DF17]
rep1 = [sentinel(ref_sizes, ref_off, ref_edges, seat_digest(panel[0], s))[0]
        for s in range(3)]
rep2 = [sentinel(ref_sizes, ref_off, ref_edges, seat_digest(panel[0], s))[0]
        for s in range(3)]
shuffled = sorted([0xB0DF19, 0xB0DF17, 0xB0DF1A, 0xB0DF18],
                  key=lambda sd: (rows_by_seed[sd][0], rows_by_seed[sd][1]))
abstain_list = [rows_by_seed[sd][0] for sd in LEAGUE_SEEDS]
pin("hard.league.council_replay_identical", rep1 == rep2)
pin("hard.league.shuffle_stable", shuffled == standing)
pin("hard.league.tie_engaged", len(set(abstain_list)) < len(abstain_list))
pin("hard.league.table_total_order", len(set(standing)) == 4)
pin("hard.league.total_councils", len(LEAGUE_SEEDS) * 8)
pin("hard.league.abstain_sum", sum(abstain_list))

# ------------------------------------------------------------------ [act2]
# ACT-2: the ghost-reaper. The v1 sweep proved the forger's ONLY freedom is
# the "ghost refractory" (r_before flip at the window's first tick), because
# neither the v1 chain fold nor the windowed C6 looks at tick T-1's
# refractory counter. The v2 kill is total, and it is architectural, not a
# patch on v1 (v1 stays frozen):
#   1. NEW CHAIN: fold2(t) = sha256(head2(t-1) | t be8 | sh(spikes) |
#      vh(v_after LE) | rh(r_before LE))  -- the refractory counter enters
#      the committed state.
#   2. BOTH window ticks are judged against their published head2 anchors:
#      fold2(T-1) == head2(T-1) AND fold2(T) == head2(T). A ghost at the
#      window's left edge dies on the FIRST anchor.
# Wire: "ACT2" + identical header/rows layout as ACT-1 (same 24,708 bytes).
def anchor2_log_generic(ticks, stim):
    adj = [[] for _ in range(total)]
    for p, q, w in edges:
        adj[p].append((q, w))
    v = [0] * total
    refr = [0] * total
    pend = [0] * total
    anchor = b"\x00" * 32
    log = []
    for t in range(ticks):
        spiking = []
        sbytes = bytearray(total)
        rbytes = bytearray()
        for gid in range(total):
            i_ext = I_STIM if (gid in stim and stim[gid][0] <= t < stim[gid][1]) else 0
            i_syn = pend[gid] if t > 0 else 0
            vb, rb = v[gid], refr[gid]
            rbytes += rb.to_bytes(4, "little")
            if rb > 0:
                refr[gid] = rb - 1
                v[gid] = 0
            else:
                raw = max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + i_ext + i_syn))
                if raw >= V_TH:
                    v[gid] = 0
                    refr[gid] = REFRAC
                    spiking.append(gid)
                    sbytes[gid] = 1
                else:
                    v[gid] = raw
        nxt = [0] * total
        for p in spiking:
            for q, w in adj[p]:
                nxt[q] = max(-FAN_IN_MAX, min(FAN_IN_MAX, nxt[q] + w))
        pend = nxt
        sh = hashlib.sha256(bytes(sbytes)).digest()
        vh = hashlib.sha256(b"".join(x.to_bytes(4, "little", signed=True) for x in v)).digest()
        rh = hashlib.sha256(bytes(rbytes)).digest()
        anchor = hashlib.sha256(anchor + t.to_bytes(8, "big") + sh + vh + rh).digest()
        log.append(anchor.hex())
    return log

a2log = anchor2_log_generic(48, ff_stim)
A2_TM2, A2_TM1, A2_T = a2log[T - 2], a2log[T - 1], a2log[T]
tape2 = b"ACT2" + tape[4:]

def tape2_decode(b):
    """Same decode rules as ACT-1, different magic."""
    if len(b) < 12 or b[:4] != b"ACT2":
        return None
    return tape_decode_rust(b"ACT1" + b[4:])

def tape2_viol(dec):
    tk, rows = dec
    by_neuron = [[] for _ in range(total)]
    for idx_r, r in enumerate(rows):
        by_neuron[idx_r % total].append(r)
    return sum(air_window_violations(nrs) for nrs in by_neuron), rows

def fold2_one(tick_val, one_tick_rows, prev_head):
    """fold2 over exactly `total` rows of one tick."""
    rr = one_tick_rows[:total]
    sh = hashlib.sha256(bytes(r[5] for r in rr)).digest()
    vh = hashlib.sha256(b"".join(r[3].to_bytes(4, "little", signed=True) for r in rr)).digest()
    rh = hashlib.sha256(b"".join(r[4].to_bytes(4, "little") for r in rr)).digest()
    return hashlib.sha256(prev_head + tick_val.to_bytes(8, "big") + sh + vh + rh).digest()

def judge2_py(b):
    """Mirror of tape2::judge2 -> None | (viol, ok_tm1, ok_t)."""
    dec = tape2_decode(b)
    if dec is None:
        return None
    tk, rows = dec
    _v, rows = tape2_viol(dec)
    viol = _v
    # window rows: first `total` = tick base, second `total` = tick base+1
    base = tk - 1 if tk > 0 else 0
    ok_tm1 = fold2_one(base, rows[:total], bytes.fromhex(A2_TM2)).hex() == A2_TM1
    ok_t = fold2_one(base + 1, rows[total:2 * total], bytes.fromhex(A2_TM1)).hex() == A2_T
    return viol, ok_tm1, ok_t

_h2 = judge2_py(tape2)
pin("act2.sha256", hashlib.sha256(tape2).hexdigest())
pin("act2.violations_honest", _h2[0])
pin("act2.fold2_ok", _h2[1] and _h2[2])
pin("act2.head2_tm1", A2_TM1)
pin("act2.head2_t", A2_T)
# forged twin on ACT-2: same lazy liar
rowsF2 = {g: list(rowsA[g]) for g in rowsA}
_t, vb, ie, isy, va, sp, rb = rowsF2[gid_tam][T]
rowsF2[gid_tam][T] = (_t, vb, ie, isy, va, 1 - sp, rb)
for g in range(total):
    _t2, vb2, ie2, isy2, va2, sp2, rb2 = rowsF2[g][T]
    rowsF2[g][T] = (_t2, vb2, ie2, isy2, 0, sp2, rb2)
tape2F = b"ACT2" + act_encode(rowsF2, T)[4:]
_f2 = judge2_py(tape2F)
pin("act2.violations_forged", _f2[0])
pin("act2.forged_fold2_ok", _f2[1] and _f2[2])
# THE GHOST-REAPER SWEEP: rerun all 24,708 single-bit mutants under judge2.
a2_mut = a2_mal = a2_viol = a2_fold = 0
a2_esc = []
for o in range(len(tape2)):
    mb2 = bytearray(tape2)
    mb2[o] ^= 1
    dec2 = tape2_decode(bytes(mb2))
    if dec2 is None:
        a2_mal += 1
        continue
    tk2, rows2 = dec2
    if o >= 12:
        # neuron-local violation fast path (baseline is zero)
        i = (o - 12) // ROWB
        gid2 = i % total
        tw2 = T - 1 + i // total
        patched2 = rows2[i]
        honest2 = [tuple(rowsA[gid2][t][j] for j in (1, 2, 3, 4, 6, 5)) + (t,)
                   for t in (T - 1, T)]
        win2 = [patched2 if t == tw2 else h for t, h in zip((T - 1, T), honest2)]
        viol2 = air_window_violations(win2)
    else:
        by_neuron2 = [[] for _ in range(total)]
        for idx_r2, r2 in enumerate(rows2):
            by_neuron2[idx_r2 % total].append(r2)
        viol2 = sum(air_window_violations(nrs) for nrs in by_neuron2)
    if viol2 > 0:
        a2_viol += 1
        continue
    base2 = tk2 - 1 if tk2 > 0 else 0
    ok1 = fold2_one(base2, rows2[:total], bytes.fromhex(A2_TM2)).hex() == A2_TM1
    ok2 = fold2_one(base2 + 1, rows2[total:2 * total], bytes.fromhex(A2_TM1)).hex() == A2_T
    if not (ok1 and ok2):
        a2_fold += 1
    else:
        a2_esc.append(o)
a2_hash = hashlib.sha256(b"".join(o.to_bytes(4, "little") for o in a2_esc)).hexdigest()
pin("act2.sweep_mutations", len(tape2))
pin("act2.sweep_malformed", a2_mal)
pin("act2.sweep_viol", a2_viol)
pin("act2.sweep_fold2", a2_fold)
pin("act2.sweep_escaped", len(a2_esc))
pin("act2.sweep_escape_hash", a2_hash)
pin("act2.ghosts_retired", a2_mal == 8 and a2_viol == 24300 and a2_fold == 400
    and len(a2_esc) == 0)

# ------------------------------------------------------------- [envelope3]
# BSE-3: the verdict plus a STACK of counterfactual cards — every "peki ya"
# the court considered, in one envelope. Layout (frozen):
#   "BSE3" | digest | 3*32 anchors | 3 verdicts | council | card_count u8 |
#   per card: fork u32 LE | base_head@fork-1 32B | branch_final 32B (68 B)
# Cheap rule (per card, no execution): one 32-byte compare of the card's
# base head against the PUBLISHED base chain at fork-1.
def encode_bse3(digest, seat_anchors, seat_verdicts, council, cards):
    b = b"BSE3" + digest + b"".join(bytes.fromhex(a) for a in seat_anchors)
    b += bytes([VCODE[v] for v in seat_verdicts]) + bytes([VCODE[council]])
    b += bytes([len(cards)])
    for fork, base_hex, branch_hex in cards:
        b += fork.to_bytes(4, "little") + bytes.fromhex(base_hex) + bytes.fromhex(branch_hex)
    return b

# card 2's counterfactual: "koku 4. tickte kesilseydi?" (odor cut early)
cf3 = dict(sentinel_stim(FF))
for g3 in list(cf3):
    if cf3[g3] == (0, 8):
        cf3[g3] = (0, 4)
branch2_log = anchor_log_generic(48, cf3)
assert branch2_log[:4] == base_log[:4], "card 2 prefix must bind before tick 4"
b3_cards = [(FORK, base_log[FORK - 1], branch_log[-1]),
            (4, base_log[3], branch2_log[-1])]
b3 = encode_bse3(FF, ff_seat_a, ff_seat_v, ff_council, b3_cards)
def cheap3(env_b, published_base):
    n = env_b[136]
    for c in range(n):
        s = 137 + c * 68
        fk = int.from_bytes(env_b[s:s + 4], "little")
        if fk <= 0 or env_b[s + 4:s + 36].hex() != published_base[fk - 1]:
            return False
    return True
pin("envelope3.len", len(b3))
pin("envelope3.cards", b3[136])
pin("envelope3.hex", b3.hex())
pin("envelope3.forks", ",".join(str(c[0]) for c in b3_cards))
pin("envelope3.branch2_final", branch2_log[-1])
pin("envelope3.cheap_replay_all", cheap3(b3, base_log))

# ------------------------------------------------------------- [erasure2]
# v1's K=8 frozen sample is a HEARTBEAT; the round protocol WALKS THE WHOLE
# BLOB. The schedule is a root-bound permutation (Fisher-Yates over the 32
# chunk indices, keystream sha256(DOMAIN | root | 'perm' | ctr le16)):
# round r takes slots perm[8r..8r+8]. Four rounds x 8 slots = 32 =
# FULL SURFACE COVERAGE BY CONSTRUCTION (a permuted walk visits every chunk
# exactly once). Still NOT possession (the v1 pin stands): each answer
# proves the pinned map ran on a chunk-bound challenge, never that bytes
# were read.
def era2_permutation(root):
    """Root-bound permutation of the 32 chunks: Fisher-Yates fed by the
    keystream sha256(DOMAIN | root | 'perm' | ctr le16). Frozen draw order:
    i walks 31..1, stream consumed two bytes at a time, carrying across
    draws. Four rounds x 8 slots then cover EVERY chunk exactly once --
    coverage is a theorem about the permutation, not a statistic."""
    order = list(range(32))
    stream = b""
    ctr = 0
    for i in range(31, 0, -1):
        while len(stream) < 2:
            stream += hashlib.sha256(b"BUDFLY-ERASURE1\x00" + root +
                                     b"perm" + ctr.to_bytes(2, "little")).digest()
            ctr += 1
        j = int.from_bytes(stream[:2], "little") % (i + 1)
        stream = stream[2:]
        order[i], order[j] = order[j], order[i]
    return order

era2_perm = era2_permutation(er_root)
era2_sched = [era2_perm[r * 8:(r + 1) * 8] for r in range(4)]
era2_all_idx = [x for row in era2_sched for x in row]
era2_answers = []
for r in range(4):
    for ix in era2_sched[r]:
        era2_answers.append(anchor_log_generic(
            1, sentinel_stim(chunk_challenge(er_root, ix, 3)))[0])
# per-round avalanche: chunk 13 bit flip must move every answer of EVERY round
era2_av_ok = 0
CH2 = list(CHUNKS)
CH2[13] = bytes([CH2[13][0] ^ 1]) + CH2[13][1:]
er_root2b = hashlib.sha256(b"".join(CH2)).digest()
for r in range(4):
    moved = True
    for j, ix in enumerate(era2_sched[r]):
        a2b = anchor_log_generic(1, sentinel_stim(chunk_challenge(er_root2b, ix, 3)))[0]
        if a2b == era2_answers[r * 8 + j]:
            moved = False
    if moved:
        era2_av_ok += 1
pin("erasure2.rounds", 4)
pin("erasure2.schedule", ";".join(",".join(map(str, row)) for row in era2_sched))
pin("erasure2.coverage_unique", len(set(era2_all_idx)))
pin("erasure2.round_avalanche", era2_av_ok)
pin("erasure2.answers_sha256",
    hashlib.sha256(b"".join(bytes.fromhex(a) for a in era2_answers)).hexdigest())

# ------------------------------------------------------------------ [zk]
# BudZero bridge, stone 1: the arithmetization SKELETON, executable.
# Trace columns per row (8): vb, ie, isy, va, rb, sp, sel_refrac, sel_above.
# Gates (4), each a selector-multiplied equation set over the columns:
#   G0 genesis (tick==0):     vb==0, rb==0
#   G1 refractory (rb>0):     va==0, sp==0
#   G2 integrate (rb==0):     sp==sel_above, va==(sp?0:clamp(leak+ie+isy))
#   G3 chain (i>0):           rb==(prev.rb>0 ? prev.rb-1 : (prev.sp?REFRAC:0))
# Threshold/ clamp are NOT native field ops: they arithmetize via bit-
# decomposition notes; with selectors multiplied in, max constraint degree
# is 3. The skeleton is EXECUTABLE: gate selection follows the same decision
# tree as air.rs, so its mismatch count MUST equal air's violation count on
# any trace — pinned on both the honest and the forged window.
def zk_skel_violations(nrows_all):
    bad = 0
    for nrs in nrows_all:
        for i, (vb, ie, isy, va, rb, sp, tick) in enumerate(nrs):
            # EXACT air.rs tree (genesis-OK rows fall through to G1/G2):
            if tick == 0:  # G0
                if vb != 0 or rb != 0:
                    bad += 1
                    continue
            elif i > 0:  # G3
                pp = nrs[i - 1]
                exp_r = (pp[4] - 1) if pp[4] > 0 else (REFRAC if pp[5] == 1 else 0)
                if rb != exp_r:
                    bad += 1
                    continue
            if rb > 0:  # G1
                if va != 0 or sp != 0:
                    bad += 1
                continue
            # G2 with selectors
            leaked = vb - (vb >> LEAK_SHIFT)
            raw = max(V_MIN, min(V_MAX, leaked + ie + isy))
            sel_above = 1 if raw >= V_TH else 0
            if sp != sel_above:
                bad += 1
                continue
            if sel_above == 1:
                if va != 0:
                    bad += 1
            elif va != raw:
                bad += 1
    return bad

zk_by_neuron = [[tuple(rowsA[g][t][j] for j in (1, 2, 3, 4, 6, 5)) + (t,)
                 for t in (T - 1, T)] for g in range(total)]
zk_by_neuron_F = [[tuple(rowsF[g][t][j] for j in (1, 2, 3, 4, 6, 5)) + (t,)
                   for t in (T - 1, T)] for g in range(total)]
pin("zk.columns", 8)
pin("zk.gates", 4)
pin("zk.degree_max", 3)
pin("zk.window_rows", 2 * total)
pin("zk.honest_violations", zk_skel_violations(zk_by_neuron))
pin("zk.forged_violations", zk_skel_violations(zk_by_neuron_F))

# --------------------------------------------------------------- [league2]
# The 8-team league and the SEASON ARCHIVE: three seasons, each on its own
# panel stream (season-salted digests), every table hashed into an archive
# chain. Rule: sha256("budfly-panel" | season u8 | i u8) for season >= 1.
# standing_digest(s) = sha256(season u8 | per team IN STANDING ORDER:
#   seed u64 LE | abstains u8 | tie_anchor 32B)
# archive_head: seeded sha256("BUDFLY-ARCHIVE1\x00"), chained per season.
LEAGUE2_SEEDS = list(range(0xB0DF17, 0xB0DF1F))
def season_panel_digest(season, i):
    return hashlib.sha256(b"budfly-panel" + bytes([season]) + bytes([i])).digest()
def play_season_table(season):
    table_rows = {}
    for sd in LEAGUE2_SEEDS:
        sz, of, tot, ed = build_connectome(1, 16, seed=sd)
        ab = 0
        for i in range(8):
            d = season_panel_digest(season, i)
            vs = [sentinel(sz, of, ed, seat_digest(d, s))[0] for s in range(3)]
            if not (vs[0] == vs[1] == vs[2]):
                ab += 1
        anchor = sentinel(sz, of, ed, seat_digest(season_panel_digest(season, 0), 0))[4]
        table_rows[sd] = (ab, anchor)
    order = sorted(LEAGUE2_SEEDS, key=lambda sd: (table_rows[sd][0], table_rows[sd][1]))
    return order, table_rows

ARCH_SEED = hashlib.sha256(b"BUDFLY-ARCHIVE1\x00").digest()
arch_head = ARCH_SEED.hex()
league2_out = {}
for S in (1, 2, 3):
    order, trows = play_season_table(S)
    sd_bytes = bytes([S])
    for sd in order:
        ab, anch = trows[sd]
        sd_bytes += sd.to_bytes(8, "little") + bytes([ab]) + bytes.fromhex(anch)
    sdigest = hashlib.sha256(sd_bytes).digest()
    arch_head = hashlib.sha256(bytes.fromhex(arch_head) + sdigest).hexdigest()
    league2_out[S] = (order, trows)
    pin(f"league2.s{S}.abstains", ",".join(str(trows[sd][0]) for sd in LEAGUE2_SEEDS))
    pin(f"league2.s{S}.standing", ",".join(hex(sd) for sd in order))
pin("league2.seeds", ",".join(hex(s) for s in LEAGUE2_SEEDS))
pin("league2.seasons", 3)
pin("league2.champions", ",".join(hex(league2_out[S][0][0]) for S in (1, 2, 3)))
pin("league2.archive_head_final", arch_head)

# ------------------------------------------------------------ [prosecution]
# FULL-STACK CONVICTION SWEEP: for EVERY tick k of the 48, the lazy liar
# tampers exactly there (CxEb+3 spike flip + zeroed v_after). The validator
# must (i) locate divergence EXACTLY at k and (ii) convict with the ACT-1
# judgement of that window (violations > 0 or fold mismatch). Any miss at
# any tick would be a hole in the whole dispute story.
pros_convicted = 0
pros_div_exact = True
pros_viols = []
for k in range(48):
    liar = anchor_log_run(48, ff_stim, tamper=k)
    div = next((t for t in range(48) if ff_log[t] != liar[t]), None)
    if div != k:
        pros_div_exact = False
    rowsK = {g: list(rowsA[g]) for g in rowsA}
    _t, vb, ie, isy, va, sp, rb = rowsK[gid_tam][k]
    rowsK[gid_tam][k] = (_t, vb, ie, isy, va, 1 - sp, rb)
    for g in range(total):
        _t2, v2, i2, s2, a2, p2, r2 = rowsK[g][k]
        rowsK[g][k] = (_t2, v2, i2, s2, 0, p2, r2)
    sel_k = (k - 1, k) if k > 0 else (0,)
    win_n = [[tuple(rowsK[g][t][j] for j in (1, 2, 3, 4, 6, 5)) + (t,)
              for t in sel_k] for g in range(total)]
    viol_k = sum(air_window_violations(nrs) for nrs in win_n)
    prev_k = (b"\x00" * 32) if k == 0 else bytes.fromhex(ff_log[k - 1])
    fold_ok = act_fold(rowsK, k, prev_k) == ff_log[k]
    if div == k and (viol_k > 0 or not fold_ok):
        pros_convicted += 1
    pros_viols.append(viol_k)
pin("pros.ticks", 48)
pin("pros.convicted", pros_convicted)
pin("pros.divergence_exact", pros_div_exact)
pin("pros.viol_min", min(pros_viols))
pin("pros.viol_max", max(pros_viols))
pin("pros.viol_sum", sum(pros_viols))
pin("pros.viol_hash",
    hashlib.sha256(",".join(map(str, pros_viols)).encode()).hexdigest())

# ------------------------------------------------------------- [bse3cheap]
# THE CHEAP GATE'S TRUST BOUNDARY, QUANTIFIED: flip every byte of the frozen
# BSE-3 envelope and ask the FREE checks (strict decode + council rule +
# per-card base compare) whether the mutant is accepted. Whatever survives
# is cheap-blind BY DESIGN: seat anchors, the fact digest, and branch finals
# are commitments only the expensive path (re-execution under dispute)
# re-verifies. The pin maps that boundary byte-exactly.
def py_decode3_strict(b):
    if len(b) < 137 or b[:4] != b"BSE3":
        return None
    n = b[136]
    if len(b) != 137 + n * 68:
        return None
    for c in range(n):
        s = 137 + c * 68
        fk = int.from_bytes(b[s:s + 4], "little")
        if fk == 0 or b[s + 4:s + 36] == bytes(32) or b[s + 36:s + 68] == bytes(32):
            return None
    return n

def py_cheap_accept(b):
    n = py_decode3_strict(b)
    if n is None:
        return False
    vdec = [x if x in (1, 2) else 0 for x in (b[132], b[133], b[134])]
    unan = vdec[0] == vdec[1] == vdec[2]
    council_dec = b[135] if b[135] in (1, 2) else 0
    if (vdec[0] if unan else 0) != council_dec:
        return False
    for c in range(n):
        s = 137 + c * 68
        fk = int.from_bytes(b[s:s + 4], "little")
        if not (0 < fk <= 48) or b[s + 4:s + 36].hex() != base_log[fk - 1]:
            return False
    return True

cheap_accepted = []
for o in range(len(b3)):
    mb = bytearray(b3)
    mb[o] ^= 1
    if py_cheap_accept(bytes(mb)):
        cheap_accepted.append(o)
def cheap_class(o):
    if 4 <= o < 36: return "digest"
    if 36 <= o < 132: return "anchors"
    if 132 <= o < 135: return "verdicts"
    if o == 135: return "council"
    if o == 136: return "count"
    if o >= 137:
        c, off = (o - 137) // 68, (o - 137) % 68
        if off < 4: return f"card{c}.fork"
        if off < 36: return f"card{c}.base_head"
        return f"card{c}.branch_final"
    return "magic"
from collections import Counter as _C2
cc = sorted(_C2(cheap_class(o) for o in cheap_accepted).items())
pin("bse3cheap.mutations", len(b3))
pin("bse3cheap.rejected", len(b3) - len(cheap_accepted))
pin("bse3cheap.accepted", len(cheap_accepted))
pin("bse3cheap.accepted_offsets_sha256",
    hashlib.sha256(b"".join(o.to_bytes(4, "little") for o in cheap_accepted)).hexdigest())
pin("bse3cheap.accepted_classes", ";".join(f"{k}={v}" for k, v in cc))

# ------------------------------------------------------------------ [zk2]
# BudZero bridge, stone 2: the arithmetized WITNESS of the frozen fixture.
# Columns materialized per row (8): vb, ie, isy, va, rb, sp, sel_refrac,
# sel_above (frozen rule: sel_above = 1 iff rb == 0 and clamp(leak + ie +
# isy) >= V_TH). The witness hash is what a circuit's test vector binds to.
def zk_witness_bytes(nrows_all):
    buf = bytearray()
    for nrs in nrows_all:
        for (vb, ie, isy, va, rb, sp, _tick) in nrs:
            leaked = vb - (vb >> LEAK_SHIFT)
            raw = max(V_MIN, min(V_MAX, leaked + ie + isy))
            sel_r = 1 if rb > 0 else 0
            sel_a = 1 if (rb == 0 and raw >= V_TH) else 0
            buf += vb.to_bytes(4, "little", signed=True)
            buf += ie.to_bytes(4, "little", signed=True)
            buf += isy.to_bytes(4, "little", signed=True)
            buf += va.to_bytes(4, "little", signed=True)
            buf += rb.to_bytes(4, "little")
            buf += bytes([sp, sel_r, sel_a])
    return bytes(buf)

hon_witness = zk_witness_bytes(zk_by_neuron)
lia_witness = zk_witness_bytes(zk_by_neuron_F)
w_sel_ones = a_sel_ones = 0
for nrs in zk_by_neuron:
    for (vb, ie, isy, va, rb, sp, _t) in nrs:
        if rb > 0:
            w_sel_ones += 1
        else:
            leaked = vb - (vb >> LEAK_SHIFT)
            if max(V_MIN, min(V_MAX, leaked + ie + isy)) >= V_TH:
                a_sel_ones += 1
pin("zk2.witness_rows", 2 * total)
pin("zk2.witness_sha256", hashlib.sha256(hon_witness).hexdigest())
pin("zk2.sel_refrac_ones", w_sel_ones)
pin("zk2.sel_above_ones", a_sel_ones)
pin("zk2.gate_g3_rows", total)
pin("zk2.liar_witness_differs",
    hashlib.sha256(lia_witness).digest() != hashlib.sha256(hon_witness).digest())
# NON-VACUITY PROBE: the fixture window (22,23) is a quiet tail, so both
# selector columns are all-zero there by pin. Prove the columns are not
# degenerate by re-measuring them over the bump-era window (7,8), where
# the compass stimulus is active:
pr_r = pr_a = 0
for g in range(total):
    for t in (7, 8):
        _t, vb, ie, isy, va, sp, rb = rowsA[g][t]
        if rb > 0:
            pr_r += 1
        else:
            if max(V_MIN, min(V_MAX, vb - (vb >> LEAK_SHIFT) + ie + isy)) >= V_TH:
                pr_a += 1
pin("zk2.probe_refrac_ones", pr_r)
pin("zk2.probe_above_ones", pr_a)

# ---------------------------------------------- manifest cross-validation
man = tomllib.loads((Path(__file__).resolve().parents[1] / "goldens.anchor.toml").read_text())
exp = man.get("expansion", {})
n_ok = 0
for tag, want in exp.items():
    got = RESULTS.get(tag)
    assert got is not None, f"manifest expansion tag without computation: {tag}"
    assert got == str(want), f"{tag}: manifest pins {want}, recomputed {got}"
    n_ok += 1
print(f"[manifest] {n_ok} expansion pins agree (one table, twice)")
print("EXPANSION CHECK PASS")
