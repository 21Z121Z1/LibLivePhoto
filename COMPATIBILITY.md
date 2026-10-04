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
