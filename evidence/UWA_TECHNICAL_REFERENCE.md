# Public UWA technical reference, not an implemented profile

This is proposal-summary evidence, separate from
[normative Google/XMP/HEIF mappings](NORMATIVE_CONFORMANCE.md).
It is not a UWA compliance claim. N0124 and N0176 are project identifiers, not
verified published standard numbers. No field-level draft or final standard was
obtained here; no restricted working-group material is accessed.

The public [HDR dynamic-image encapsulation project application](https://www.theuwa.com/upload/ueditor/file/20241211/1733884198930823/f58427e2129082e26cc3f7fdc5dc4824.pdf)
was actually read through the public PDF reader. It describes a common
still/movie packaging effort and compatibility goals. The
[announcement](https://www.theuwa.com/new-detail/592) was not accessible through
this reader. Public minutes are linked in the
[November2024 thread](https://maillist.theuwa.com/hyperkitty/list/std@maillist.theuwa.com/thread/TCDN6GQBVJQKTJW5VVT7ONNRYPC3QKE5/)
and [June2025 thread](https://maillist.theuwa.com/hyperkitty/list/std@maillist.theuwa.com/thread/JQHOOCLDP6WN4K6VUAGOPR6ZIWZAAQ2O/).
Those thread attachment listings were read; the PDF attachment bodies returned
unsupported `application/octet-stream` through this reader. The technical
summaries below retain the parent task's inspected-public-minutes findings;
they are not described as local attachment-body verification. The user's later
priority is the supplied ISO text, so further UWA document retrieval is deferred.

| Public proposal summary / authority | Existing analogous functionality | Difference / support status |
| --- | --- | --- |
| N0124, attachment7, UWA-OUT-V-2024112802: single-file still plus tail movie/mpvd | Google JPEG directory and HEIF/AVIF mpvd parse/build; exact resource ranges | Similar container concepts do not establish identical namespaces, length rules or native Gallery support. No new UWA parser/writer is claimed. |
| N0124: XMP position, length and format declarations | XMP document, bounded directory parsing, MIME/range validation | Exact field names/URIs/versioning are not available from a summary; none are guessed. |
| N0124: private secondary boxes at the movie tail | Unknown resource/box preservation and explicit unmapped policy | Unknown bytes remain unknown; they are not assigned vendor semantics or labelled depth. A preserved tail is not proof a UWA consumer accepts it. |
| N0124: JPEG/HEIF SDR/HDR plus MP4 | Lossless coded-data preservation, CICP/ICC/transform facts and AGTM extraction | No universal HDR rendering or UWA color/payload compliance claim. |
| N0176, attachment4, UWA-OUT-V-2025061201: system integration; June13,2025 requirements accepted pending initiation; investigate iOS interfaces and avoid fragmentation | Public Apple adapters and independent Google resource model | Research direction only. Existing PhotoKit acceptance gates remain separate; no private iOS API, standard-number inference or new integration protocol. |

Further implementation requires authorized field-level text and testable
requirements. The current code remains bounded to the declared Google/legacy,
vendor and Apple profiles, with synthetic versus original-device acceptance
explicitly distinguished.
