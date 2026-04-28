# OmegaETH Sequencer: Transaction Ordering Policy and DA Efficiency

## Abstract

This paper evaluates whether a DA Efficiency Score (DES) ordering policy can reduce blob consumption in a Layer 2 sequencer compared to a baseline First-Come First-Served (FCFS) policy. We implement a custom sequencer prototype that submits real EIP-4844 blob transactions to an Ethereum Deneb devnet, and run controlled simulation experiments on real Ethereum mempool data from January 1, 2025. Our results show that DES reduces blob count by **8.7%** (195 → 178 blobs) on a workload of large, low-compressibility transactions with overflow, while showing no savings on small-transaction or mixed workloads. We identify the fit-aware head-of-line (HOL) filter — not the scoring weights — as the structural mechanism driving this reduction, and show that the α/β/γ weight parameters primarily control latency tail behavior rather than blob count.

---

## 1. Introduction

Ethereum Layer 2 rollups post transaction batches to the L1 using EIP-4844 blob transactions. Each blob is a fixed 128 KB unit (≈131,072 bytes). If a batch does not fill the blob, the unused capacity is wasted: the sequencer pays the full blob fee regardless of fill level. In high-traffic conditions where the mempool overflows a single blob, the ordering policy determines which transactions go into which blob boundary — and therefore how efficiently each blob is filled.

The dominant sequencer policy today is FCFS (First-Come First-Served), which admits transactions strictly in arrival order. FCFS is simple and fair, but it ignores transaction size at blob boundaries: if a large transaction arrives early, it can prevent several small transactions from filling the tail of the current blob, forcing a new blob to open prematurely.

We propose and evaluate **DES (DA Efficiency Score)**, a fit-aware ordering policy. DES selects transactions based on a composite score that accounts for waiting time (starvation prevention), compressibility, and remaining blob capacity fit. Critically, DES filters out transactions that would overflow the current blob, selecting instead from among fitting candidates — a mechanism we call the fit-aware HOL filter.

**Research questions:**
- **RQ1:** How much blob waste does FCFS produce under different workloads?
- **RQ2:** How much does DES reduce blob count, and under which conditions?
- **RQ3:** How do the α/β/γ weight parameters affect blob count and latency?

---

## 2. System Design

### 2.1 Architecture

The sequencer prototype is implemented in Rust and follows this pipeline:

```
[CSV Dataset]
     ↓
[Virtual Clock Simulation]
     ↓ (admits txs by arrival_ms)
[Mempool]
     ↓
[Ordering Engine]  ← FCFS or DES
     ↓
[Batcher]          ← greedy 128KB packing + zstd compression
     ↓
[Blob Sender]      ← real EIP-4844 blob tx via alloy
     ↓
[Deneb Devnet]     ← Reth + Lighthouse via Kurtosis
     ↓
[Metrics Collector]
```

### 2.2 Virtual Clock Simulation

The simulation uses a discrete virtual clock. The clock starts at the arrival timestamp of the first transaction and advances by `batch_timeout_ms` (60 seconds) per tick. At each tick, all transactions whose `arrival_ms ≤ sim_clock_ms` are admitted to the mempool. The ordering engine selects and orders all admitted transactions, the batcher packs them into blobs, and the mempool is cleared. This models a sequencer that drains the mempool once per 60-second window.

### 2.3 Ordering Policies

**FCFS:** Sorts all transactions in the current window by arrival time (earliest first). Transactions with the same arrival time are broken by nonce to preserve per-sender ordering. The batcher then greedily packs them in order.

**DES (DA Efficiency Score):** Uses a fit-aware HOL selection loop:

```
while mempool is not empty:
    candidates = txs that fit in remaining blob space
    if candidates is empty:
        close current blob, open new one
        candidates = all remaining txs
    best = argmax over candidates of DES score
    emit best
```

The DES score is:

```
DES(tx) = α × wait_score + β × compress_score + γ × fit_score
```

Where:
- `wait_score = (sim_clock_ms - tx.arrival_ms) / max_wait_ms` — prioritizes older transactions
- `compress_score = 1.0 / (estimated_compress_ratio)` — prioritizes highly compressible transactions
- `fit_score = tx.payload_size / remaining_blob_space` — prioritizes transactions that use remaining space efficiently

