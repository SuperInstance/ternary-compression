# ternary-compression

A multi-algorithm compression library for balanced ternary sequences {-1, 0, +1}. Implements **run-length encoding**, **Huffman coding** adapted for ternary alphabets, **dictionary-based compression** with automatic pattern mining, and **5-trits-per-byte packing**. Includes a `TernarySequence` core type with byte-level serialization.

## Why It Matters

Ternary data arises in quantized neural networks, agent decision logs, and balanced-ternary computing. Different data distributions call for different compression strategies:

- **Long runs** → RLE (pruned network layers, zero-padded tensors)
- **Skewed distributions** → Huffman (e.g., 80% zeros, 10% +1, 10% −1)
- **Repeated patterns** → Dictionary (structured weight blocks, periodic signals)
- **Uniform random** → Bit packing (2 bits/trit, no further compression possible)

This crate provides all four strategies with a unified `TernarySequence` type, so you can benchmark each algorithm on your data and choose the best.

Within the **γ + η = C** framework:

| Symbol | Domain |
|--------|--------|
| γ | `TernarySequence` — ordered trit data |
| η | Algorithm selection: RLE, Huffman, dictionary, packing |
| C | Shannon entropy bound: $H(X) \leq \log_2 3 \approx 1.585$ bits/trit |

## How It Works

### Ternary Sequence Packing: 5 Trits per Byte

Since $3^5 = 243 < 256 = 2^8$, we can pack exactly 5 trits into a single byte with 13 unused bit patterns:

$$\text{byte} = \sum_{i=0}^{4} d_i \cdot 3^i$$

where $d_i \in \{0, 1, 2\}$ maps from `{Neg, Zero, Pos}`.

This achieves $\frac{8}{5} = 1.6$ bits/trit — within 0.015 bits of the Shannon limit ($\log_2 3 = 1.585$).

| Method | Bits/Trit | Efficiency |
|--------|-----------|------------|
| Naive (1 byte/trit) | 8.000 | 19.8% |
| 2-bit packing | 2.000 | 79.2% |
| **5-trits/byte** | **1.600** | **99.1%** |
| Shannon limit | 1.585 | 100% |

**Complexity**: O(n) for both `to_bytes` and `from_bytes`.

### Run-Length Encoding

Standard RLE on ternary sequences: consecutive identical trits collapse to `(Trit, count)` pairs.

$$\text{RLE ratio} = \frac{2k}{n}$$

where $k$ = number of runs. For alternating data (worst case), $k = n$ and RLE doubles the size. For constant data (best case), $k = 1$ and ratio approaches 0.

**Complexity**: O(n) encode and decode.

### Huffman Coding for Ternary Alphabets

The Huffman coder builds an optimal **binary** prefix code for the three trit values based on their frequencies. Since there are only 3 symbols, the code tree is small:

- Most frequent symbol: 1 bit (`0`)
- Second most frequent: 2 bits (`10`)
- Least frequent: 2 bits (`11`)

**Expected code length**:

$$\bar{L} = \sum_{i} p_i \cdot l_i$$

For uniform distribution ($p = 1/3$ each): $\bar{L} = \frac{1}{3}(1 + 2 + 2) = \frac{5}{3} \approx 1.667$ bits/trit.

For skewed distributions ($p_0 = 0.9, p_{\pm1} = 0.05$ each): $\bar{L} = 0.9 \times 1 + 0.05 \times 2 + 0.05 \times 2 = 1.1$ bits/trit.

**Complexity**: O(n + k log k) where k = 3 (constant), so effectively O(n).

Decoding uses a **prefix-matching scan**: at each bit position, compare against all codewords. With only 3 codewords (max length 2), this is O(1) per decoded symbol.

### Dictionary Compression with Pattern Mining

The dictionary compressor automatically discovers frequent patterns:

1. **Mine**: Scan all subsequences of length `min_len..=max_len`.
2. **Filter**: Keep patterns appearing more than once.
3. **Rank**: Sort by $\text{freq} \times \text{length}$ (value = total symbols covered).
4. **Encode**: Greedy longest-match at each position.

$$\text{gain}(p) = \text{freq}(p) \times (|p| - \text{code\_size})$$

Dictionary entries are assigned codes starting at 3 (0, 1, 2 reserved for single trits).

**Complexity**: Mining is O(n · L) where L = max pattern length. Encoding is O(n · d) where d = dictionary size. Practical dictionaries are < 1000 entries, so encoding is near-linear.

### Compression Statistics

`CompressionStats` computes:

- **Ratio**: $\frac{\text{compressed}}{\text{original}}$
- **Space saving**: $1 - \text{ratio}$

## Quick Start

