#!/usr/bin/env python3
"""Budlum baglanti olcumu: belgelenmis durumu kodda dogrular.

Belge: docs/audit-2026-09-10/01-BAGLANTI-DOGRULAMA.md
Kural: FAIL = kod belgelenmis durumdan sapti (kodu ya da belgeyi guncelle).
Kullanim: python3 tools/audit/wiring_check.py   (repo kokunden)
Stdlib-only.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
fails: list[str] = []


def ok(msg: str) -> None:
    print(f"PASS  {msg}")


def fail(msg: str) -> None:
    print(f"FAIL  {msg}")
    fails.append(msg)


def warn(msg: str) -> None:
    print(f"WARN  {msg}")


def rs_files(*roots: str) -> list[Path]:
    out: list[Path] = []
    for r in roots:
        p = ROOT / r
        if p.is_dir():
            out.extend(sorted(p.rglob("*.rs")))
    return out


def in_test_region(path: Path, idx: int, lines: list[str]) -> bool:
    s = str(path)
    if "/tests/" in s or "/test" in s or "/kani/" in s:
        return True
    for j in range(max(0, idx - 30), idx):
        t = lines[j].strip()
        if t in ("#[test]", "#[cfg(test)]", "#[kani::proof]",
                 "#[cfg(kani)]") or t.startswith("mod tests"):
            return True
    return False


def prod_hits(pattern: str, skip_files: set[str]) -> list[str]:
    """Uretim "cagri" taramasi.

    Haric: `use` satirlari, `fn` tanim satirlari (ad cakismasi),
    kani ispat dosyalari, test bolgeleri.
    """
    hits: list[str] = []
    for f in rs_files("src", "crates", "bud", "budzero"):
        if f.name in skip_files:
            continue
        if "/kani/" in str(f):
            continue
        try:
            lines = f.read_text(errors="replace").splitlines()
        except OSError:
            continue
        for i, ln in enumerate(lines):
            s = ln.strip()
            if (s.startswith("use ") or s.startswith("pub use")
                    or s.startswith("fn ") or s.startswith("pub fn")
                    or s.startswith("async fn")):
                continue
            if pattern in ln and "pub mod" not in ln:
                if not in_test_region(f, i, lines):
                    hits.append(f"{f.relative_to(ROOT)}:{i + 1}")
    return hits


def check_unsafe_lock() -> None:
    lib = ROOT / "src/lib.rs"
    if "#![forbid(unsafe_code)]" in lib.read_text(errors="replace"):
        ok("unsafe kilidi: src/lib.rs forbid(unsafe_code)")
    else:
        fail("unsafe kilidi YOK: src/lib.rs")


def check_readme_coverage() -> None:
    mods = sorted(d for d in (ROOT / "src").iterdir() if d.is_dir())
    have = sorted(d.name for d in mods if (d / "README.md").is_file())
    missing = sorted(d.name for d in mods if not (d / "README.md").is_file())
    print(f"INFO  src README: {len(have)}/{len(mods)} ({' '.join(have)})")
    if missing:
        warn(f"README'siz {len(missing)} modul (deneysel sayilir): {' '.join(missing)}")


# Kodlama/onarim yolu (uretici): uretim cagrisi beklenmez (UNWIRED).
ENCODE_CALLS = ("encode_object(", "encode_parity(", "reconstruct_object(",
                "reconstruct(", "to_manifest(")
# Dogrulama/denetim yolu: uretim cagrisi beklenir (BAĞLI kalmali).
VERIFY_CALLS = ("for_scheme(", "verify_object_encoding(")


def check_erasure() -> None:
    hits: list[str] = []
    for pat in ENCODE_CALLS:
        hits.extend(prod_hits(pat, {"erasure.rs"}))
    hits = sorted(set(hits))
    if not hits:
        ok("erasure encode/reconstruct: uretim cagrisi yok (UNWIRED, belgeyle tutarli)")
    else:
        fail(f"erasure encode/reconstruct: yeni uretim cagrilari {hits[:5]} (01'i guncelle)")
    for pat in VERIFY_CALLS:
        vhits = prod_hits(pat, {"erasure.rs"})
        name = pat.rstrip("(")
        if vhits:
            extra = f" +{len(vhits) - 1}" if len(vhits) > 1 else ""
            ok(f"erasure {name}: BAGLI ({vhits[0]}{extra})")
        else:
            fail(f"erasure {name}: uretim cagrisi YOK (01'e gore BAĞLI olmali)")


def check_assign() -> None:
    deal = ROOT / "src/domain/storage_deal.rs"
    if "assignment::assign_shard" in deal.read_text(errors="replace"):
        ok("assign_shard: BAGLI (storage_deal.rs)")
    else:
        fail("assign_shard: beklenen cagri storage_deal.rs'de YOK")
    for name, skip in (("assign_object(", {"assignment.rs"}),
                       ("reconstruct_object(", {"erasure.rs"})):
        hits = [h for h in prod_hits(name, skip) if "storage/mod.rs" not in h]
        if not hits:
            ok(f"{name.rstrip('(')}: uretim cagrisi yok (UNWIRED, belgeyle tutarli)")
        else:
            fail(f"{name.rstrip('(')}: yeni uretim cagrilari {hits[:5]} (01'i guncelle)")


def check_verifymerkle() -> None:
    zt = (ROOT / "src/execution/zkvm.rs").read_text(errors="replace")
    if re.search(r"#\[cfg\(test\)\]\s*\n\s*pub fn execute_bytecode_ungated", zt):
        ok("VerifyMerkle: ungated yalnizca #[cfg(test)]")
    else:
        fail("VerifyMerkle: ungated gate isareti bulunamadi")
    spec = ROOT / "budzero/docs/BudL_SPEC.md"
    if "gated off on mainnet" in spec.read_text(errors="replace"):
        ok("VerifyMerkle: BudL_SPEC gate ifadesi mevcut")
    else:
        fail("VerifyMerkle: BudL_SPEC gate ifadesi YOK")
    sr = ROOT / "src/storage/README.md"
    if "production gate (closed)" in sr.read_text(errors="replace"):
        ok("VerifyMerkle: storage README gate ifadesi mevcut")
    else:
        fail("VerifyMerkle: storage README gate ifadesi YOK")


def check_baselines() -> None:
    p = ROOT / ".github/unwired-guards-baseline.txt"
    n = int(p.read_text(errors="replace").splitlines()[0])
    if n <= 3:
        ok(f"unwired-guards-baseline: {n} <= 3")
    else:
        fail(f"unwired-guards-baseline: {n} > 3 (yalnizca dusebilir)")
    idle = ROOT / ".github/idle-code-baseline.txt"
    if idle.is_file():
        first = idle.read_text(errors="replace").splitlines()[0][:60]
        print(f"INFO  idle-code-baseline mevcut: {first}")


def check_lubot_refs() -> None:
    hits = []
    for f in rs_files("src", "crates", "bud", "budzero"):
        try:
            txt = f.read_text(errors="replace")
        except OSError:
            continue
        if re.search(r"(?i)lubot", txt):
            hits.append(str(f.relative_to(ROOT)))
    if not hits:
        ok("lubot: cekirdekte referans yok (sapma notu gecerli)")
    else:
        warn(f"lubot: cekirdekte {len(hits)} dosya: {hits[:5]} (01'i guncelle)")


def main() -> int:
    check_unsafe_lock()
    check_readme_coverage()
    check_erasure()
    check_assign()
    check_verifymerkle()
    check_baselines()
    check_lubot_refs()
    print(f"--- {len(fails)} FAIL ---")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
