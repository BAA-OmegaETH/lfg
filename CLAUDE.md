# CLAUDE.md

## 0. 프로젝트 개요

### 프로젝트명

OmegaETH Sequencer Prototype

### 핵심 목표

본 프로젝트는 **Ethereum devnet 위에서 실제 blob transaction(EIP-4844)을 사용하여**,
Layer 2 시퀀서의 **트랜잭션 오더링 정책이 DA(Data Availability) 비용과 latency에 미치는 영향**을 실험적으로 분석하는 것을 목표로 한다.

---

## 1. 문제 정의

기존 L2 시퀀서의 기본 정책은 대부분 **FCFS (First-Come First-Served)** 이다.

문제:

* Blob은 고정 크기 (≈128KB)
* 덜 채워진 상태로 제출되면 **공간 낭비 → 비용 증가**
* 트랜잭션 간 압축률 차이 고려 안됨

---

## 2. 연구 질문

### RQ1

FCFS 기반 오더링에서 Blob 낭비는 얼마나 발생하는가?

### RQ2

DES (DA Efficiency Score) 적용 시 Blob 사용량은 얼마나 감소하는가?

### RQ3

오더링 최적화가 latency에 미치는 영향은?

---

## 3. 핵심 아이디어

### DES (DA Efficiency Score)

```
DES = α * wait_score
    + β * compress_score
    + γ * fit_score
```

#### 구성 요소

* wait_score
  → 오래 기다린 tx 우선 (starvation 방지)

* compress_score
  → 압축 잘 되는 tx 우선

* fit_score
  → 현재 blob 공간에 잘 맞는 tx 우선

---

## 4. 전체 시스템 아키텍처

```
[Tx Dataset / Generator]
            ↓
     [Custom Sequencer]
        - mempool
        - ordering (FCFS / DES)
        - execution
        - batch builder
            ↓
      [Blob Sender]
            ↓
[Local Ethereum Devnet]
  - Execution Client (Geth)
  - Consensus Client (Lighthouse)
```

---

## 5. 설계 철학

### 우리가 하지 않는 것

* Full rollup 구현 ❌
* zk proof ❌
* consensus 변경 ❌
* client 내부 수정 ❌

### 우리가 하는 것

* Sequencer만 직접 구현 ⭕
* Ordering 정책 실험 ⭕
* 실제 blob tx 사용 ⭕
* Execution path 포함 ⭕

---

## 6. 기술 스택

### 인프라

* Docker
* Kurtosis (Ethereum devnet)
* Geth (Execution client)
* Lighthouse (Consensus client)

### 개발

* Rust
* revm (EVM execution engine)
* VS Code + rust-analyzer

---

## 7. 개발 환경 세팅

### 7.1 필수 설치

* Rust (`rustup`)
* Docker Desktop
* Kurtosis CLI
* VS Code

---

### 7.2 폴더 구조

```
omegaeth/
  ├─ devnet/
  │   └─ network_params.yaml
  ├─ sequencer/
  │   ├─ Cargo.toml
  │   └─ src/
  │       ├─ main.rs
  │       ├─ config.rs
  │       ├─ types.rs
  │       ├─ mempool.rs
  │       ├─ ordering.rs
  │       ├─ executor.rs
  │       ├─ batcher.rs
  │       ├─ blob_sender.rs
  │       └─ metrics.rs
  ├─ datasets/
  └─ README.md
```

---

### 7.3 devnet 실행

```bash
kurtosis run github.com/ethpandaops/ethereum-package --args-file ./devnet/network_params.yaml
```

---

## 8. 핵심 모듈 설명

### 8.1 Tx 구조

```rust
pub struct UserTx {
    pub tx_id: u64,
    pub arrival_ms: u64,
    pub payload_size: usize,
    pub tx_type: String,
    pub gas_bid: u64,
}
```

---

### 8.2 Mempool

* tx 저장
* ordering 대상 제공
* 선택된 tx 제거

---

### 8.3 Ordering Engine

#### FCFS

* arrival 순

#### DES

* score 기반 정렬

---

### 8.4 Executor

* tx 실행 (revm)
* state update

---

### 8.5 Batcher

* tx → batch bytes 변환
* blob 크기 기준으로 묶음

---

### 8.6 Blob Sender

* batch → blob tx 변환
* devnet에 전송

---

### 8.7 Metrics

* blob count
* fill rate
* latency
* execution time

---

## 9. 실험 설계

### Baseline

* FCFS

### Proposed

* DES

---

### 비교 지표

* Total blob count
* Average fill rate
* Average latency
* Max latency
* Cost proxy

---

## 10. 실험 시나리오

### 1. 기본 비교

FCFS vs DES

### 2. 파라미터 sweep

α, β, γ 변경

### 3. workload 변화

* small tx
* large tx
* mixed

---

## 11. 개발 로드맵

### Phase 1

* devnet 구축
* blob tx 전송 성공

### Phase 2

* sequencer skeleton
* FCFS 구현

### Phase 3

* execution engine 연결

### Phase 4

* blob posting 연결

### Phase 5

* DES 구현

### Phase 6

* 실험 및 분석

---

## 12. 핵심 설계 결정

### 1. 외부 sequencer 구조

→ client 수정 없이 실험 가능

### 2. batch 단위 blob 사용

→ rollup 구조 반영

### 3. execution 포함

→ 단순 packing 문제 아님

---

## 13. 프로젝트 정의 (한 줄)

> Ethereum devnet 위에서 동작하는 custom sequencer를 구현하고,
> 실제 blob transaction을 사용하여 트랜잭션 오더링 정책의 DA 효율성과 latency를 비교 분석하는 연구

---

## 14. 구현 범위

### 포함

* ordering
* execution
* batch
* blob posting

### 제외

* zk proof
* fraud proof
* decentralized sequencer
* full L2 infra

---

## 15. 성공 기준

* blob tx 실제 전송 성공
* FCFS vs DES 비교 결과 도출
* latency vs cost tradeoff 시각화
* reproducible experiment

---

## 16. 핵심 한 줄 요약

> "실제 Ethereum 환경에서 blob을 사용하여, 시퀀서의 트랜잭션 오더링 전략이 비용과 지연에 미치는 영향을 측정하는 연구"

---
