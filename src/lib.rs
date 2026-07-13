//! # ternary-compression
//!
//! Compress ternary sequences using various algorithms.
//!
//! Provides:
//! - `RunLengthEncoder` — Run-length encoding for ternary sequences
//! - `TernaryHuffman` — Huffman coding adapted for ternary alphabets
//! - `DictionaryCompressor` — Dictionary-based compression for repeated patterns
//! - `TernarySequence` — Core ternary sequence type

use std::collections::HashMap;

/// Ternary digit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Trit {
    Neg,
    Zero,
    Pos,
}

impl Trit {
    pub fn to_i8(self) -> i8 {
        match self {
            Trit::Neg => -1,
            Trit::Zero => 0,
            Trit::Pos => 1,
        }
    }

    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Trit::Neg),
            0 => Some(Trit::Zero),
            1 => Some(Trit::Pos),
            _ => None,
        }
    }

    pub fn digit(self) -> u8 {
        match self {
            Trit::Neg => 0,
            Trit::Zero => 1,
            Trit::Pos => 2,
        }
    }

    pub fn from_digit(d: u8) -> Option<Self> {
        match d {
            0 => Some(Trit::Neg),
            1 => Some(Trit::Zero),
            2 => Some(Trit::Pos),
            _ => None,
        }
    }
}

/// A sequence of ternary values
#[derive(Debug, Clone)]
pub struct TernarySequence {
    trits: Vec<Trit>,
}

impl TernarySequence {
    pub fn new(trits: Vec<Trit>) -> Self {
        TernarySequence { trits }
    }

    pub fn from_i8(values: &[i8]) -> Self {
        TernarySequence {
            trits: values.iter().filter_map(|&v| Trit::from_i8(v)).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.trits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.trits.is_empty()
    }

    pub fn trits(&self) -> &[Trit] {
        &self.trits
    }

    pub fn get(&self, idx: usize) -> Option<Trit> {
        self.trits.get(idx).copied()
    }

    /// Convert to bytes (packing 5 trits per byte, since 3^5=243 < 256)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for chunk in self.trits.chunks(5) {
            let mut byte = 0u8;
            for (i, &trit) in chunk.iter().enumerate() {
                byte += trit.digit() * 3u8.pow(i as u32);
            }
            bytes.push(byte);
        }
        bytes
    }

    /// Convert from bytes back to ternary sequence
    pub fn from_bytes(bytes: &[u8], trit_count: usize) -> Self {
        let mut trits = Vec::with_capacity(trit_count);
        for &byte in bytes {
            let mut remaining = byte;
            for _ in 0..5 {
                if trits.len() >= trit_count {
                    break;
                }
                let digit = remaining % 3;
                trits.push(Trit::from_digit(digit).unwrap_or(Trit::Zero));
                remaining /= 3;
            }
        }
        TernarySequence::new(trits)
    }
}

// ─── Run-Length Encoding ───────────────────────────────────────────

/// Run-length encoding for ternary sequences
#[derive(Debug, Clone)]
pub struct RunLengthEncoder;

impl RunLengthEncoder {
    /// Encode a ternary sequence using RLE
    pub fn encode(seq: &TernarySequence) -> Vec<(Trit, usize)> {
        if seq.is_empty() {
            return Vec::new();
        }

        let mut runs = Vec::new();
        let mut current = seq.get(0).unwrap();
        let mut count = 1;

        for i in 1..seq.len() {
            let trit = seq.get(i).unwrap();
            if trit == current {
                count += 1;
            } else {
                runs.push((current, count));
                current = trit;
                count = 1;
            }
        }
        runs.push((current, count));
        runs
    }

    /// Decode RLE back to ternary sequence
    pub fn decode(runs: &[(Trit, usize)]) -> TernarySequence {
        let mut trits = Vec::new();
        for &(trit, count) in runs {
            for _ in 0..count {
                trits.push(trit);
            }
        }
        TernarySequence::new(trits)
    }

