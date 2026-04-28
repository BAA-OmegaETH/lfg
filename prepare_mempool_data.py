import csv
import os

_HERE = os.path.dirname(os.path.abspath(__file__))
INPUT_FILE = os.path.join(_HERE, "2025-01-01.csv")
NUM_TXS = 5000

# Per-dataset target spans (batch_timeout_ms=60s throughout).
# small_heavy: 10 windows * ~500 txs * 68B   =  34KB/window = 0.26 blobs -> no overflow (correct)
# large_heavy: 10 windows * ~500 txs * 2468B = 1.23MB/window = ~9.6 blobs -> heavy overflow
# mixed:        5 windows * ~1000 txs * 415B =  415KB/window = ~3.2 blobs -> meaningful overflow
SPAN_SMALL_HEAVY_MS = 10 * 60 * 1000  # 10 min
SPAN_LARGE_HEAVY_MS = 10 * 60 * 1000  # 10 min
SPAN_MIXED_MS       =  5 * 60 * 1000  #  5 min — tighter window forces overflow

SELECTOR_MAP = {
    # ERC-20 transfers and approvals
    "0xa9059cbb": "transfer",  # transfer(address,uint256)
    "0x23b872dd": "transfer",  # transferFrom(address,address,uint256)
    "0x095ea7b3": "transfer",  # approve(address,uint256)
    "0xa22cb465": "transfer",  # setApprovalForAll(address,bool)
    "0xd0e30db0": "transfer",  # deposit() — WETH wrap
    "0x2e1a7d4d": "transfer",  # withdraw(uint256) — WETH unwrap
    # DEX swaps
    "0x3593564c": "swap",
    "0x38ed1739": "swap",
    "0x8803dbee": "swap",
    "0x7ff36ab5": "swap",
    "0x18cbafe5": "swap",
    "0x5c11d795": "swap",
    "0x12aa3caf": "swap",
    "0x9871efa4": "swap",
    "0x2213bc0b": "swap",
    "0x5f575529": "swap",      # swap(string,address,uint256,bytes)
    "0x0d5f0e3b": "swap",      # uniswapV3SwapTo(uint256,uint256,uint256,uint256[])
    "0x07ed2379": "swap",      # swap(address,(address,...),bytes)
    # Mints
    "0x40c10f19": "mint",
    "0x1249c58b": "mint",
    "0xa0712d68": "mint",
}

def infer_tx_type(selector):
    if not selector:
        return "other"
    return SELECTOR_MAP.get(selector, "other")

def generate(output_file, size_min=1, size_max=None, target_span_ms=SPAN_SMALL_HEAVY_MS):
    # Pass 1: collect matching rows
    rows = []
    scanned = 0
    with open(INPUT_FILE, newline='') as infile:
        for row in csv.DictReader(infile):
            scanned += 1
            data_size = int(row['data_size'])
            if data_size < size_min:
                continue
            if size_max is not None and data_size > size_max:
                continue
            rows.append(row)
            if len(rows) >= NUM_TXS:
                break

    # Pass 2: normalize timestamps to TARGET_SPAN_MS while preserving relative pattern
    ts_first = int(rows[0]['timestamp_ms'])
    ts_last  = int(rows[-1]['timestamp_ms'])
    actual_span = ts_last - ts_first

    with open(output_file, 'w', newline='') as outfile:
        writer = csv.writer(outfile)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms', 'from', 'nonce'])
        for i, row in enumerate(rows):
            raw_ts = int(row['timestamp_ms'])
            if actual_span > 0:
                normalized_ts = ts_first + int((raw_ts - ts_first) * target_span_ms / actual_span)
            else:
                normalized_ts = ts_first + i
            writer.writerow([
                i,
                int(row['data_size']),
                infer_tx_type(row['data_4bytes']),
                normalized_ts,
                row['from'],
                int(row['nonce']),
            ])

    print(f"  scanned {scanned} rows, wrote {len(rows)} txs, span {actual_span/1000:.0f}s -> {target_span_ms//1000}s")