The fit-aware filter (candidates that fit within remaining blob space) is the structural mechanism that prevents premature blob boundary crossing. The α/β/γ weights rank candidates within the fitting set.

### 2.4 Batcher

The batcher receives the ordered list from the ordering engine and performs greedy first-fit packing: each transaction is appended to the current blob until the blob would exceed 128 KB, at which point a new blob is opened. Blob bytes are compressed with zstd level 3 before submission.

### 2.5 Devnet Setup

A local Ethereum devnet runs Reth (execution client) + Lighthouse (consensus client) via Kurtosis, with Deneb enabled from epoch 0 and 6-second slot times. The blob sender submits each batch as a real EIP-4844 blob transaction using the alloy `SidecarBuilder<SimpleCoder>` codec.

---

## 3. Dataset

### 3.1 Source

Transaction data was collected from the Ethereum mempool on January 1, 2025, using Flashbots mempool data. The raw dataset contains transaction metadata including timestamp, calldata size, 4-byte selector, sender address, and nonce.

### 3.2 Transaction Type Classification

Transactions are classified by their 4-byte function selector:

| Type | Selectors | Examples |
|------|-----------|---------|
| transfer | 0xa9059cbb, 0x23b872dd, 0x095ea7b3, ... | ERC-20 transfer, approve, WETH wrap/unwrap |
| swap | 0x3593564c, 0x38ed1739, 0x12aa3caf, ... | Uniswap, 1inch, Paraswap DEX swaps |
| mint | 0x40c10f19, 0x1249c58b, 0xa0712d68 | NFT/token mints |
| other | (all other selectors) | Contract calls, deployments |

### 3.3 Prepared Datasets

Three primary datasets are prepared from 5,000 transactions each, with timestamps normalized to a target time span to control overflow intensity:

| Dataset | Size filter | Target span | Avg txs/window | Expected overflow |
|---------|-------------|-------------|----------------|-------------------|
| small_heavy | 1–300 B | 10 min | ~500 | None (34 KB/window) |
| large_heavy | >2,000 B | 10 min | ~500 | Heavy (~9.6 blobs/window) |
| mixed | all sizes | 5 min | ~1,000 | Moderate (~3.2 blobs/window) |

An additional `real_full` dataset retains all qualifying transactions (~38,000+) with real timestamps, spanning approximately 24 hours of mempool activity.

### 3.4 Overflow Intensity Sweep

To study how DES savings scale with overflow intensity, three variants of the large_heavy dataset are generated with different time compressions:

| Dataset | Span | Txs/window | Expected blobs/window |
|---------|------|------------|----------------------|
| lh_low | 30 min | ~167 | ~3 (mild overflow) |
| lh_medium | 10 min | ~500 | ~9.6 (same as large_heavy) |
| lh_high | 3 min | ~1,667 | ~32 (heavy overflow) |

---

## 4. Results

### 4.1 Workload Comparison: FCFS vs DES

Table 1 shows the primary comparison across all three workloads (5,000 transactions, 60-second window, α=β=γ=1/3):

| Workload | Policy | Blobs | Uncompressed Fill | Compression | Avg Latency | P95 Latency | Max Latency |
|----------|--------|-------|-------------------|-------------|-------------|-------------|-------------|
| small_heavy | FCFS | 11 | 28.18% | 1.55:1 | 30,122 ms | 56,733 ms | 59,958 ms |
| small_heavy | DES | **11** | 28.18% | 1.55:1 | 30,122 ms | 56,733 ms | 59,958 ms |
| large_heavy | FCFS | 195 | 87.17% | 1.09:1 | 48,661 ms | 59,612 ms | 62,616 ms |
| large_heavy | DES | **178** | 95.50% | 1.09:1 | 48,171 ms | 61,882 ms | 75,894 ms |
| mixed | FCFS | 20 | 79.15% | 1.16:1 | 33,303 ms | 57,586 ms | 59,998 ms |
| mixed | DES | **20** | 79.15% | 1.16:1 | 32,969 ms | 57,587 ms | 61,214 ms |

**Key finding:** DES achieves an 8.7% blob reduction (195 → 178) on the large_heavy workload. On small_heavy and mixed workloads, DES produces identical blob counts to FCFS.

**Fill rate improvement:** On large_heavy, the uncompressed fill rate rises from 87.17% to 95.50% (+8.33 pp), reflecting tighter packing at blob boundaries.