```rust
use ternary_compression::{
    TernarySequence, Trit,
    RunLengthEncoder, TernaryHuffman, DictionaryCompressor, CompressionStats,
};

// Create from i8 values
let seq = TernarySequence::from_i8(&[1, 1, 1, -1, -1, 0, 0, 0, 0]);

// RLE
let runs = RunLengthEncoder::encode(&seq);
assert_eq!(runs, vec![(Trit::Pos, 3), (Trit::Neg, 2), (Trit::Zero, 4)]);
let decoded = RunLengthEncoder::decode(&runs);
assert_eq!(seq.trits(), decoded.trits());

// Huffman
let huffman = TernaryHuffman::build(&seq);
let encoded = huffman.encode(&seq);
let decoded = huffman.decode(&encoded);
assert_eq!(seq.trits(), decoded.trits());

// Dictionary
let data = TernarySequence::from_i8(&[1, 1, -1, 1, 1, -1, 0, 0, 0, 1, 1, -1]);
let mut compressor = DictionaryCompressor::new(2, 4);
compressor.build_dict(&data);
let encoded = compressor.encode(&data);
let decoded = compressor.decode(&encoded);
assert_eq!(data.trits(), decoded.trits());

// Byte packing (5 trits/byte)
let seq2 = TernarySequence::from_i8(&[-1, 0, 1, -1, 0, 1, -1, 0, 1, 0]);
let bytes = seq2.to_bytes();
let recovered = TernarySequence::from_bytes(&bytes, 10);
assert_eq!(seq2.trits(), recovered.trits());

// Stats
let stats = CompressionStats::new(1000, 400);
assert!((stats.ratio - 0.4).abs() < 0.01);
assert!((stats.space_saving - 0.6).abs() < 0.01);
```

## API

### `Trit` and `TernarySequence`

| Method | Description |
|--------|-------------|
| `Trit::to_i8()` / `from_i8()` | Convert between Trit and i8 |
| `Trit::digit()` / `from_digit()` | Map to {0, 1, 2} for packing |
| `TernarySequence::new(trits)` | Create from Vec<Trit> |
| `TernarySequence::from_i8(&[i8])` | Create from raw values |
| `to_bytes()` / `from_bytes()` | 5-trits-per-byte serialization |
| `len()`, `is_empty()`, `get(i)` | Accessors |

### `RunLengthEncoder`

| Method | Description |
|--------|-------------|
| `encode(&TernarySequence)` | → `Vec<(Trit, usize)>` |
| `decode(&[(Trit, usize)])` | → `TernarySequence` |
| `compression_ratio(&TernarySequence)` | → `f64` |

### `TernaryHuffman`

| Method | Description |
|--------|-------------|
| `build(&TernarySequence)` | Construct frequency-optimal code |
| `encode(&TernarySequence)` | → `Vec<u8>` (bit sequence) |
| `decode(&[u8])` | → `TernarySequence` |
| `codes()` | → `&HashMap<Trit, Vec<u8>>` |
| `avg_bits_per_trit()` | → `f64` |

### `DictionaryCompressor`

| Method | Description |
|--------|-------------|
| `new(min_len, max_len)` | Configure pattern length range |
| `build_dict(&TernarySequence)` | Mine patterns and build dictionary |
| `encode(&TernarySequence)` | → `Vec<usize>` (codes) |
| `decode(&[usize])` | → `TernarySequence` |
| `dict()`, `dict_size()` | Dictionary access |

### `CompressionStats`

Fields: `original_trits`, `compressed_size`, `ratio`, `space_saving`.

## Architecture Notes

The `TernarySequence` type is the **canonical input/output** for all algorithms. By keeping it separate from the compression algorithms, the crate allows easy composition: e.g., dictionary-compress first, then Huffman-encode the dictionary codes.

The Huffman implementation deliberately uses `Vec<u8>` for bit storage (one byte per bit) rather than true bit-packing. This trades 8× memory overhead for simplicity and debuggability — the encoded stream is human-inspectable. For production deployment, a bit-level writer would reduce memory by 8×.

The dictionary compressor's mining phase has O(n · L) complexity, which can be expensive for large sequences with long max patterns. The `min_pattern_len` and `max_pattern_len` parameters let callers bound this: e.g., `new(2, 4)` mines only 2-, 3-, and 4-grams.

## References

- **Huffman, D. A.** (1952). "A Method for the Construction of Minimum-Redundancy Codes." *Proceedings of the IRE*, 40(9), 1098–1101. — Original Huffman coding.
- **Ziv, J., & Lempel, A.** (1977). "A Universal Algorithm for Sequential Data Compression." *IEEE Transactions on Information Theory*, 23(3), 337–343. — LZ77 dictionary compression.
- **Cover, T. M., & Thomas, J. A.** (2006). *Elements of Information Theory* (2nd ed.). — Shannon entropy, optimal coding bounds.
- **Knuth, D. E.** (1997). *The Art of Computer Programming, Vol. 2: Seminumerical Algorithms* (3rd ed.), §4.6 — Mixed-radix number systems.
- **Rissanen, J.** (1976). "Generalized Kraft Inequality and Arithmetic Coding." *IBM Journal of Research and Development*, 20(3), 198–203. — Arithmetic coding as alternative to Huffman.

## License

MIT