    /// Calculate compression ratio
    pub fn compression_ratio(seq: &TernarySequence) -> f64 {
        if seq.is_empty() {
            return 1.0;
        }
        let runs = Self::encode(seq);
        let original_size = seq.len() as f64;
        let encoded_size = runs.len() as f64 * 2.0; // (trit, count) pairs
        encoded_size / original_size
    }
}

// ─── Huffman Coding ────────────────────────────────────────────────

/// Huffman node for ternary data
#[derive(Debug, Clone)]
struct HuffmanNode {
    trit: Option<Trit>,
    freq: usize,
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
    fn leaf(trit: Trit, freq: usize) -> Self {
        HuffmanNode {
            trit: Some(trit),
            freq,
            left: None,
            right: None,
        }
    }

    fn internal(left: HuffmanNode, right: HuffmanNode) -> Self {
        HuffmanNode {
            trit: None,
            freq: left.freq + right.freq,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }
}

/// Huffman coding for ternary sequences
#[derive(Debug, Clone)]
pub struct TernaryHuffman {
    codes: HashMap<Trit, Vec<u8>>,
    expected_bits_per_trit: f64,
}

impl TernaryHuffman {
    /// Build Huffman codes from a ternary sequence
    pub fn build(seq: &TernarySequence) -> Self {
        // Actual empirical occurrence counts (used for the true expected length)
        let mut counts: HashMap<Trit, usize> = HashMap::new();
        for &trit in seq.trits() {
            *counts.entry(trit).or_insert(0) += 1;
        }
        let total = seq.len();

        // Frequencies for tree construction: ensure all three trits are present
        // so the code is complete and prefix-free even for skewed/degenerate input.
        let mut freq = counts.clone();
        for t in [Trit::Neg, Trit::Zero, Trit::Pos] {
            freq.entry(t).or_insert(1);
        }

        let mut nodes: Vec<HuffmanNode> = freq
            .into_iter()
            .map(|(trit, f)| HuffmanNode::leaf(trit, f))
            .collect();

        // Build tree
        while nodes.len() > 1 {
            // Sort by frequency (simple selection)
            nodes.sort_by_key(|n| n.freq);
            let left = nodes.remove(0);
            let right = nodes.remove(0);
            nodes.push(HuffmanNode::internal(left, right));
        }

        let mut codes = HashMap::new();
        if let Some(root) = nodes.into_iter().next() {
            Self::build_codes(&root, Vec::new(), &mut codes);
        }

        // Expected bits per trit under the empirical symbol distribution:
        // L_bar = sum_i p_i * l_i, with p_i = count_i / total. This equals the
        // actual number of bits emitted when encoding `seq`, divided by seq.len().
        let expected_bits_per_trit = if total == 0 {
            0.0
        } else {
            let bits: usize = codes
                .iter()
                .map(|(trit, code)| counts.get(trit).copied().unwrap_or(0) * code.len())
                .sum();
            bits as f64 / total as f64
        };

        TernaryHuffman {
            codes,
            expected_bits_per_trit,
        }
    }

    fn build_codes(node: &HuffmanNode, prefix: Vec<u8>, codes: &mut HashMap<Trit, Vec<u8>>) {
        if let Some(trit) = node.trit {
            codes.insert(trit, if prefix.is_empty() { vec![0] } else { prefix });
            return;
        }
        if let Some(ref left) = node.left {
            let mut p = prefix.clone();
            p.push(0);
            Self::build_codes(left, p, codes);
        }
        if let Some(ref right) = node.right {
            let mut p = prefix;
            p.push(1);
            Self::build_codes(right, p, codes);
        }
    }

    /// Encode sequence to bits
    pub fn encode(&self, seq: &TernarySequence) -> Vec<u8> {
        let mut bits = Vec::new();
        for &trit in seq.trits() {
            if let Some(code) = self.codes.get(&trit) {
                bits.extend_from_slice(code);
            }
        }
        bits
    }