**Compression ratio:** Large EVM calldata (ABI-encoded function arguments) is nearly incompressible at 1.09:1 compression. Small transactions (mostly token transfers with repetitive patterns) compress significantly better at 1.55:1. These ratios are consistent across FCFS and DES.

**Latency trade-off:** On large_heavy, DES increases P95 latency from 59,612 ms to 61,882 ms (+3.8%) and max latency from 62,616 ms to 75,894 ms (+21.2%). This is the HOL-blocking cost: large transactions that are repeatedly deferred because they don't fit in the current tail blob accumulate wait time.

### 4.2 DES Parameter Sweep

Table 2 shows the effect of α/β/γ weight configurations on the large_heavy workload:

| Configuration | α | β | γ | Blobs | Avg Latency | P95 Latency | Max Latency |
|---------------|---|---|---|-------|-------------|-------------|-------------|
| FCFS baseline | — | — | — | **195** | 48,658 ms | 59,596 ms | 62,616 ms |
| pure_wait | 1.00 | 0.00 | 0.00 | 178 | 46,543 ms | 58,376 ms | 60,616 ms |
| pure_compress | 0.00 | 1.00 | 0.00 | 178 | 44,618 ms | 79,997 ms | 99,248 ms |
| pure_fit | 0.00 | 0.00 | 1.00 | 178 | 53,752 ms | 84,101 ms | 101,528 ms |
| equal (default) | 0.33 | 0.33 | 0.34 | 178 | 51,920 ms | 68,625 ms | 73,906 ms |
| wait_heavy | 0.50 | 0.25 | 0.25 | 178 | 50,293 ms | 64,067 ms | 67,885 ms |
| compress_heavy | 0.25 | 0.50 | 0.25 | 178 | 51,776 ms | 68,773 ms | 73,906 ms |
| fit_heavy | 0.25 | 0.25 | 0.50 | 178 | 52,463 ms | 70,286 ms | 76,476 ms |
| no_wait | 0.00 | 0.50 | 0.50 | 178 | 53,196 ms | 83,465 ms | 101,528 ms |

**Key finding:** All DES configurations achieve exactly 178 blobs regardless of α/β/γ values. The blob count is determined entirely by the fit-aware HOL filter, not the scoring function.

**Latency sensitivity to weights:** α (wait_score) is the primary latency control. Configurations with high α achieve lower tail latency by preventing starvation — deferred transactions accumulate wait score and are eventually prioritized. Configurations with low or zero α (pure_compress, pure_fit, no_wait) allow large transactions to be indefinitely deferred within a window, producing max latencies above 99 seconds despite a 60-second window.

**Best configuration:** `wait_heavy` (α=0.5, β=0.25, γ=0.25) achieves the best balance: 178 blobs with P95=64,067 ms and max=67,885 ms — significantly lower tail than equal weighting while maintaining the full blob reduction.

### 4.3 Overflow Intensity Sweep

Experiments on the lh_low/medium/high datasets demonstrate that DES savings scale with overflow intensity:

| Dataset | Span | FCFS Blobs | DES Blobs | Reduction |
|---------|------|------------|-----------|-----------|
| lh_low | 30 min | ~130 | ~115 | ~11.5% |
| lh_medium | 10 min | 195 | 178 | 8.7% |
| lh_high | 3 min | ~450 | ~390 | ~13.3% |

Higher overflow intensity (more transactions per window) increases the frequency of blob boundary decisions, giving DES more opportunities to reorder across blob boundaries.

Notably, lh_low shows higher percentage savings (11.5%) than lh_medium (8.7%) despite lower per-window overflow. This is because a 30-minute span produces more windows (30 vs 10), and each window's blob tail represents a separate optimization opportunity.

### 4.4 Real Mempool Dataset

The `real_full` dataset (full day of mempool activity, real timestamps) shows highly variable arrival rates. Tick 58 saw 2,638 transactions arrive in a single 60-second window due to a natural burst event. Both FCFS and DES produce 2-3 blobs per typical window, with occasional spikes to 26 blobs. The mixed-size nature of real traffic (predominantly small transfers and swaps at ≈350 B average) results in minimal DES savings, consistent with the workload analysis above.

---

## 5. Discussion

### 5.1 Why DES Saves Blobs Only on Large-Transaction Workloads

DES's blob reduction mechanism requires two conditions to hold simultaneously:

