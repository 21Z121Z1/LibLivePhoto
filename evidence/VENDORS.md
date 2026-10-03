# Vendor and platform evidence

The matrix separates producer, consumer, bytes and device evidence. The component
versions of a fixture must not be inferred from its device name or EXIF software.
An unknown version remains unknown until a component package or runtime identifies it.

| Family and version | Producer evidence | Consumer evidence | Original byte evidence | Limit |
| --- | --- | --- | --- | --- |
| Apple iPhone 15; component version unknown | Original HEIC/MOV pair | Public Photos resource-pair API | Matching identity, 820/600 time, five tracks | No Photos import/edit/export run |
| Android format 1.0; Pixel 9 Pro XL components unknown | Official format contract and original file | Official substitution and presentation contract | JPEG, gain map, primary/secondary video and audio | No Google Photos device run |
| Samsung Galaxy S20/S20 FE; components unknown | Original SEF 106/107 files | Official Motion Photo playback/export procedures | Direct SEF video and HEIF pointer; versionless file | Camera/Gallery implementation and acceptance missing |
| Samsung A34/S23 Ultra/Tab S9; components unknown | Original SEF 107 files | Same documented feature; version-specific behavior unresolved | Direct JPEG and HEIF `mpvd`; `mpv3` record independent of `mpv2` pointer | No claim of identical firmware behavior |
| Samsung XDRemux JPEG corpus; components unknown | Original bytes | Selection between two videos unresolved | Primary `MotionPhoto_Data` and complete `MotionPhoto_AutoPlay` | Unknown generic purpose |
| OPlus ColorOS 15 | Original source fixtures and LPEX | Gallery component missing | Cover time, vendor matrices, opaque suffix | No dedicated Gallery compatibility writer |
| OPlus ColorOS 16 | Original source fixtures and LPEX | Gallery component missing | Two concatenated videos, vendor raw fields | Stream 2 purpose unresolved |
| Xiaomi SDK 0.0.1–0.0.4 snapshots | Official release history | Header/MediaStore detection and extraction APIs | One Android-style original | SDK binary requires authorized repository access |
| Xiaomi SDK 1.0.0–1.0.3 | Official client guide and release history | HyperOS Gallery and MediaStore requirements | Same original; no portrait/depth corpus | No independent depth/subvideo mapping |
| vivo Android-style single file; versions unknown | Two original fixtures | No original Gallery component | Standard directory plus opaque vendor bytes | Dual-file and portrait modes missing |
| HarmonyOS/OpenHarmony MovingPhoto | Official Camera/Media Library platform contracts | MovingPhotoView and MediaAssetManager APIs | No private on-disk original | Platform boundary only |
| Huawei/Honor private dialect | No sufficient file evidence | User guides describe product behavior | No matching original | No vendor parser or writer added |

## Primary sources

- [Android Motion Photo format 1.0](https://developer.android.com/media/platform/motion-photo-format):
  resources, XMP namespaces, HEIF `mpvd`, exact microsecond time and the optional
  secondary substitution track. The standard does not equate every OEM subvideo.
- [Apple Photos resource-pair API](https://developer.apple.com/documentation/photos/phlivephoto/request(withresourcefileurls:placeholderimage:targetsize:contentmode:resulthandler:)):
  the platform consumer receives still and video resources. This API is outside the library.
- [Samsung support](https://www.samsung.com/us/support/answer/ANS10004858/) and
  [Camera setting guide](https://www.samsung.com/ae/support/mobile-devices/enable-motion-photos-on-my-samsung-phone/):
  Camera generates Motion Photos; Gallery selects frames and exports media. These
  user procedures do not specify SEF offsets or the secondary video's purpose.
- Xiaomi [business guide](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=2000),
  [client guide](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=2003),
  [release history](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=2001):
  SDK 1.0.3 is documented on 2026-01-14; the client guide requires HyperOS Gallery
  versionCode above 407621 and system code at least 816 for third-party production.
  SDK 1.0.2 adds compatibility with the newer Google contract. These are documented
  requirements, not locally tested version combinations.
- OpenHarmony [asset operations](https://github.com/openharmony/docs/blob/master/en/application-dev/media/medialibrary/photoAccessHelper-movingphoto.md)
  and [MovingPhotoView](https://github.com/openharmony/docs/blob/master/en/application-dev/media/medialibrary/movingphotoview-guidelines.md):
  the application constructs a MovingPhoto object and the platform selects resources.
  This does not establish a Huawei private file trailer.

## Independent and binary research

[ExifTool Samsung](https://github.com/exiftool/exiftool/blob/2200871d9cef988051d2a99d67df3bda6cbb30a8/lib/Image/ExifTool/Samsung.pm)
and exact originals support [the independent SEF specification](SAMSUNG_SEF_SPEC.md).
Production code was written from that byte contract and synthetic vectors.

[VLiveConvert](https://github.com/brovast/VLiveConvert/tree/deec1e7c2470abefad63351247d31f8f04041d2f)
describes vivo footer variants, paired files, `streamdata` and Gallery selection.
Its portrait claims are independent implementation evidence. They are not promoted
to a verified role or writer without originals and a matching consumer.

The user-provided `OPCameraPro_v3.1.10.apk` identifies package
`com.tlsu.opluscamerapro`. It is a third-party camera component, not the OEM Camera
or Gallery. A partial jadx result has 17 decompilation errors. It cannot establish
the firmware producer/consumer contract. Decompiled source and APK bytes remain
in an ignored research directory. No code is translated into Rust.

The public sample repository permits analysis in its README but has no general
redistribution license. Its files remain outside Git; only paths and SHA-256 values
are recorded in [public-samples.json](public-samples.json). The owned XDRemux corpus
is referenced by a pinned revision and exact digests. No personal metadata is copied
into the evidence documents.

## Required missing evidence

No local or searched public corpus contains `MVIMG_194659`, `MVIMG_194654` or
`MVIMG_194703`. Obtain those unedited Xiaomi portrait originals with device,
HyperOS/MIUI, Camera and Gallery versions. They must distinguish a directory item,
HEIF item, internal video track, separate video and portrait depth payload.
The experiment must compare normal playback, portrait use and edit/save/export.

Original Camera/Gallery packages and matching devices are also needed for OPlus,
Samsung, Xiaomi and vivo consumer acceptance. Public docs and community reports do
not close these gaps. Authentication, DRM, encryption and access controls are not
bypassed to acquire components or SDKs.
