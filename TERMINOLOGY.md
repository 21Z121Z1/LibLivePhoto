# Terminology

| Term | Definition |
| --- | --- |
| Asset | Resources with declared or observed identity, time or semantic relationships. |
| Source | One immutable input byte sequence. An asset can have multiple sources. |
| Resource | An extractable view of one source, with ordered byte extents and a role. |
| Extent | A half-open byte interval `[lower_bound, upper_bound)`. |
| Container | The physical JPEG, HEIF or BMFF structure of a resource. |
| Track | A BMFF track inside a video container. Track identity is independent of resource identity. |
| Layout | Physical arrangement of resources in sources. |
| Format | The standard resource and metadata contract. |
| Dialect | Vendor or historical interpretation beyond the standard contract. |
| Provenance | The basis of a fact: declared, observed, recovered or inferred. |
| Presentation time | The exact media time associated with the still image. |
| Pairing identifier | A declared identity shared by resources. A filename is not an identifier. |
| Secondary video | A video distinct from the primary video; the name does not define its purpose. |
| Substitution video | A video whose evidenced contract permits substitution for the still image. |
| Sidecar | An output that preserves bytes outside the native target resource graph. |
| Compatibility | Acceptance by a specified consumer and version, with stated evidence. |
| Preservation evidence | A byte or exact-time comparison made during conversion. |

Use `preserved` for an evidenced property. Do not use `lossless` for an entire
conversion when only media payload equality is known. Unknown vendor role names
must remain available even when no generic role is assigned.
