# ternary-compression

Compression algorithms for ternary sequences — run-length encoding, Huffman coding, dictionary compression, and compact byte packing for `{-1, 0, +1}` data.

## Why This Exists

Ternary data (three-valued sequences) has only 3 possible values per position, but a naive byte-per-element encoding wastes 99.2% of the representable space. Standard compressors (gzip, zstd) don't exploit the small alphabet. This crate provides compression methods tuned specifically for ternary data: RLE for runs, Huffman for skewed distributions, dictionary substitution for repeated patterns, and base-3 byte packing for compact representation.

## Core Concepts

- **Trit** — The fundamental unit: `Neg` (-1), `Zero` (0), or `Pos` (+1)
- **TernarySequence** — Core sequence type with byte packing (5 trits per byte, since 3⁵ = 243 < 256)
- **RunLengthEncoder** — Groups consecutive identical trits into (value, count) pairs
- **TernaryHuffman** — Frequency-weighted variable-length codes; most common trit gets shortest code
- **DictionaryCompressor** — Finds repeated multi-trit patterns in training data and replaces them with single codes

## Quick Start

```toml
# Cargo.toml
[dependencies]
ternary-compression = "0.1"
```

```rust
use ternary_compression::*;

let seq = TernarySequence::from_i8(&[1, 1, 1, -1, -1, 0, 0, 0, 0]);

// RLE compression
let runs = RunLengthEncoder::encode(&seq);
let decoded = RunLengthEncoder::decode(&runs);
assert_eq!(seq.trits(), decoded.trits());
let ratio = RunLengthEncoder::compression_ratio(&seq);

// Huffman coding
let huffman = TernaryHuffman::build(&seq);
let encoded = huffman.encode(&seq);
let decoded = huffman.decode(&encoded);
assert_eq!(seq.trits(), decoded.trits());
println!("Avg bits/trit: {:.2}", huffman.avg_bits_per_trit());

// Dictionary compression
let mut dict = DictionaryCompressor::new(2, 4);
dict.build_dict(&seq);
let encoded = dict.encode(&seq);
let decoded = dict.decode(&encoded);
assert_eq!(seq.trits(), decoded.trits());

// Compact byte packing
let bytes = seq.to_bytes();
let recovered = TernarySequence::from_bytes(&bytes, seq.len());
assert_eq!(seq.trits(), recovered.trits());
```

## API Overview

| Type / Function | Description |
|---|---|
| `Trit` | Enum: `Neg`, `Zero`, `Pos` with conversion helpers |
| `TernarySequence` | Sequence type with `to_bytes()` / `from_bytes()` packing |
| `RunLengthEncoder` | RLE encode/decode, compression ratio |
| `TernaryHuffman` | Huffman coding with `build()`, `encode()`, `decode()` |
| `DictionaryCompressor` | N-gram dictionary compression with training |
| `CompressionStats` | Ratio and space-saving statistics |

## How It Works

**Byte packing** stores 5 trits per byte using base-3 encoding: each group of up to 5 trits is converted to an integer in [0, 242] and stored as a single byte. This achieves 1.6× compression vs. byte-per-trit with no information loss.

**RLE** scans the sequence, grouping consecutive identical trits into (Trit, count) pairs. It excels on data with long runs (e.g., all-zero padding). Alternating sequences expand rather than compress.

**Huffman** builds a binary tree from symbol frequencies. With only 3 symbols, the tree has at most 2 levels — the most frequent trit gets a 1-bit code, the others get 2 bits. All three symbols are always included in the tree (frequency-1 minimum) to ensure decodability.

**Dictionary compression** scans training data for repeated n-gram patterns (configurable length range), sorts by `frequency × length`, assigns codes starting at 3 (reserving 0–2 for single trits), and replaces matching patterns during encoding.

## Known Limitations

- **No streaming API**: All compressors require the full `TernarySequence` in memory. `RunLengthEncoder::encode()`, `TernaryHuffman::encode()`, and `DictionaryCompressor::encode()` all iterate the complete input before producing output. Not suitable for compressing data streams larger than available memory.

- **Huffman decoding is O(N × K)**: `TernaryHuffman::decode()` tries each code pattern at every position via linear scan of the reverse lookup table. With K = 3 symbols this is fast, but the approach doesn't scale and is fragile — if the bit stream is corrupted, decoding silently stops at the first unrecognizable pattern without reporting an error position.

- **Dictionary decode is O(N × D)**: `DictionaryCompressor::decode()` does a linear scan of the dictionary for each code ≥ 3. With a large dictionary, this becomes slow. The dictionary should use a `HashMap<usize, &DictEntry>` for O(1) lookup.

- **Byte packing doesn't preserve length**: `TernarySequence::from_bytes()` requires the caller to pass the original trit count. Without it, you can't tell whether the last byte represents 1, 2, 3, 4, or 5 trits. The length must be stored separately in the compressed format.

- **RLE compression ratio calculation assumes 2 bytes per run**: `RunLengthEncoder::compression_ratio()` divides `runs × 2` by the original length, which doesn't account for variable-size count fields or the overhead of the Trit enum.

- **Dictionary compressor doesn't support nested patterns**: Patterns are matched greedily left-to-right during encoding. Overlapping or nested patterns (e.g., `[Pos, Pos]` vs `[Pos, Pos, Neg]`) are not handled optimally — the longest match wins, but this can prevent shorter matches that would enable more compression overall.

- **No entropy calculation**: Unlike `ternary-compression-v2`, this crate does not compute Shannon entropy or provide an information-theoretic baseline for comparison.

## Use Cases

1. **Ternary neural network weight storage** — Compress quantized {-1, 0, +1} weight matrices before serialization
2. **Ternary sequence archival** — Store large ternary datasets compactly using byte packing (5 trits/byte baseline)
3. **Sensor data compression** — RLE compress ternary thresholded sensor readings with long stable periods
4. **Simulation output** — Compress ternary cellular automata or agent model outputs that contain repeated patterns

## Ecosystem

Part of the **SuperInstance** ternary computing crate family. This is the earlier version; `ternary-compression-v2` adds LZW, entropy coding, and a `CompressionTracker` for method comparison.

## License

MIT
