$ResultsFile = "../results.txt"
Set-Content $ResultsFile "" -Encoding utf8

# Suppress all tracing INFO logs - only metrics println! will appear
$env:RUST_LOG = "error"

$datasets = @("small_heavy", "large_heavy", "mixed")
$policies = @("fcfs", "des")

foreach ($policy in $policies) {
    foreach ($dataset in $datasets) {
        $env:DATASET = "../$dataset.csv"
        $env:ORDERING_POLICY = $policy

        $header = "=== $dataset | $policy ==="
        Write-Host $header
        Add-Content $ResultsFile $header -Encoding utf8

        cargo run --release | Add-Content $ResultsFile -Encoding utf8
        Add-Content $ResultsFile "" -Encoding utf8
    }
}

Write-Host ""
Write-Host "Done. Results saved to results.txt"
