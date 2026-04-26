import csv

INPUT_FILE = "2025-01-01.csv"
NUM_TXS = 5000

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
    written = 0
    scanned = 0

    with open(INPUT_FILE, newline='') as infile, \
         open(output_file, 'w', newline='') as outfile:

        reader = csv.DictReader(infile)
        writer = csv.writer(outfile)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms', 'from', 'nonce'])

        for row in reader:
            scanned += 1
            data_size = int(row['data_size'])

            if data_size < size_min:
                continue
            if size_max is not None and data_size > size_max:
                continue

            writer.writerow([
                written,
                data_size,
                infer_tx_type(row['data_4bytes']),
                int(row['timestamp_ms']),
                row['from'],
                int(row['nonce']),
            ])
            written += 1

            if written >= NUM_TXS:
                break

    print(f"  scanned {scanned} rows → wrote {written} txs to {output_file}")

print("Generating datasets from 2025-01-01.csv...")
print("small_heavy  (1-300 bytes):")
generate("small_heavy.csv",  size_min=1,    size_max=300)
print("large_heavy  (>2000 bytes):")
generate("large_heavy.csv",  size_min=2001, size_max=None)
print("mixed        (all sizes with calldata):")
generate("mixed.csv",        size_min=1,    size_max=None)
print("Done.")
