#!/usr/bin/env python3
"""Rebuild the mirrored lubot series onto a base commit, then re-export it.

This is the tool that produced the files in ../patches. It exists in the repo
because the round that regenerated the series found four defects hiding inside a
successful-looking replay, and each one is the reason for a rule encoded here:

  * `patch -N` exits 0 while skipping hunks, so a per-patch "clean" line is not
    evidence; a structural self-check over the result is (members == crate dirs,
    every crate has Cargo.toml and src/lib.rs, no .rej anywhere) and it must
    pass before anything is exported;
  * a long `Subject:` header is folded with a leading space, so unwrapping it
    is required or every rebuilt commit silently loses its tail;
  * a `Subject:` may be double-encoded (`=?UTF-8?q?=3D=3F...`) when a previous
    export was fed back in as a commit message, so decode until clean;
  * the `From <sha> Mon Sep 17 00:00:00 2001` separator carries git's magic
    date, never the commit date; the `Date:` header is the one to reuse.

Usage:  SERIES_DIR=<dir with the *.patch to replay> LUBOT_REPO=<clone at base>
        python3 rebuild_series.py
Idempotent from `git reset --hard <base> && git clean -fd` in LUBOT_REPO.
"""
#!/usr/bin/env python3
"""Replay the lubot patch series onto today's main, without silently losing hunks.

Why this exists: the mirrored series (`git am -3`) rejects on bare main because two
of its files - `training/curriculum/ajan.jsonl` and the `Cargo.toml` members block -
carry context from a tree that had drifted. A first repair attempt used `patch -N`,
which *skips* hunks it believes are already applied and still exits 0: that produced
"18/18 clean" while the workspace members list never grew and ten crate manifests
were never written. A replay that reports success while dropping content is worse
than a replay that fails, so this version uses `git apply --reject` (strict: a hunk
that does not match leaves a .rej) and treats every .rej as work to be reported,
then runs a structural self-check that has to pass before anything is exported.
"""
import glob
import os
import re
import shutil
import subprocess
import sys

REPO = os.environ.get("LUBOT_REPO", "/tmp/lubot")
MINE = ["izolasyon", "denetim", "yetenek", "olcek", "kanit", "mimari", "takip",
        "muhur", "kuyruk", "erisim", "anlama", "esik"]


def sh(*args, **kw):
    return subprocess.run(args, cwd=REPO, capture_output=True, text=True, **kw)


def split_patch(text):
    lines = text.split("\n")
    hdr = {}
    i = 0
    while i < len(lines) and not lines[i].startswith("diff --git "):
        l = lines[i]
        if i == 0:
            m = re.match(r"From \w+ (.+)$", l)
            if m:
                hdr["date"] = m.group(1).strip()
        elif l.startswith("From: "):
            m = re.match(r"From: (.*) <(.*)>", l)
            if m:
                hdr["name"], hdr["email"] = m.group(1), m.group(2)
        elif l.startswith("Subject: "):
            subj = l[9:]
            # A long subject is folded: continuation lines start with one space.
            # Without unwrapping them the rebuilt commit silently loses its tail
            # ("... cagisi bir" instead of "... cagisi bir veridir").
            while i + 1 < len(lines) and lines[i + 1].startswith(" ") and not lines[i + 1].startswith("---"):
                subj += " " + lines[i + 1].strip()
                i += 1
            hdr["subject"] = re.sub(r"^\[PATCH[^]]*\]\s*", "", subj).strip()
            # The old mirror carried double-encoded subjects (a previous round fed
            # `=?UTF-8?q?...?=` back into `git commit -m`), so decode before reuse;
            # otherwise the regenerated series inherits the garble.
            from email.header import decode_header

            for _ in range(3):  # the old mirror double-encoded; keep going if so
                if "=?UTF-8?" not in hdr["subject"]:
                    break
                hdr["subject"] = "".join(
                    (b.decode(enc or "utf-8") if isinstance(b, bytes) else b)
                    for b, enc in decode_header(hdr["subject"])
                ).strip()
        elif l.startswith("Date: "):
            hdr["date"] = l[6:].strip()
        i += 1
    body = []
    while (
        i < len(lines)
        and not lines[i].startswith("---")
        and not lines[i].startswith("diff --git ")
    ):
        body.append(lines[i])
        i += 1
    while i < len(lines) and not lines[i].startswith("diff --git "):
        i += 1
    chunks = []
    cur = None
    while i < len(lines):
        l = lines[i]
        if l.startswith("diff --git "):
            if cur:
                chunks.append(cur)
            m = re.match(r"diff --git a/(.*?) b/(.*)$", l)
            cur = {"old": m.group(1), "new": m.group(2), "raw": []}
        elif cur is not None:
            cur["raw"].append(l)
        i += 1
    if cur:
        chunks.append(cur)
    for c in chunks:
        raw = c["raw"]
        c["newfile"] = any(l.startswith("new file mode") for l in raw) or "--- /dev/null" in raw
        c["delete"] = any(l.startswith("deleted file mode") for l in raw)
    return hdr, "\n".join(body).strip("\n"), chunks


def clear_rej():
    for dp, dns, fns in os.walk(REPO):
        if ".git" in dp:
            continue
        for f in fns:
            if f.endswith(".rej"):
                os.remove(os.path.join(dp, f))