    /// Decode bits back to ternary sequence
    pub fn decode(&self, bits: &[u8]) -> TernarySequence {
        // Build reverse lookup: bit pattern -> trit
        let mut reverse: Vec<(Vec<u8>, Trit)> = Vec::new();
        for (&trit, code) in &self.codes {
            reverse.push((code.clone(), trit));
        }

        let mut trits = Vec::new();
        let mut pos = 0;
        while pos < bits.len() {
            let mut found = false;
            for (code, trit) in &reverse {
                if pos + code.len() <= bits.len() && &bits[pos..pos + code.len()] == code.as_slice()
                {
                    trits.push(*trit);
                    pos += code.len();
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        TernarySequence::new(trits)
    }

    /// Get the code table
    pub fn codes(&self) -> &HashMap<Trit, Vec<u8>> {
        &self.codes
    }

    /// Expected bits per trit under the empirical distribution the code was
    /// built from: `L_bar = sum_i p_i * l_i`. Equals the actual number of bits
    /// emitted when encoding the build sequence, divided by its length.
    pub fn avg_bits_per_trit(&self) -> f64 {
        self.expected_bits_per_trit
    }
}

// ─── Dictionary Compression ────────────────────────────────────────

/// Dictionary entry
#[derive(Debug, Clone)]
pub struct DictEntry {
    pub pattern: Vec<Trit>,
    pub code: usize,
    pub freq: usize,
}

/// Dictionary-based compressor
#[derive(Debug, Clone)]
pub struct DictionaryCompressor {
    dict: Vec<DictEntry>,
    min_pattern_len: usize,
    max_pattern_len: usize,
}

impl DictionaryCompressor {
    pub fn new(min_pattern_len: usize, max_pattern_len: usize) -> Self {
        DictionaryCompressor {
            dict: Vec::new(),
            min_pattern_len,
            max_pattern_len,
        }
    }

    /// Build dictionary from training data
    pub fn build_dict(&mut self, seq: &TernarySequence) {
        let mut freq_map: HashMap<Vec<Trit>, usize> = HashMap::new();

        for len in self.min_pattern_len..=self.max_pattern_len {
            if len > seq.len() {
                break;
            }
            for i in 0..=seq.len() - len {
                let pattern: Vec<Trit> = (i..i + len).filter_map(|j| seq.get(j)).collect();
                *freq_map.entry(pattern).or_insert(0) += 1;
            }
        }

        // Keep patterns that appear more than once
        let mut entries: Vec<_> = freq_map
            .into_iter()
            .filter(|(_, f)| *f > 1)
            .map(|(pattern, freq)| DictEntry {
                pattern,
                code: 0,
                freq,
            })
            .collect();

        // Sort by frequency * length (prioritize long, frequent patterns)
        entries.sort_by_key(|b| std::cmp::Reverse(b.freq * b.pattern.len()));

        // Assign codes
        for (i, entry) in entries.iter_mut().enumerate() {
            entry.code = i + 3; // 0,1,2 reserved for single trits
        }

        self.dict = entries;
    }

    /// Encode using dictionary
    pub fn encode(&self, seq: &TernarySequence) -> Vec<usize> {
        let mut encoded = Vec::new();
        let mut i = 0;
        let trits = seq.trits();

        while i < trits.len() {
            let mut best_match = None;
            let mut best_len = 0;

            // Try longest dictionary match first
            for entry in &self.dict {
                if entry.pattern.len() > best_len
                    && i + entry.pattern.len() <= trits.len()
                    && &trits[i..i + entry.pattern.len()] == entry.pattern.as_slice()
                {
                    best_match = Some(entry.code);
                    best_len = entry.pattern.len();
                }
            }

            if let Some(code) = best_match {
                encoded.push(code);
                i += best_len;
            } else {
                // Single trit
                let trit = seq.get(i).unwrap();
                encoded.push(trit.digit() as usize);
                i += 1;
            }
        }
        encoded
    }

    /// Decode using dictionary
    pub fn decode(&self, encoded: &[usize]) -> TernarySequence {
        let mut trits = Vec::new();

        for &code in encoded {
            if code < 3 {
                // Single trit
                if let Some(trit) = Trit::from_digit(code as u8) {
                    trits.push(trit);
                }
            } else {
                // Dictionary lookup
                if let Some(entry) = self.dict.iter().find(|e| e.code == code) {
                    trits.extend_from_slice(&entry.pattern);
                }
            }
        }

        TernarySequence::new(trits)
    }

    pub fn dict_size(&self) -> usize {
        self.dict.len()
    }

    pub fn dict(&self) -> &[DictEntry] {
        &self.dict
    }
}

// ─── Compression Stats ─────────────────────────────────────────────

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub original_trits: usize,
    pub compressed_size: usize,
    pub ratio: f64,
    pub space_saving: f64,
}

impl CompressionStats {
    pub fn new(original_trits: usize, compressed_size: usize) -> Self {
        let ratio = if original_trits > 0 {
            compressed_size as f64 / original_trits as f64
        } else {
            1.0
        };
        let space_saving = (1.0 - ratio).max(0.0);
        CompressionStats {
            original_trits,
            compressed_size,
            ratio,
            space_saving,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_conversions() {
        assert_eq!(Trit::Neg.to_i8(), -1);
        assert_eq!(Trit::Zero.digit(), 1);
        assert_eq!(Trit::from_digit(2), Some(Trit::Pos));
    }

    #[test]
    fn test_sequence_from_i8() {
        let seq = TernarySequence::from_i8(&[-1, 0, 1]);
        assert_eq!(seq.len(), 3);
        assert_eq!(seq.get(0), Some(Trit::Neg));
        assert_eq!(seq.get(2), Some(Trit::Pos));
    }

    #[test]
    fn test_sequence_to_from_bytes() {
        let seq = TernarySequence::from_i8(&[-1, 0, 1, -1, 0, 1, -1, 0, 1, 0]);
        let bytes = seq.to_bytes();
        let recovered = TernarySequence::from_bytes(&bytes, 10);
        assert_eq!(seq.trits(), recovered.trits());
    }

    #[test]
    fn test_rle_basic() {
        let seq = TernarySequence::from_i8(&[1, 1, 1, -1, -1, 0]);
        let runs = RunLengthEncoder::encode(&seq);
        assert_eq!(runs, vec![(Trit::Pos, 3), (Trit::Neg, 2), (Trit::Zero, 1)]);
    }

    #[test]
    fn test_rle_roundtrip() {
        let seq = TernarySequence::from_i8(&[1, 1, 1, -1, -1, 0, 0, 0, 0]);
        let runs = RunLengthEncoder::encode(&seq);
        let decoded = RunLengthEncoder::decode(&runs);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_rle_empty() {
        let seq = TernarySequence::new(vec![]);
        let runs = RunLengthEncoder::encode(&seq);
        assert!(runs.is_empty());
    }

    #[test]
    fn test_rle_single() {
        let seq = TernarySequence::from_i8(&[1]);
        let runs = RunLengthEncoder::encode(&seq);
        assert_eq!(runs, vec![(Trit::Pos, 1)]);
    }

    #[test]
    fn test_rle_all_same() {
        let seq = TernarySequence::from_i8(&[0; 100]);
        let runs = RunLengthEncoder::encode(&seq);
        assert_eq!(runs, vec![(Trit::Zero, 100)]);
    }

    #[test]
    fn test_rle_compression_ratio() {
        let seq = TernarySequence::from_i8(&[1; 100]);
        let ratio = RunLengthEncoder::compression_ratio(&seq);
        assert!(ratio < 0.1); // Should be very small for long runs
    }

    #[test]
    fn test_huffman_build() {
        let seq = TernarySequence::from_i8(&[1, 1, 1, 1, -1, 0]);
        let huffman = TernaryHuffman::build(&seq);
        // Most frequent trit (Pos) should have shortest code
        let pos_len = huffman
            .codes()
            .get(&Trit::Pos)
            .map(|c| c.len())
            .unwrap_or(99);
        let neg_len = huffman
            .codes()
            .get(&Trit::Neg)
            .map(|c| c.len())
            .unwrap_or(99);
        assert!(pos_len <= neg_len);
    }

    #[test]
    fn test_huffman_roundtrip() {
        let seq = TernarySequence::from_i8(&[1, -1, 0, 1, -1, 0, 1, 1]);
        let huffman = TernaryHuffman::build(&seq);
        let encoded = huffman.encode(&seq);
        let decoded = huffman.decode(&encoded);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_huffman_avg_bits() {
        let seq = TernarySequence::from_i8(&[1, 1, 1, 1, 1, -1, 0]);
        let huffman = TernaryHuffman::build(&seq);
        let avg = huffman.avg_bits_per_trit();
        assert!(avg > 0.0 && avg <= 2.0);
    }

    #[test]
    fn test_huffman_single_trit() {
        let seq = TernarySequence::from_i8(&[1, 1, 1]);
        let huffman = TernaryHuffman::build(&seq);
        let encoded = huffman.encode(&seq);
        let decoded = huffman.decode(&encoded);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_dict_build() {
        let seq = TernarySequence::from_i8(&[1, 1, -1, 1, 1, -1, 0, 0, 0]);
        let mut compressor = DictionaryCompressor::new(2, 3);
        compressor.build_dict(&seq);
        assert!(compressor.dict_size() > 0);
    }

    #[test]
    fn test_dict_roundtrip() {
        let seq = TernarySequence::from_i8(&[1, 1, -1, 1, 1, -1, 0, 0, 0, 1, 1, -1]);
        let mut compressor = DictionaryCompressor::new(2, 4);
        compressor.build_dict(&seq);
        let encoded = compressor.encode(&seq);
        let decoded = compressor.decode(&encoded);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_dict_empty() {
        let seq = TernarySequence::new(vec![]);
        let mut compressor = DictionaryCompressor::new(2, 4);
        compressor.build_dict(&seq);
        let encoded = compressor.encode(&seq);
        assert!(encoded.is_empty());
    }

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats::new(100, 50);
        assert!((stats.ratio - 0.5).abs() < 0.01);
        assert!((stats.space_saving - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_compression_stats_zero() {
        let stats = CompressionStats::new(0, 0);
        assert!((stats.ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_sequence_empty() {
        let seq = TernarySequence::new(vec![]);
        assert!(seq.is_empty());
        assert_eq!(seq.len(), 0);
    }

    #[test]
    fn test_rle_alternating() {
        let seq = TernarySequence::from_i8(&[1, -1, 1, -1, 1, -1]);
        let runs = RunLengthEncoder::encode(&seq);
        assert_eq!(runs.len(), 6); // No compression for alternating
        let decoded = RunLengthEncoder::decode(&runs);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_huffman_long_sequence() {
        let values: Vec<i8> = (0..100)
            .map(|i| match i % 5 {
                0 => 1,
                1 => 1,
                2 => 1,
                3 => 0,
                _ => -1,
            })
            .collect();
        let seq = TernarySequence::from_i8(&values);
        let huffman = TernaryHuffman::build(&seq);
        let encoded = huffman.encode(&seq);
        let decoded = huffman.decode(&encoded);
        assert_eq!(seq.trits(), decoded.trits());
    }

    #[test]
    fn test_dict_no_repeat_patterns() {
        let seq = TernarySequence::from_i8(&[1, 0, -1, 1, 0, -1, 1, 0, -1]);
        let mut compressor = DictionaryCompressor::new(3, 3);
        compressor.build_dict(&seq);
        let encoded = compressor.encode(&seq);
        let decoded = compressor.decode(&encoded);
        assert_eq!(seq.trits(), decoded.trits());
    }
}
