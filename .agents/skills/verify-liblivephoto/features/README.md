# Extraction feature map

This map covers the library, interface, and CLI behavior touched by the bounded resource-extraction change.

| Feature | Proof |
| --- | --- |
| [Library resource bytes](library-resources.md) | Core reader tests and ordered multi-extent interface extraction. |
| [Shared operation response](interface-extraction.md) | Unknown resources, request order, defaults, errors, LLP1 length, and scaled public-path replay. |
| [CLI publication](cli-extraction.md) | Build, check, byte-exact extraction, and refusal to overwrite through the real binary. |

The common command is `python3 scripts/verify_extraction.py run --evidence evidence/local/verify-extraction`. All entries run in its drive stage.
