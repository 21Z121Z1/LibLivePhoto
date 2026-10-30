mod common;
use liblivephoto::*;

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}
fn shared_curve() -> Vec<u8> {
    let mut bytes = vec![0, 0x40, 0x27, 0x10, 0x2f]; // baseline 1, two rules, custom space, shared mix/curve.
    bytes.extend(words(&[
        35400, 14600, 8500, 39850, 6550, 2300, 15635, 16450,
    ]));
    bytes.extend(words(&[20000]));
    bytes.push(0xe9); // custom mix, six flags 101001.
    bytes.extend(words(&[100, 2000, 65535]));
    bytes.push(8); // two x values; explicit slopes.
    bytes.extend(words(&[0, 1000, 0, 10000, 18000, 27000]));
    bytes.extend(words(&[5000, 1000, 2000, 18000, 9000]));
    bytes
}

#[test]
fn all_shared_rules_custom_chromaticities_mix_values_and_slopes_are_parsed() {
    let bytes = shared_curve();
    let data = decode_agtm(&bytes).unwrap();
    assert_eq!(data.raw_bytes, bytes);
    assert_eq!(data.rules.len(), 2);
    assert!(data.common_component_mix && data.common_curve);
    assert_eq!(data.color_space_mode, Some(3));
    assert!((data.custom_chromaticities.unwrap()[0] - 0.708).abs() < 1e-6);
    let first = &data.rules[0];
    let second = &data.rules[1];
    assert_eq!(first.component_mix.component_type, 3);
    assert_eq!(
        first.component_mix.encoded_coefficients,
        [Some(100), None, Some(2000), None, None, Some(65535)]
    );
    assert_eq!(first.component_mix, second.component_mix);
    assert_eq!(
        first.curve.iter().map(|p| p.x).collect::<Vec<_>>(),
        vec![0.0, 1.0]
    );
    assert_eq!(first.curve[1].y, 1.0);
    assert_eq!(second.curve[0].y, -0.1);
    assert_eq!(first.curve[0].encoded_slope_angle, Some(18000));
    assert!(first.curve[0].slope.unwrap().abs() < 1e-6);
    assert!((first.curve[1].slope.unwrap() - 1.0).abs() < 1e-5);
    assert!((second.curve[1].slope.unwrap() + 1.0).abs() < 1e-5);
    assert_eq!(data.consumed_bits, bytes.len() * 8);
    assert_eq!(data.details_json()["rules"].as_array().unwrap().len(), 2);
    assert_eq!(
        data.reference_json()["headroomAdaptiveToneMap"]["curve"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

fn maximum_curves() -> Vec<u8> {
    let mut bytes = vec![0, 0x40, 0x27, 0x10, 0x40]; // four independent rules, preset space 0.
    for index in 0..4 {
        bytes.extend(words(&[20000 + index * 1000]));
        bytes.push((index % 3) as u8 * 64);
        bytes.push(0xfc); // 32 points, PCHIP.
        bytes.extend(words(&(0..32).map(|i| i * 1000).collect::<Vec<_>>()));
        bytes.extend(words(&(0..32).map(|i| i * 500).collect::<Vec<_>>()));
    }
    bytes
}
#[test]
fn every_rule_and_maximum_point_count_is_preserved_with_pchip_absence() {
    let bytes = maximum_curves();
    let parsed = decode_agtm(&bytes).unwrap();
    assert_eq!(parsed.rules.len(), 4);
    for rule in &parsed.rules {
        assert_eq!(rule.curve.len(), 32);
        assert!(rule.use_pchip_slope);
        assert!(rule
            .curve
            .iter()
            .all(|p| p.slope.is_none() && p.encoded_slope_angle.is_none()));
    }
    for prefix in 0..bytes.len() {
        assert!(
            decode_agtm(&bytes[..prefix]).is_err(),
            "accepted prefix {prefix}"
        );
    }
}

#[test]
fn overlapping_agtm_samples_have_aggregate_count_payload_and_curve_budgets() {
    let config = [5, 0xb5, 0, 0x90, 0, 1];
    let at = MediaTime::new(0, 1).unwrap();
    let allowed = common::overlapping_metadata_movie(b"it35", &config, &[0, 0], 4096);
    assert_eq!(extract_agtm_from_video(&allowed, at).unwrap().len(), 4096);
    let mut large = vec![0, 0];
    large.resize(65_536, 0xaa); // Preserved extension bytes count in every sample.
    for (payload, count) in [(vec![0, 0], 4097), (large, 257), (maximum_curves(), 497)] {
        let movie = common::overlapping_metadata_movie(b"it35", &config, &payload, count);
        assert_eq!(metadata_packets(&movie, 2).unwrap().len(), count);
        assert!(matches!(
            extract_agtm_from_video(&movie, at),
            Err(MotionPhotoReadError::InputLimit)
        ));
    }
}

#[test]
fn complete_decoder_rejects_truncation_and_unknown_versions_without_fabrication() {
    let bytes = shared_curve();
    for prefix in 0..bytes.len() {
        assert!(decode_agtm(&bytes[..prefix]).is_err());
    }
    assert!(decode_agtm(&[0, 0x40, 0x27, 0x10, 0x50]).is_err()); // >4 rules.
    assert!(decode_agtm(&[0x20, 0]).is_err());
    assert!(decode_agtm(&[0, 0x80, 0, 0]).is_err());
    assert!(matches!(
        decode_agtm(&vec![0; 65_537]),
        Err(MotionPhotoReadError::InputLimit)
    ));
    let reference_white = decode_agtm(&[0, 0x40, 0x27, 0x10, 0x80]).unwrap();
    assert!(reference_white.rules.is_empty());
    assert_eq!(reference_white.consumed_bits, 33);
    assert!(decode_agtm(&[0, 0]).unwrap().rules.is_empty());
    let wrapped = [vec![5, 0xb5, 0, 0x90, 0, 1], bytes.clone()].concat();
    let parsed = decode_agtm(&wrapped).unwrap();
    assert_eq!(parsed.raw_bytes, wrapped);
    assert_eq!(parsed.rules, decode_agtm(&bytes).unwrap().rules);
    let mut extended = bytes.clone();
    extended.extend_from_slice(b"opaque future extension");
    let parsed = decode_agtm(&extended).unwrap();
    assert_eq!(parsed.raw_bytes, extended);
    assert_eq!(parsed.consumed_bits, bytes.len() * 8);
}

#[test]
fn standalone_movie_and_motion_photo_return_the_same_complete_metadata() {
    let bytes = shared_curve();
    let movie = common::metadata_movie(b"it35", &[5, 0xb5, 0, 0x90, 0, 1], &[(&bytes, 2000)], 0);
    let raw = extract_agtm_from_video(&movie, MediaTime::new(1, 1).unwrap()).unwrap();
    assert_eq!(raw[0].metadata().unwrap().rules.len(), 2);
    let still = vec![255, 216, 255, 217];
    let photo = MotionPhoto::parse(
        Input::Parts {
            still: &still,
            movie: &movie,
            presentation: MediaTime::new(1, 1),
        },
        ParseOptions::strict(),
    )
    .unwrap();
    assert_eq!(photo.agtm_at(MediaTime::new(1, 1).unwrap()).unwrap(), raw);
    let bad = common::metadata_movie(
        b"it35",
        &[5, 0xb5, 0, 0x90, 0, 1],
        &[(&bytes[..bytes.len() - 1], 2000)],
        0,
    );
    assert!(extract_agtm_from_video(&bad, MediaTime::new(1, 1).unwrap()).is_err());
}
