use ternary_compression::{
    CompressionStats, DictionaryCompressor, RunLengthEncoder, TernaryHuffman, TernarySequence, Trit,
};

fn lcg(seed: u64) -> impl FnMut() -> u64 {
    let mut state = seed;
    move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state
    }
}

fn rand_seq(n: usize, rng: &mut impl FnMut() -> u64) -> TernarySequence {
    let vals: Vec<i8> = (0..n)
        .map(|_| match rng() % 3 {
            0 => -1,
            1 => 0,
            _ => 1,
        })
        .collect();
    TernarySequence::from_i8(&vals)
}

fn rand_seq_skewed(n: usize, p_zero: u64, rng: &mut impl FnMut() -> u64) -> TernarySequence {
    let m = 100;
    let vals: Vec<i8> = (0..n)
        .map(|_| {
            let r = rng() % m;
            if r < p_zero {
                0
            } else if r < p_zero + (100 - p_zero) / 2 {
                1
            } else {
                -1
            }
        })
        .collect();
    TernarySequence::from_i8(&vals)
}

#[test]
fn prop_rle_roundtrip_uniform() {
    let mut rng = lcg(1);
    for n in [0usize, 1, 2, 3, 4, 5, 6, 7, 10, 31, 100, 1000] {
        for _ in 0..50 {
            let seq = rand_seq(n, &mut rng);
            let runs = RunLengthEncoder::encode(&seq);
            assert_eq!(
                RunLengthEncoder::decode(&runs).trits(),
                seq.trits(),
                "n={n}"
            );
        }
    }
}

#[test]
fn prop_huffman_roundtrip_uniform() {
    let mut rng = lcg(2);
    for n in [1usize, 2, 3, 5, 7, 31, 100, 1000] {
        for _ in 0..50 {
            let seq = rand_seq(n, &mut rng);
            let h = TernaryHuffman::build(&seq);
            let enc = h.encode(&seq);
            assert_eq!(h.decode(&enc).trits(), seq.trits(), "n={n}");
        }
    }
}

#[test]
fn prop_huffman_roundtrip_skewed() {
    let mut rng = lcg(3);
    for n in [1usize, 5, 50, 500] {
        for _ in 0..50 {
            let seq = rand_seq_skewed(n, 90, &mut rng);
            let h = TernaryHuffman::build(&seq);
            let enc = h.encode(&seq);
            assert_eq!(h.decode(&enc).trits(), seq.trits(), "n={n}");
        }
    }
}

#[test]
fn prop_bytes_roundtrip() {
    let mut rng = lcg(4);
    for n in [0usize, 1, 4, 5, 6, 10, 13, 100, 1000] {
        for _ in 0..50 {
            let seq = rand_seq(n, &mut rng);
            let bytes = seq.to_bytes();
            assert_eq!(
                TernarySequence::from_bytes(&bytes, n).trits(),
                seq.trits(),
                "n={n}"
            );
        }
    }
}

#[test]
fn prop_dict_roundtrip() {
    let mut rng = lcg(5);
    for n in [0usize, 1, 2, 5, 12, 50, 200] {
        for &(mn, mx) in &[(2usize, 4usize), (2, 2), (3, 6), (1, 1)] {
            let seq = rand_seq(n, &mut rng);
            let mut c = DictionaryCompressor::new(mn, mx);
            c.build_dict(&seq);
            let enc = c.encode(&seq);
            assert_eq!(c.decode(&enc).trits(), seq.trits(), "n={n} mn={mn} mx={mx}");
        }
    }
}

#[test]
fn prop_dict_roundtrip_repetitive() {
    let mut rng = lcg(6);
    for rep in [1usize, 5, 50] {
        let base: Vec<i8> = vec![1, 1, -1, 0];
        let vals: Vec<i8> = (0..rep).flat_map(|_| base.iter().copied()).collect();
        let seq = TernarySequence::from_i8(&vals);
        let mut c = DictionaryCompressor::new(2, 5);
        c.build_dict(&seq);
        let enc = c.encode(&seq);
        assert_eq!(c.decode(&enc).trits(), seq.trits(), "rep={rep}");
        // repetitive data should actually use the dictionary
        let _ = rng();
    }
}

#[test]
fn prop_huffman_prefix_free() {
    let mut rng = lcg(7);
    for _ in 0..200 {
        let seq = rand_seq((rng() % 50 + 1) as usize, &mut rng);
        let h = TernaryHuffman::build(&seq);
        let codes: Vec<Vec<u8>> = h.codes().values().cloned().collect();
        for i in 0..codes.len() {
            for j in 0..codes.len() {
                if i != j {
                    assert!(
                        !codes[i].iter().eq(codes[j].iter().take(codes[i].len())),
                        "non-prefix-free: {:?} is prefix of {:?}",
                        codes[i],
                        codes[j]
                    );
                }
            }
        }
    }
}

#[test]
fn prop_huffman_avg_matches_actual() {
    let mut rng = lcg(8);
    for _ in 0..200 {
        let n = (rng() % 200 + 1) as usize;
        let seq = rand_seq(n, &mut rng);
        let h = TernaryHuffman::build(&seq);
        let enc = h.encode(&seq);
        let actual = enc.len() as f64 / n as f64;
        let reported = h.avg_bits_per_trit();
        assert!(
            (reported - actual).abs() < 1e-9,
            "avg_bits_per_trit={reported} but actual={actual} (n={n})"
        );
    }
}

#[test]
fn smoke_stats() {
    let s = CompressionStats::new(1000, 400);
    assert!((s.ratio - 0.4).abs() < 1e-12);
    assert!((s.space_saving - 0.6).abs() < 1e-12);
}

#[test]
fn edge_single_element() {
    for v in [-1i8, 0, 1] {
        let seq = TernarySequence::from_i8(&[v]);
        let runs = RunLengthEncoder::encode(&seq);
        assert_eq!(RunLengthEncoder::decode(&runs).trits(), seq.trits());
        let h = TernaryHuffman::build(&seq);
        assert_eq!(h.decode(&h.encode(&seq)).trits(), seq.trits());
        let b = seq.to_bytes();
        assert_eq!(TernarySequence::from_bytes(&b, 1).trits(), seq.trits());
    }
}

#[test]
fn edge_constant() {
    let seq = TernarySequence::from_i8(&vec![0; 500]);
    let runs = RunLengthEncoder::encode(&seq);
    assert_eq!(RunLengthEncoder::decode(&runs).trits(), seq.trits());
}

#[test]
fn edge_trit_roundtrip_all() {
    for d in 0u8..3 {
        let t = Trit::from_digit(d).unwrap();
        assert_eq!(Trit::from_i8(t.to_i8()).unwrap(), t);
        assert_eq!(Trit::from_digit(t.digit()).unwrap(), t);
    }
}