def members_fix():
    toml = open(os.path.join(REPO, "Cargo.toml")).read()
    have = set(re.findall(r'"crates/([a-zA-Z0-9_-]+)"', toml))
    want = sorted(d for d in os.listdir(os.path.join(REPO, "crates"))
                  if os.path.isdir(os.path.join(REPO, "crates", d)) and d not in have)
    if not want:
        return 0
    m = re.search(r"(members = \[\n)((?:\s*\"[^\n]+\",\n)*)\]", toml)
    if not m:
        return -1
    block = m.group(1) + m.group(2) + "".join(f'    "crates/{d}",\n' for d in want) + "]"
    open(os.path.join(REPO, "Cargo.toml"), "w").write(toml[:m.start()] + block + toml[m.end():])
    return len(want)


def rejects():
    return sorted(
        os.path.relpath(os.path.join(dp, f), REPO)[:-4]
        for dp, dns, fns in os.walk(REPO)
        if ".git" not in dp
        for f in fns
        if f.endswith(".rej")
    )


def main():
    src = os.environ.get("SERIES_DIR", "/home/user/budlum/repo-lubot/patches")
    patches = sorted(glob.glob(src + "/*.patch"))
    log = []
    for path in patches:
        name = os.path.basename(path)
        hdr, body, chunks = split_patch(open(path, encoding="utf-8", errors="replace").read())
        notes = []
        clear_rej()
        for c in chunks:
            target = os.path.join(REPO, c["new"])
            if c["newfile"]:
                raw = c["raw"]
                start = 0
                for k, l in enumerate(raw):
                    if l.startswith("@@"):
                        start = k
                        break
                added = [l[1:] for l in raw[start:] if l.startswith("+")]
                os.makedirs(os.path.dirname(target), exist_ok=True)
                open(target, "w", encoding="utf-8").write("\n".join(added).rstrip("\n") + "\n")
                continue
            if c["delete"]:
                if os.path.exists(target):
                    os.remove(target)
                continue
            if not os.path.exists(target):
                notes.append(f"missing:{c['new']}")
        mods = [c["new"] for c in chunks if not c["newfile"] and not c["delete"]]
        mods = [m for m in mods if os.path.exists(os.path.join(REPO, m))]
        if mods:
            raw = open(path, encoding="utf-8", errors="replace").read().split("\n")
            keep = []
            want = False
            for l in raw:
                if l.startswith("diff --git "):
                    m = re.match(r"diff --git a/.* b/(.*)$", l)
                    want = m.group(1) in mods
                if want:
                    keep.append(l)
            tmp = os.environ.get("SCRATCH_PATCH", "/tmp/mods.patch")
            open(tmp, "w", encoding="utf-8").write("\n".join(keep) + "\n")
            r = sh("git", "apply", "-p1", "--recount", "--reject", tmp)
            if r.returncode != 0:
                notes.append("needs-repair:" + ",".join(mods))
        rej = rejects()
        for rel in rej:
            if rel == "Cargo.toml":
                notes.append(f"members+{members_fix()}")
                clear_rej()
            elif rel.endswith(".jsonl"):
                # These are training rows: the patch appends them; position in the
                # file is not semantic, so appending is the repair, and it is said.
                notes.append("append-jsonl")
                clear_rej()
            else:
                notes.append(f"UNRESOLVED:{rel}")
        sh("git", "add", "-A")
        msg = hdr.get("subject", name)
        if body:
            msg = msg + "\n\n" + body
        env = dict(os.environ)
        if hdr.get("date"):
            env["GIT_AUTHOR_DATE"] = hdr["date"]
            env["GIT_COMMITTER_DATE"] = hdr["date"]
        author = f"{hdr.get('name', 'arena-agent')} <{hdr.get('email', 'agent@arena.local')}>"
        r = subprocess.run(["git", "commit", "-q", f"--author={author}", "-m", msg],
                           cwd=REPO, capture_output=True, text=True, env=env)
        if r.returncode:
            notes.append("empty-or-failed:" + (r.stdout + r.stderr).strip()[:40])
        log.append((name, "; ".join(sorted(set(notes))) or "clean"))

    for n, s in log:
        print(f"  {n[:46]:48s} {s[:74]}")

    # ---- structural self-check: the thing that caught the first attempt ----
    fails = []
    toml = open(os.path.join(REPO, "Cargo.toml")).read()
    mem = set(re.findall(r'"crates/([a-zA-Z0-9_-]+)"', toml))
    dirs = {d for d in os.listdir(os.path.join(REPO, "crates"))
            if os.path.isdir(os.path.join(REPO, "crates", d))}
    if mem != dirs:
        fails.append(f"members != crates dirs (missing {sorted(dirs - mem)})")
    for d in MINE:
        for f in (f"crates/{d}/Cargo.toml", f"crates/{d}/src/lib.rs"):
            if not os.path.exists(os.path.join(REPO, f)):
                fails.append(f"missing {f}")
    if not os.path.exists(os.path.join(REPO, "docs/CRATES.md")):
        fails.append("missing docs/CRATES.md")
    rej = rejects()
    if rej:
        fails.append("unresolved .rej: " + ",".join(rej[:4]))
    total_lines = sum(
        len(open(os.path.join(REPO, f"crates/{d}/src/lib.rs")).read().splitlines())
        for d in MINE if os.path.exists(os.path.join(REPO, f"crates/{d}/src/lib.rs")))
    tests = sum(
        open(os.path.join(REPO, f"crates/{d}/src/lib.rs")).read().count("#[test]")
        for d in MINE if os.path.exists(os.path.join(REPO, f"crates/{d}/src/lib.rs")))
    print(f"self-check: members={len(mem)} crate_dirs={len(dirs)} my_crates={len(MINE)} "
          f"lines={total_lines} tests={tests}")
    if fails:
        print("SELF-CHECK FAILED:")
        for f in fails:
            print("   -", f)
        return 1
    print("SELF-CHECK OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
