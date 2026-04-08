import csv
import random

NUM_TXS = 5000

def generate_dataset(filename, scenario):
    with open(filename, mode='w', newline='') as file:
        writer = csv.writer(file)
        writer.writerow(['tx_id', 'payload_size', 'tx_type', 'arrival_ms'])

        arrival_time = 1000000

        for i in range(NUM_TXS):
            # Simulate real-world arrival intervals (5-20ms apart)
            arrival_time += random.randint(5, 20)

            if scenario == "small_heavy":
                # Mostly simple transfers (100 - 300 bytes)
                size = random.randint(100, 300)
                tx_type = "transfer"
            elif scenario == "large_heavy":
                # Mostly complex contract interactions (10KB - 40KB)
                size = random.randint(10000, 40000)
                tx_type = random.choice(["swap", "mint"])
            else: 
                # Mixed: 80% small transfers, 20% large contracts
                if random.random() < 0.80:
                    size = random.randint(100, 300)
                    tx_type = "transfer"
                else:
                    size = random.randint(10000, 40000)
                    tx_type = random.choice(["swap", "mint"])

            writer.writerow([i, size, tx_type, arrival_time])

print("Generating 5,000 transactions per scenario...")
generate_dataset("small_heavy.csv", "small_heavy")
generate_dataset("large_heavy.csv", "large_heavy")
generate_dataset("mixed.csv", "mixed")
print("Done! 3 CSV files created.")
