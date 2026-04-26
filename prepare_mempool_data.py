import csv
import os

_HERE = os.path.dirname(os.path.abspath(__file__))
INPUT_FILE = os.path.join(_HERE, "2025-01-01.csv")
NUM_TXS = 5000

# Normalize all timestamps to span exactly this duration.
# With batch_timeout_ms=60s and 10 windows, each window holds ~500 txs on average.
# large_heavy: 500 txs * 2468 bytes = 1.23MB per window = ~9.6 blobs -> real overflow
# mixed:       500 txs *  415 bytes = 207KB per window  = ~1.6 blobs -> slight overflow
# small_heavy: 500 txs *   68 bytes =  34KB per window  = 0.26 blobs -> no overflow (expected)
TARGET_SPAN_MS = 10 * 60 * 1000  # 10 minutes

SELECTOR_MAP = {
    "0xa9059cbb": "transfer",
    "0x23b872dd": "transfer",
    "0x095ea7b3": "transfer",
    "0x3593564c": "swap",
    "0x38ed1739": "swap",
    "0x8803dbee": "swap",
    "0x7ff36ab5": "swap",
    "0x18cbafe5": "swap",
    "0x5c11d795": "swap",
    "0x12aa3caf": "swap",
    "0x9871efa4": "swap",
    "0x2213bc0b": "swap",
    "0x40c10f19": "mint",
    "0x1249c58b": "mint",
    "0xa0712d68": "mint",
}

def infer_tx_type(selector):
    if not selector:
        return "other"
    return SELECTOR_MAP.get(selector, "other")

def generate(output_file, size_min=1, size_max=None):
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
                normalized_ts = ts_first + int((raw_ts - ts_first) * TARGET_SPAN_MS / actual_span)
            else:
                normalized_ts = ts_first + i  # fallback: sequential
            writer.writerow([
                i,
                int(row['data_size']),
                infer_tx_type(row['data_4bytes']),
                normalized_ts,
                row['from'],
                int(row['nonce']),
            ])

    print(f"  scanned {scanned} rows, wrote {len(rows)} txs, span {actual_span/1000:.0f}s -> {TARGET_SPAN_MS//1000}s")

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
generate(out("small_heavy.csv"),  size_min=1,    size_max=300)
print("large_heavy  (>2000 bytes):")
generate(out("large_heavy.csv"),  size_min=2001, size_max=None)
print("mixed        (all sizes with calldata):")
generate(out("mixed.csv"),        size_min=1,    size_max=None)
print("real_full    (all qualifying rows, real timestamps):")
generate_real(out("real_full.csv"))
print("Done.")
