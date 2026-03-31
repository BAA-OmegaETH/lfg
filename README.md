# OmegaETH Sequencer Prototype

> Ethereum Deneb devnet 상에서 외부 custom sequencer를 구현하고, batch를 실제 blob transaction으로 게시하여 **FCFS와 DES ordering 정책**이 blob utilization과 transaction latency에 미치는 영향을 비교 분석하는 연구 프로젝트.

## 📋 프로젝트 개요

### 핵심 목표

Layer 2 시퀀서의 **트랜잭션 오더링 정책**이 **DA(Data Availability) 비용**과 **latency**에 미치는 영향을 실험적으로 분석

### 연구 질문

- **RQ1**: FCFS 기반 오더링에서 Blob 낭비는 얼마나 발생하는가?
- **RQ2**: DES (DA Efficiency Score) 적용 시 Blob 사용량은 얼마나 감소하는가?
- **RQ3**: 오더링 최적화가 latency에 미치는 영향은?

### DES (DA Efficiency Score)

```
DES = α * wait_score + β * compress_score + γ * fit_score
```

- `wait_score`: 대기 시간 (starvation 방지)
- `compress_score`: 압축률 추정
- `fit_score`: 현재 blob 공간 활용도

---

## 🏗️ 프로젝트 구조

```
omegaeth/
  ├─ devnet/
  │   └─ network_params.yaml    # Deneb(EIP-4844) devnet 설정
  ├─ sequencer/                  # Custom sequencer 구현
  │   ├─ Cargo.toml
  │   └─ src/
  │       ├─ main.rs             # 메인 루프
  │       ├─ config.rs           # 설정
  │       ├─ types.rs            # UserTx, Batch, Metrics
  │       ├─ mempool.rs          # 트랜잭션 풀
  │       ├─ ordering.rs         # FCFS/DES 정책 ✅
  │       ├─ executor.rs         # EVM 실행
  │       ├─ batcher.rs          # 배치 생성 + 압축 ✅
  │       ├─ blob_sender.rs      # Blob 전송 (TODO)
  │       └─ metrics.rs          # 메트릭 수집 ✅
  ├─ datasets/                   # 트랜잭션 데이터셋
  ├─ results/                    # 실험 결과 (CSV, 그래프)
  ├─ CLAUDE.md                   # 상세 설계 문서
  └─ README.md
```

---

## ✅ 현재 진행 상황

### Phase 1: Devnet 구축 ✅

- [x] Kurtosis 설치
- [x] Deneb fork 활성화 (EIP-4844 blob tx)
- [x] Fulu/Electra fork 비활성화 (불필요)
- [x] Blobscan, Prometheus, Grafana 설정
- [x] Devnet 실행 성공
- [x] Geth RPC 연결 확인 (`http://127.0.0.1:60337`)

### Phase 2: Sequencer 구현 🔄

- [x] 프로젝트 구조 생성
- [x] FCFS ordering 구현
- [x] **DES ordering 구현 완료**
- [x] Batcher (압축 포함) 구현
- [x] Metrics collector 구현
- [ ] **Blob sender 구현 (TODO)**
- [ ] Dataset generator 구현

### Phase 3: 실험 🔜

- [ ] FCFS baseline 실험
- [ ] DES 실험
- [ ] 결과 비교 (blob count, fill rate, latency)
- [ ] 시각화 및 분석

---

## 🚀 빠른 시작

### 1. 필수 도구 설치

```bash
# Rust (이미 설치됨)
rustup update

# Docker Desktop (필요)
brew install --cask docker

# Kurtosis CLI (이미 설치됨)
brew install kurtosis-tech/tap/kurtosis-cli
```

### 2. Devnet 실행

```bash
# 기존 enclave 정리
kurtosis clean -a

# Deneb devnet 시작 (Blobscan, Grafana 포함)
kurtosis run github.com/ethpandaops/ethereum-package \
  --args-file ./devnet/network_params.yaml

# RPC endpoint 확인
kurtosis enclave inspect <enclave-name>
```

**접속 가능한 서비스:**
- Geth RPC: `http://127.0.0.1:60337`
- Grafana: `http://127.0.0.1:60357`
- Prometheus: `http://127.0.0.1:60354`

### 3. Sequencer 빌드

```bash
cd sequencer
cargo build --release
```

### 4. 실험 실행

#### FCFS Baseline
```bash
cd sequencer
ORDERING_POLICY=fcfs cargo run --release
```

#### DES 실험
```bash
cd sequencer
ORDERING_POLICY=des cargo run --release
```

---

## 🧪 실험 설계

### Workload 시나리오

1. **Small-heavy**: 작은 tx 위주
2. **Large-heavy**: 큰 payload 위주
3. **Mixed**: small/medium/large 혼합

### 비교 지표

- `total_blob_count`: 사용된 blob 개수
- `average_blob_fill_rate`: blob 평균 사용률
- `wasted_blob_bytes`: 낭비된 공간
- `average_tx_latency`: 평균 대기 시간
- `p95_tx_latency`: tail latency
- `max_tx_latency`: 최대 대기 시간

---

## 📝 TODO

### 우선순위 High

- [ ] **Blob sender 구현** (alloy 사용, EIP-4844 blob tx 전송)
- [ ] **Dataset generator** (small/large/mixed workload)
- [ ] **메트릭 CSV 저장** 기능 추가

### 우선순위 Medium

- [ ] FCFS vs DES 실험 자동화 스크립트
- [ ] 결과 시각화 (Python/matplotlib)
- [ ] DES 파라미터 sweep (α, β, γ)

### 우선순위 Low

- [ ] Execution overhead 측정
- [ ] Burst arrival 시나리오
- [ ] 실시간 메트릭 대시보드

---

## 🛠️ 개발 환경

- **Rust**: 1.80+
- **Docker**: 최신 버전
- **Kurtosis**: 최신 버전
- **OS**: macOS (Darwin 24.6.0)

---

## 📚 참고 문서

- [CLAUDE.md](./CLAUDE.md) - 상세 설계 문서
- [EIP-4844](https://eips.ethereum.org/EIPS/eip-4844) - Shard Blob Transactions
- [Kurtosis Ethereum Package](https://github.com/ethpandaops/ethereum-package)

---

## 📊 예상 결과

FCFS 대비 DES의 개선 지표:
- Blob count: **15-30% 감소** (예상)
- Fill rate: **70% → 85%** (예상)
- Latency: trade-off 분석 필요

---

## 📄 라이센스

Research/Educational Purpose
