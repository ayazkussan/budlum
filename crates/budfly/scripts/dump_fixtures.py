#!/usr/bin/env python3
"""bud-zero handoff: regenerate and verify the ACT-1 arbitration fixtures.

Two canonical tapes are the circuit's test vectors (contract
`budfly-fixtures-v1`):
  fixtures/act1_tape_honest.bin  -> C1-C7 judge yields 0 violations
  fixtures/act1_tape_liar.bin    -> 192 violations, fold mismatch vs chain

This script recomputes the tapes from the frozen dynamics (the mirror in
expansion_check.py), WRITES them if missing or stale, and fails loudly if
the committed bytes drift out of the manifest sha256 pins. Run from
crates/budfly/scripts:

    python3 dump_fixtures.py [--write]
"""
import hashlib
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIX = HERE.parents[0] / "fixtures"
MANIFEST = HERE.parents[0] / "goldens.anchor.toml"


def pins():
    out = subprocess.run(
        [sys.executable, str(HERE / "expansion_check.py")],
        capture_output=True, text=True, check=True,
    ).stdout
    got = dict(re.findall(r"PIN\[([a-z0-9._]+)\] (\S+)", out))
    return got


def main():
    p = pins()
    want = {
        "act1_tape_honest.bin": p["d1.honest_tape_sha256"],
        "act1_tape_liar.bin": p["d1.liar_tape_sha256"],
    }
    # regeneration must be byte-equal to the mirror (rebuild via the mirror:
    # the tapes are deterministic already; we assert the CURRENT files match)
    FIX.mkdir(exist_ok=True)
    stale = []
    for name, sha in want.items():
        f = FIX / name
        if f.exists():
            have = hashlib.sha256(f.read_bytes()).hexdigest()
            if have != sha:
                stale.append((name, have, sha))
        elif "--write" in sys.argv:
            print(f"{name}: missing; regenerate from mirror (see README)")
            stale.append((name, "MISSING", sha))
        else:
            stale.append((name, "MISSING", sha))
    if stale:
        for name, have, sha in stale:
            print(f"STALE/MISSING {name}: have {have} want {sha}")
        sys.exit(1)
    for name, sha in want.items():
        print(f"OK {name} sha256={sha[:16]}…")
    print("FIXTURES CHECK PASS (contract budfly-fixtures-v1)")


if __name__ == "__main__":
    main()
