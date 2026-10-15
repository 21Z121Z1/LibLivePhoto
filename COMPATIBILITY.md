# Compatibility and conversion

Read, write, validation and target-consumer status are separate fields.
Runtime reports use `Unverified` for target consumers. A successful parser or
writer does not establish Camera, Gallery, Photos, editing or device acceptance.

`TargetProfile` selects source preservation, Android JPEG, Android HEIF, Apple
pairing, or the limited Samsung JPEG SEF contract. `WritePolicy` selects pairing
identity and the treatment of unmapped resources. No writer transcodes media.
A JPEG/HEIF container change reports `TranscodeRequired` and blocks execution.

The default policy rejects resources without an evidenced native mapping.
`PreserveSidecars` retains exact resource bytes and reports that the target's native
graph cannot express them. `DropExplicitly` records the drop. Sidecar preservation
does not imply that a Gallery can use the auxiliary resource.

Reports distinguish preserved, rewritten, generated, unmapped, dropped and
transcode-required resources. Preservation evidence names the property that was
compared: complete source bytes, video container bytes, encoded media payloads,
exact presentation time or extracted resource bytes. Do not promote this evidence
to a claim about all vendor semantics.

Apple conversion requires an explicit pairing identifier and known exact time.
Android conversion requires an exact integer microsecond timestamp when time is
known. No writer silently rounds time. The Samsung SEF writer currently requires
unknown time; a source with known time must use another target or source copy.

Before device verification, bind evidence to the original sample digest, device,
OS, Camera and Gallery versions, output digest and active writer policy. Verify
recognition, playback, selected video, cover time, auxiliary use, edit/save/export
and resource survival separately. A transferred filename or UI badge is insufficient.

Missing portrait samples prevent a mapping of Xiaomi `depthMotionPhoto`, depth
payload and subvideo relationships. Missing original vendor components and devices
prevent consumer acceptance claims. These are external evidence gaps, not parser
successes or implied support.

## Cross-format execution contracts

The public `plan` and `execute` APIs accept resources from every implemented reader.
The following limits apply before an OEM acceptance claim is possible.

| Source | Target profile | Automated evidence | Native consumer |
| --- | --- | --- | --- |
| Android JPEG, including evidenced ColorOS 15/16, Xiaomi and vivo files | Apple pair | Original fixtures; encoded movie payload and exact time comparisons; explicit sidecars for unmapped resources | Unverified |
| Android HEIF and evidenced Samsung HEIF | Apple pair | Original fixtures; image item and encoded movie payload comparisons | Unverified |
| Apple JPEG pair | Android JPEG | Synthetic Android → Apple → Android → Apple; unchanged video container on the Android leg, exact time and encoded media comparisons | Unverified |
| Apple HEIF pair | Android HEIF | Existing HEIF writer; only times exactly expressible in integer microseconds are executable | Unverified |
| Parsed JPEG with no known presentation time | Samsung JPEG SEF | Bounded SEF direct-record writer and contract tests | Unverified |
| Any implemented reader | Original source | Complete source byte comparisons | Does not establish cross-vendor recognition |
| Apple or another vendor | ColorOS private, HyperOS private, vivo private | Dedicated private writers are not implemented | Unverified |

An Apple time such as 1/3 second cannot be expressed exactly in Android integer
microseconds. The Android plan blocks it even when sidecars or explicit drops are
allowed. Those policies do not authorize changing a known time. A still-container
change also blocks until the caller supplies transcoded input. No codec backend,
HDR conversion, implicit timestamp rounding or vendor field inference runs here.

`cargo run -p liblivephoto --example convert -- PROFILE NEW_OUTPUT_DIR STILL
[MOVIE] [--pair-id ID] [--sidecars|--allow-drop]` is a complete standalone consumer.
Profiles are `preserve`, `google-jpeg`, `google-heif`, `apple` and
`samsung-jpeg-sef`. The example prints the plan before execution, validates output
through the library's reader, and publishes only into a new directory. It rejects
unmapped resources by default. Sidecars are separate files with resource IDs;
`report.txt` describes their disposition. A new Apple pair requires a caller-owned
unique pairing identifier. The example is not an atomic production publisher.

Device acceptance must record the source and output SHA-256, target profile and
policy, device, OS, Camera and Gallery versions. Record recognition, playback,
cover display, cover selection, edit, save and export independently, then repeat
resource/time/HDR checks on exported files. Until such a run exists, each field
remains unverified. The example and all CI jobs provide structural evidence only.
