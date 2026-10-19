use liblivephoto::*;

fn profile() -> Vec<u8> {
    // Two distinct private tags share one11-byte unknown data element; the
    // original three-byte payload and final one-byte pad are kept exactly.
    let mut p = vec![0; 168];
    p[..4].copy_from_slice(&168u32.to_be_bytes());
    p[8..12].copy_from_slice(&[4, 0x40, 0, 0]);
    p[36..40].copy_from_slice(b"acsp");
    p[128..132].copy_from_slice(&2u32.to_be_bytes());
    for (i, sig) in [b"prv1", b"prv2"].iter().enumerate() {
        let at = 132 + i * 12;
        p[at..at + 4].copy_from_slice(*sig);
        p[at + 4..at + 8].copy_from_slice(&156u32.to_be_bytes());
        p[at + 8..at + 12].copy_from_slice(&11u32.to_be_bytes());
    }
    p[156..160].copy_from_slice(b"unkn");
    p[164..167].copy_from_slice(b"raw");
    p
}

#[test]
fn shared_unknown_tags_are_borrowed_in_table_order_without_rendering_claim() {
    let p = profile();
    let before = p.clone();
    let r = inspect_icc_structure(&p).unwrap();
    assert!(r.version_supported);
    assert_eq!(r.shared_references, 1);
    assert_eq!(r.tags[0].raw, r.tags[1].raw);
    assert_eq!(r.tags[0].raw.as_ptr(), r.tags[1].raw.as_ptr());
    assert_eq!(&r.tags[1].raw[8..], b"raw");
    assert_eq!(r.tags[0].signature, *b"prv1");
    assert_eq!(p, before);
    assert!(validate_icc_structure(&p)
        .unwrap()
        .checked_rules_satisfied());
    let mut other = p;
    other[9] = 0x30;
    assert!(!inspect_icc_structure(&other).unwrap().version_supported);
    let report = validate_icc_structure(&other).unwrap();
    assert!(!report.checked_rules_satisfied());
    assert_eq!(report.issues[0].kind, FormatIssueKind::Unsupported);
}

#[test]
fn malformed_headers_tables_overlaps_gaps_padding_and_overflow_are_explicit() {
    let p = profile();
    for len in [0, 127, 128, 131, 155, 167] {
        assert!(inspect_icc_structure(&p[..len]).is_err());
    }
    for (at, bytes) in [
        (0, 172u32.to_be_bytes()),
        (36, *b"fake"),
        (64, 4u32.to_be_bytes()),
        (140, 10u32.to_be_bytes()),
        (148, 160u32.to_be_bytes()),
        (148, u32::MAX.to_be_bytes()),
        (152, u32::MAX.to_be_bytes()),
        (144, *b"prv1"),
        (136, 128u32.to_be_bytes()),
        (128, 4097u32.to_be_bytes()),
    ] {
        let mut bad = p.clone();
        bad[at..at + 4].copy_from_slice(&bytes);
        assert!(inspect_icc_structure(&bad).is_err(), "offset{at}");
    }
    for at in [10, 100, 127, 160, 167] {
        let mut bad = p.clone();
        bad[at] = 1;
        assert!(inspect_icc_structure(&bad).is_err(), "offset{at}");
    }
    let mut trailing = p;
    trailing.extend([0; 4]);
    trailing[..4].copy_from_slice(&172u32.to_be_bytes());
    assert!(inspect_icc_structure(&trailing).is_err());
}

#[test]
fn unordered_distinct_tag_extents_are_valid_but_partial_sharing_is_not() {
    let mut p = profile();
    p.resize(176, 0);
    p[..4].copy_from_slice(&176u32.to_be_bytes());
    p[136..140].copy_from_slice(&164u32.to_be_bytes());
    p[148..152].copy_from_slice(&156u32.to_be_bytes());
    p[152..156].copy_from_slice(&8u32.to_be_bytes());
    p[156..].fill(0);
    p[156..160].copy_from_slice(b"one!");
    p[164..168].copy_from_slice(b"two!");
    p[172..175].copy_from_slice(b"raw");
    let r = inspect_icc_structure(&p).unwrap();
    assert_eq!(r.shared_references, 0);
    assert_eq!(r.tags[0].extent.lower_bound, 164);
    assert_eq!(r.tags[1].extent.lower_bound, 156);
    let mut overlapping = profile();
    overlapping[156..].fill(0);
    overlapping[148..152].copy_from_slice(&160u32.to_be_bytes());
    overlapping[152..156].copy_from_slice(&8u32.to_be_bytes());
    assert!(inspect_icc_structure(&overlapping)
        .unwrap_err()
        .to_string()
        .contains("overlaps"));
    assert!(matches!(
        inspect_icc_structure(&vec![0; 16 * 1024 * 1024 + 1]),
        Err(MotionPhotoReadError::InputLimit)
    ));
}