def generate_real(output_file):
    """Write all qualifying rows with real (un-normalized) timestamps."""
    rows = []
    scanned = 0
    with open(INPUT_FILE, newline='') as infile:
        for row in csv.DictReader(infile):
            scanned += 1
            data_size = int(row['data_size'])
            if data_size < 1:
                continue
            rows.append(row)

    with open(output_file, 'w', newline='') as outfile:
        writer = csv.writer(outfile)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms', 'from', 'nonce'])
        for i, row in enumerate(rows):
            writer.writerow([
                i,
                int(row['data_size']),
                infer_tx_type(row['data_4bytes']),
                int(row['timestamp_ms']),
                row['from'],
                int(row['nonce']),
            ])

    ts_first = int(rows[0]['timestamp_ms'])
    ts_last  = int(rows[-1]['timestamp_ms'])
    span_hours = (ts_last - ts_first) / 3_600_000
    print(f"  scanned {scanned} rows, wrote {len(rows)} txs, span {span_hours:.2f}h (real timestamps)")

def out(name):
    return os.path.join(_HERE, name)

print("Generating datasets from 2025-01-01.csv...")
print("small_heavy  (1-300 bytes):")
generate(out("small_heavy.csv"),  size_min=1,    size_max=300,  target_span_ms=SPAN_SMALL_HEAVY_MS)
print("large_heavy  (>2000 bytes):")
generate(out("large_heavy.csv"),  size_min=2001, size_max=None, target_span_ms=SPAN_LARGE_HEAVY_MS)
print("mixed        (all sizes, 5-min window):")
generate(out("mixed.csv"),        size_min=1,    size_max=None, target_span_ms=SPAN_MIXED_MS)
print("real_full    (all qualifying rows, real timestamps):")
generate_real(out("real_full.csv"))
print("Done.")

def generate_hourly(output_file, hour_utc, size_min=1, size_max=None):
    """Extract txs from a specific UTC hour with real timestamps."""
    rows = []
    with open(INPUT_FILE, newline='') as infile:
        for row in csv.DictReader(infile):
            ts_ms = int(row['timestamp_ms'])
            row_hour = ts_ms // 3_600_000 % 24
            if row_hour != hour_utc:
                continue
            data_size = int(row['data_size'])
            if data_size < size_min:
                continue
            if size_max is not None and data_size > size_max:
                continue
            rows.append(row)

    with open(output_file, 'w', newline='') as outfile:
        writer = csv.writer(outfile)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms', 'from', 'nonce'])
        for i, row in enumerate(rows):
            writer.writerow([
                i,
                int(row['data_size']),
                infer_tx_type(row['data_4bytes']),
                int(row['timestamp_ms']),
                row['from'],
                int(row['nonce']),
            ])

    print(f"  hour=UTC{hour_utc:02d} wrote {len(rows)} txs (real timestamps)")

print("\nGenerating hourly traffic intensity datasets (all sizes, real timestamps)...")
print("high traffic   (UTC 00:00):")
generate_hourly(out("traffic_high.csv"),   hour_utc=0,  size_min=1)
print("medium traffic (UTC 12:00):")
generate_hourly(out("traffic_medium.csv"), hour_utc=12, size_min=1)
print("low traffic    (UTC 23:00):")
generate_hourly(out("traffic_low.csv"),    hour_utc=23, size_min=1)
print("Hourly datasets done.")

# large_heavy span sweep: same 5000 large txs, different time compression
# Controls txs-per-window → controls overflow intensity → shows DES advantage scaling
SPAN_LH_LOW_MS    = 30 * 60 * 1000   # 30 min → ~167 txs/window → ~3 blobs/window (mild overflow)
SPAN_LH_MEDIUM_MS = 10 * 60 * 1000   # 10 min → ~500 txs/window → ~9 blobs/window (same as large_heavy)
SPAN_LH_HIGH_MS   =  3 * 60 * 1000   #  3 min → ~1667 txs/window → ~32 blobs/window (heavy overflow)

print("\nGenerating large_heavy span sweep (5000 large txs >2000B, variable overflow)...")
print("lh_low    (30 min span, mild overflow):")
generate(out("lh_low.csv"),    size_min=2001, size_max=None, target_span_ms=SPAN_LH_LOW_MS)
print("lh_medium (10 min span, same as large_heavy):")
generate(out("lh_medium.csv"), size_min=2001, size_max=None, target_span_ms=SPAN_LH_MEDIUM_MS)
print("lh_high   (3 min span, heavy overflow):")
generate(out("lh_high.csv"),   size_min=2001, size_max=None, target_span_ms=SPAN_LH_HIGH_MS)
print("Span sweep done.")