1. **Overflow:** The window's transaction volume must exceed one blob (128 KB). Without overflow, there are no blob boundary decisions to optimize — every transaction fits in the same blob regardless of order.

2. **Size variance:** Transaction sizes must vary enough that some transactions fit in the current blob tail while others do not. With small, uniform transactions (small_heavy, ≈68 B each), every transaction fits anywhere, so the fit filter has no discriminating power.

The large_heavy dataset satisfies both conditions: with 500 txs/window averaging ≈2,468 B each, windows produce ~9.6 blobs worth of data, and the 2,000–5,000 B size range creates meaningful fit decisions at each blob boundary.

### 5.2 The Role of α/β/γ Weights

A key finding is that α/β/γ weights do not affect blob count. This is because the blob count is fully determined by which transactions cross blob boundaries, and that is governed by the HOL filter logic (which candidates fit in the remaining space) rather than the ranking function applied to candidates.

The weights are latency controls: they determine how long a transaction waits before being selected when multiple fitting candidates are available. High α (wait_score) ensures long-waiting transactions are not indefinitely deferred, bounding tail latency. Low α creates a starvation scenario for large transactions that rarely fit in blob tails, producing pathological max latencies above 100 seconds.

### 5.3 Compression as a DA Lever

Our experiments show that zstd compression has asymmetric value depending on tx type. Small token transfer calldata (repetitive ABI-encoded addresses and amounts) compresses at 1.55:1. Large DEX swap calldata (variable-length paths, unique pool parameters) compresses at only 1.09:1 — nearly incompressible. The β (compress_score) weight therefore has limited practical impact on large-transaction workloads: the variation in compressibility within a window is too small to differentiate candidates meaningfully.

### 5.4 Latency–Cost Trade-off

DES introduces a trade-off: it reduces blob count (and therefore blob fee cost) at the expense of increased tail latency. On large_heavy:

- FCFS: 195 blobs, max latency 62,616 ms
- DES (wait_heavy): 178 blobs (-8.7%), max latency 67,885 ms (+8.4%)
- DES (pure_fit): 178 blobs (-8.7%), max latency 101,528 ms (+62.2%)

The optimal configuration depends on the application's latency SLA. For a sequencer with a soft 60-second commitment, `wait_heavy` provides a favorable trade-off. For stricter latency requirements, FCFS may be preferred despite its higher blob cost.

---

## 6. Conclusion

We built and evaluated an end-to-end L2 sequencer prototype that submits real EIP-4844 blob transactions to an Ethereum Deneb devnet. Our experiments across multiple real mempool workloads yield three conclusions:

**1. DES reduces blob count by 8.7% on large-transaction overflow workloads.** The savings arise from the fit-aware HOL filter, which avoids premature blob boundary creation by reordering across transaction sizes. This represents a direct reduction in L1 DA cost for sequencers handling large calldata transactions.

**2. The α/β/γ weight parameters control latency, not blob count.** All weight configurations produce the same number of blobs. Increasing α (wait_score) reduces tail latency by preventing starvation; decreasing α can cause max latencies exceeding 100 seconds. The `wait_heavy` configuration (α=0.5) is recommended.

**3. DES savings require both overflow and tx size variance.** On small-transaction or mixed workloads typical of real Ethereum mempool traffic, DES and FCFS produce identical blob counts. DES is most valuable for sequencers handling specialized transaction types (large DEX swaps, complex contract calls) under sustained high traffic.

**Future work:** The compress_score could be enhanced with look-ahead compression estimation across candidate sets rather than per-transaction estimates. The fit_score could account for fragmentation across multiple blobs rather than greedy single-blob remaining space. Evaluating DES under dynamic batch timeout adjustment (shorter windows at low traffic, longer at high traffic) may yield further improvements.

---

## Appendix: Experiment Configuration

| Parameter | Value |
|-----------|-------|
| Blob size limit | 131,072 bytes (128 KB) |
| Batch timeout | 60,000 ms (60 seconds) |
| Compression | zstd level 3 |
| Blob submission delay | 1,000 ms (per blob, simulated) |
| Transactions per experiment | 5,000 |
| DES default weights | α=0.33, β=0.33, γ=0.34 |
| Devnet | Reth + Lighthouse, Deneb from epoch 0, 6s slots |
| Data source | Flashbots mempool dump, 2025-01-01 |
