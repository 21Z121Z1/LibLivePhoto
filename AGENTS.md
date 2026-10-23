# LibLivePhoto change checks

For resource-extraction work, use the canonical [verification skill](.agents/skills/verify-liblivephoto/SKILL.md). Keep checks proportional for unrelated edits.

| Rule | Enforcement |
| --- | --- |
| Check the complete selected response before copying its payloads. Count distinct IDs separately even when their extents are shared. | Private `CheckedExtraction`, shared `checked_response_len`, and the scaled public-path replay. |
| Keep unknown bytes, request order, default XMP behavior, and explicit invalid-selection errors. | Interface extraction and protocol tests. |
| Preserve the three historical aggregate-budget fixes. A compile failure is not a reproduced behavior failure. | The driver runs the current regression tests. `scripts/replay_historical_budgets.py` also requires four real historical assertion failures followed by four passes. |
| Run the same extraction verification locally and in CI. | `python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction` in the extraction verification job. |

Keep device, decoder, memory, and performance claims limited to evidence actually collected. The extraction driver does not establish those claims.
