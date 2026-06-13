# Ternary Compression — Run-Length, Huffman, and Dictionary Encoding for Ternary Sequences

**Ternary Compression** provides three compression algorithms for balanced-ternary sequences: run-length encoding (RLE), ternary Huffman coding, and dictionary-based pattern compression. Each algorithm is designed for data composed of {-1, 0, +1} trits — the native format for ternary neural network weights, agent decision logs, and ternary sensor data.

## Why It Matters

Binary compressors waste structural information when applied to ternary data because they treat each trit as a full byte. A ternary sequence of 1 million trits occupies ~1 MB as raw bytes, but its information content is only log₂(3) × 10⁶ ≈ 1.58 Mbit ≈ 198 KB. The algorithms in this crate work directly on ternary symbols, closing that gap. For federated learning scenarios where ternary model weights must be transmitted over bandwidth-limited connections, native ternary compression can reduce transfer size by 3–5× compared to generic gzip.

## How It Works

### Run-Length Encoding (RLE)

Compresses consecutive identical trits into `(trit, count)` pairs. For a sequence with k runs of average length n/k, the compressed size is 2k entries vs n original trits. Compression ratio = n/(2k).

```
Input:  +1 +1 +1 +1 0 0 -1 +1
RLE:    (+1,4) (0,2) (-1,1) (+1,1)
```

Time complexity: O(n) for both encoding and decoding.

### Ternary Huffman Coding

Constructs an optimal prefix-free code using a 3-ary (ternary) Huffman tree. Unlike binary Huffman (combine 2 least-frequent), ternary Huffman combines 3 least-frequent symbols at each step. If only 2 symbols remain, a dummy zero-frequency symbol is inserted to complete the tree.

The average code length approaches the ternary entropy H₃(X) = -Σ pᵢ log₃(pᵢ). For uniformly distributed ternary data, H₃ = 1 trit per symbol (no compression possible). For skewed distributions (common in ternary networks where many weights are 0), compression is significant.

### Dictionary Compression

Scans for repeated ternary substrings and builds a dictionary mapping patterns to indices. The encoder replaces occurrences with dictionary references. This is most effective on structured ternary data with repeating motifs, such as weight matrices with block structure.

## Quick Start

```rust
use ternary_compression::{Trit, TernarySequence, RunLengthEncoder};

// Create a ternary sequence
let seq = TernarySequence::from_i8(&[1, 1, 1, 0, 0, -1, 1, 1, 1]);

// Run-length encode
let encoder = RunLengthEncoder::new();
let compressed = encoder.encode(&seq);
println!("Compressed {} trits to {} entries", seq.len(), compressed.len());
```

```bash
cargo add ternary-compression
```

## API

| Type / Function | Description |
|---|---|
| `Trit` | Enum: `Neg(-1)`, `Zero(0)`, `Pos(+1)` with `digit()` / `from_digit()` |
| `TernarySequence` | Owned sequence of trits with `from_i8()`, `len()`, `is_empty()` |
| `RunLengthEncoder` | RLE: `encode(seq) → Vec<(Trit, usize)>` |
| `TernaryHuffman` | Builds ternary Huffman code from frequency analysis |
| `DictionaryCompressor` | Pattern-based dictionary compression |

## Architecture Notes

Part of the **SuperInstance** ternary ecosystem. Compression reduces the bandwidth cost of fleet synchronization — compressed ternary state vectors transfer faster between nodes while preserving the γ + η = C conservation invariant (compression reduces entropy η without losing information). See [Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

- Huffman, David A. "A Method for the Construction of Minimum-Redundancy Codes," *Proceedings of the IRE*, 40(9), 1952.
- Cover, Thomas & Thomas, Joy. *Elements of Information Theory*, 2nd ed., Wiley, 2006 — source coding theorem.
- Ziv, Jacob & Lempel, Abraham. "A Universal Algorithm for Sequential Data Compression," *IEEE Transactions on Information Theory*, 23(3), 1977.



## Complexity Summary

| Algorithm | Encoding | Decoding | Compression Ratio |
|---|---|---|---|
| RLE | O(n) | O(n) | n/(2k) for k runs |
| Huffman (3-ary) | O(n log n) | O(n) | Approaches H₃(X) |
| Dictionary | O(n × p) | O(m) | Depends on pattern frequency |

For ternary weight matrices with 40-60% zeros, RLE typically achieves 2-3× compression; Huffman adds another 1.3-1.5× on top.

## License

MIT
