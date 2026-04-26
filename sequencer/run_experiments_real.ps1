$ResultsFile = "../results_real.txt"
Set-Content $ResultsFile "" -Encoding utf8

$env:RUST_LOG = "error"

# Generate real_full.csv if it doesn't exist
if (-not (Test-Path "../real_full.csv")) {
    Write-Host "Generating real_full.csv..."
    python "../prepare_mempool_data.py"
}

$policies = @("fcfs", "des")

foreach ($policy in $policies) {
    $env:DATASET = "../real_full.csv"
    $env:ORDERING_POLICY = $policy

    $header = "=== real_full | $policy ==="
    Write-Host $header
    Add-Content $ResultsFile $header -Encoding utf8

    cargo run --release | Add-Content $ResultsFile -Encoding utf8
    Add-Content $ResultsFile "" -Encoding utf8
}

Write-Host ""
Write-Host "Done. Results saved to results_real.txt"
