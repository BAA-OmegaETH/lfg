$ResultsFile = "../results_sweep.txt"
Set-Content $ResultsFile "" -Encoding utf8

$env:RUST_LOG = "error"
$env:ORDERING_POLICY = "des"
$env:DATASET = "../large_heavy.csv"

# Key weight combinations: (alpha=wait, beta=compress, gamma=fit)
$sweeps = @(
    @{ a="1.00"; b="0.00"; c="0.00"; label="pure_wait" },
    @{ a="0.00"; b="1.00"; c="0.00"; label="pure_compress" },
    @{ a="0.00"; b="0.00"; c="1.00"; label="pure_fit" },
    @{ a="0.33"; b="0.33"; c="0.34"; label="equal (default)" },
    @{ a="0.50"; b="0.25"; c="0.25"; label="wait_heavy" },
    @{ a="0.25"; b="0.25"; c="0.50"; label="fit_heavy" },
    @{ a="0.25"; b="0.50"; c="0.25"; label="compress_heavy" },
    @{ a="0.00"; b="0.50"; c="0.50"; label="no_wait" }
)

foreach ($s in $sweeps) {
    $env:DES_ALPHA = $s.a
    $env:DES_BETA  = $s.b
    $env:DES_GAMMA = $s.c

    $header = "=== DES large_heavy | a=$($s.a) b=$($s.b) g=$($s.c) ($($s.label)) ==="
    Write-Host $header
    Add-Content $ResultsFile $header -Encoding utf8

    cargo run --release | Add-Content $ResultsFile -Encoding utf8
    Add-Content $ResultsFile "" -Encoding utf8
}

# Also run FCFS baseline for reference
$env:ORDERING_POLICY = "fcfs"
Remove-Item Env:\DES_ALPHA -ErrorAction SilentlyContinue
Remove-Item Env:\DES_BETA  -ErrorAction SilentlyContinue
Remove-Item Env:\DES_GAMMA -ErrorAction SilentlyContinue

$header = "=== FCFS large_heavy | baseline ==="
Write-Host $header
Add-Content $ResultsFile $header -Encoding utf8
cargo run --release | Add-Content $ResultsFile -Encoding utf8

Write-Host ""
Write-Host "Done. Results saved to results_sweep.txt"
