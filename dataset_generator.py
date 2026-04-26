import csv
import random

NUM_TXS = 5000
NUM_ACCOUNTS = 200

def make_accounts():
    # Power-law weights: top accounts send disproportionately many txs
    accounts = [f"0x{i:040x}" for i in range(1, NUM_ACCOUNTS + 1)]
    weights = [1.0 / (i ** 1.5) for i in range(1, NUM_ACCOUNTS + 1)]
    total = sum(weights)
    weights = [w / total for w in weights]
    return accounts, weights

def generate_dataset(filename, scenario):
    accounts, weights = make_accounts()
    nonce_counters = [0] * NUM_ACCOUNTS

    with open(filename, mode='w', newline='') as file:
        writer = csv.writer(file)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms', 'from', 'nonce'])

        arrival_time = 1000000

        for i in range(NUM_TXS):
            arrival_time += random.randint(5, 20)

            account_idx = random.choices(range(NUM_ACCOUNTS), weights=weights)[0]
            sender = accounts[account_idx]
            nonce = nonce_counters[account_idx]
            nonce_counters[account_idx] += 1

            if scenario == "small_heavy":
                size = random.randint(100, 300)
                tx_type = "transfer"
            elif scenario == "large_heavy":
                size = random.randint(10000, 40000)
                tx_type = random.choice(["swap", "mint"])
            else:
                if random.random() < 0.80:
                    size = random.randint(100, 300)
                    tx_type = "transfer"
                else:
                    size = random.randint(10000, 40000)
                    tx_type = random.choice(["swap", "mint"])

            writer.writerow([i, size, tx_type, arrival_time, sender, nonce])

print("Generating 5,000 transactions per scenario...")
generate_dataset("small_heavy.csv", "small_heavy")
generate_dataset("large_heavy.csv", "large_heavy")
generate_dataset("mixed.csv", "mixed")
print("Done! 3 CSV files created.")
