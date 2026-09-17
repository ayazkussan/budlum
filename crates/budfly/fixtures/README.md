# budfly fixtures — contract `budfly-fixtures-v1`

Test vectors for the bud-zero arithmetization (and anyone re-implementing
the ACT-1 judge). Bytes are canonical: `scripts/dump_fixtures.py`
regenerates and verifies them against the manifest pins
(`d1.honest_tape_sha256`, `d1.liar_tape_sha256`); the CI gate runs it.

| file | meaning | expected judgement |
|---|---|---|
| `act1_tape_honest.bin` | window tape of the all-ones chain at tick 23 (rows 22, 23; 588 neurons; 21 B/row) | `0` C1–C7 violations; fold lands on published chain head |
| `act1_tape_liar.bin` | lazy-liar twin (CxEb+3 spike flipped at 23, membrane bytes zeroed) | `192` violations AND fold mismatch |

Wire format: see `src/tape.rs` module doc (identical table). Consumers
MUST treat any byte drift as a breaking change: the contract version is
pinned in the manifest as `d1.fixtures_contract`.
