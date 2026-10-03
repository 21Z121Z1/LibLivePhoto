mod common;
use liblivephoto::*;
fn sample() -> Vec<u8> {
    common::sef(&[
        (0x0a33, "MotionPhoto_AutoPlay", common::movie(1000, 1)),
        (0x1234, "Unspecified", b"opaque".to_vec()),
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
    ])
}
#[test]
fn sef_roles_are_not_inferred_from_vendor_names_and_payloads_survive_round_trip() {
    let input = sample();
    let p = MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::compatible()).unwrap();
    assert_eq!(p.asset().dialect, VendorDialect::SamsungSef);
    assert_eq!(p.presentation_time(), None);
    let r = p
        .resources()
        .find(|r| r.vendor_role.as_deref() == Some("MotionPhoto_AutoPlay"))
        .unwrap();
    assert_eq!(r.role, ResourceRole::UnknownVideo);
    assert_eq!(r.relationship.as_ref().unwrap().target, p.motion_video().id);
    assert!(p
        .asset()
        .containers
        .iter()
        .any(|c| c.resource == r.id && c.tracks.len() == 1));
    assert!(MotionPhoto::parse(Input::SingleFile(&input), ParseOptions::strict()).is_err());
    let policy = WritePolicy {
        unmapped: UnmappedPolicy::PreserveSidecars,
        ..WritePolicy::default()
    };
    let output = p
        .plan(TargetProfile::SamsungJpegSef, policy)
        .execute()
        .unwrap();
    let new = MotionPhoto::parse(
        Input::SingleFile(&output.files[0].bytes),
        ParseOptions::compatible(),
    )
    .unwrap();
    assert_eq!(
        new.motion_video_bytes().unwrap(),
        p.motion_video_bytes().unwrap()
    );
    for name in ["MotionPhoto_AutoPlay", "Unspecified"] {
        let old = p
            .resources()
            .find(|r| r.vendor_role.as_deref() == Some(name))
            .unwrap();
        let r = new
            .resources()
            .find(|r| r.vendor_role.as_deref() == Some(name))
            .unwrap();
        let mut a = Vec::new();
        let mut b = Vec::new();
        p.extract(old, &mut a).unwrap();
        new.extract(r, &mut b).unwrap();
        assert_eq!(a, b);
    }
}
#[test]
fn malformed_sef_lengths_offsets_overlap_and_duplicate_primary_fail() {
    let valid = sample();
    let size =
        u32::from_le_bytes(valid[valid.len() - 8..valid.len() - 4].try_into().unwrap()) as usize;
    let start = valid.len() - 8 - size;
    for (offset, bytes) in [
        (valid.len() - 8, vec![255; 4]),
        (start + 8, vec![255; 4]),
        (start + 16, vec![255; 4]),
        (start + 20, vec![255; 4]),
        (start + 12, vec![1, 0, 0, 0]),
    ] {
        let mut d = valid.clone();
        d[offset..offset + bytes.len()].copy_from_slice(&bytes);
        assert!(MotionPhoto::parse(Input::SingleFile(&d), ParseOptions::compatible()).is_err());
    }
    let duplicated = common::sef(&[
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
        (0x0a30, "MotionPhoto_Data", common::movie(1000, 1)),
    ]);
    assert!(
        MotionPhoto::parse(Input::SingleFile(&duplicated), ParseOptions::compatible()).is_err()
    );
}
