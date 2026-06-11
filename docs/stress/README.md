# Stress 테스트 위키

이 디렉터리는 매칭엔진 부하 테스트의 실행 방법, 시나리오별 기준선, 개선 비교 결과를 관리하는 공간이다.
Criterion 벤치마크 문서는 `docs/bench`에 두고, 채널 포화, result publish, gRPC, Kafka를 포함한 부하 측정은 이 위키에서 관리한다.

## 문서 구조

| 문서 | 용도 |
| --- | --- |
| [Runtime Stress 테스트](./runtime_stress.md) | `runtime_stress` 도구의 목적, 옵션, 시나리오, 실행 방법 |
| [Integration Stress 테스트](./integration_stress.md) | `integration_stress` 도구의 목적, 옵션, 시나리오, gRPC+Kafka 종단 실행 방법 |
| [Stress 시나리오](./scenarios/README.md) | 시나리오별 runtime/integration 측정 결과와 문서 구조 |

측정 결과는 시나리오를 1차 축으로 관리한다.

```text
scenarios/<scenario>/
  README.md
  runtime/
    README.md
    measurements/
    raw/
  integration/
    README.md
    measurements/
    raw/
```

## 측정 경로 구분

| 측정 | 경로 | 용도 |
| --- | --- | --- |
| Runtime | `scenarios/<scenario>/runtime/` | `EngineRuntime` 인프로세스 경로의 처리량, 채널 포화, result publish 병목 확인 |
| Integration | `scenarios/<scenario>/integration/` | gRPC 인그레스부터 Kafka 수신까지의 E2E 지연, 무손실, 백프레셔 확인 |

Runtime TPS와 Integration TPS는 측정 경로와 병목이 다르므로 직접 비교하지 않는다.

## Runtime 고정 측정 환경

현재 runtime stress 기준선과 개선 비교는 아래 환경에서 고정 측정한다.
다른 환경에서 실행한 결과는 이 기준선과 직접 비교하지 않는다.

| 항목 | 값 |
| --- | --- |
| Cloud Provider | Vultr |
| OS | Ubuntu 24.04 |
| Kernel | Linux 6.8.0-124-generic x86_64 |
| Architecture | x86_64 |
| CPU | AMD EPYC-Rome Processor |
| vCPU | 2 |
| Physical Core / Thread | 1 Core / 2 Threads |
| CPU Clock | 2.0GHz |
| Virtualization | Full virtualization, QEMU |
| NUMA Node | 1 |
| L1 Cache | 32 KiB I-cache / 32 KiB D-cache |
| L2 Cache | 512 KiB |
| L3 Cache | 16 MiB |
| Memory | 3.8 GiB |
| Available Memory | 3.3 GiB |
| Swap | 8.0 GiB, unused |

Integration stress는 SUT, Kafka 브로커, 부하 생성기의 배치를 측정 문서마다 함께 기록한다.

## 최신 Runtime TPS 요약

`최신 안정 TPS`는 같은 target/s의 기록된 run이 모두 `채널 포화 = 0`, `완료 여부 = 예`, `발행 결과 ~= 접수 성공`을 만족할 때 인정한다.
일부 run에서만 channel full이 발생한 target/s는 `경계 TPS`로 분리한다.

| 시나리오 | 최신 측정 | 최신 안정 TPS | 경계 TPS | 초과 구간 | 요약 | raw |
| --- | --- | ---: | ---: | --- | --- | --- |
| `full-fill-same-level` | 2026-06-09 기준선 | 200,000 commands/s | 300,000 commands/s | 380,000+ commands/s | [summary](./scenarios/full-fill-same-level/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/full-fill-same-level/runtime/raw/2026-06-09_baseline.log) |
| `market-quote-sweep` | 2026-06-09 기준선 | 300,000 commands/s | 330,000 commands/s | 370,000 commands/s | [summary](./scenarios/market-quote-sweep/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/market-quote-sweep/runtime/raw/2026-06-09_baseline.log) |
| `partial-fill-rest` | 2026-06-09 기준선 | 50,000 commands/s | 55,000 commands/s | 55,000+ commands/s | [summary](./scenarios/partial-fill-rest/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/partial-fill-rest/runtime/raw/2026-06-09_baseline.log) |
| `place-resting-limit` | 2026-06-09 기준선 | 10,000 commands/s | 15,000 commands/s | 15,000+ commands/s | [summary](./scenarios/place-resting-limit/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/place-resting-limit/runtime/raw/2026-06-09_baseline.log) |
| `cancel-resting-order` | 2026-06-09 기준선 | 200,000 commands/s | 300,000-900,000 commands/s | 950,000+ commands/s | [summary](./scenarios/cancel-resting-order/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/cancel-resting-order/runtime/raw/2026-06-09_baseline.log) |
| `amend-decrease-qty` | 2026-06-09 기준선 | 500,000 commands/s | 700,000-850,000 commands/s | 900,000+ commands/s | [summary](./scenarios/amend-decrease-qty/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/amend-decrease-qty/runtime/raw/2026-06-09_baseline.log) |
| `amend-price-change` | 2026-06-09 기준선 | 300,000 commands/s | 500,000-700,000 commands/s | 800,000+ commands/s | [summary](./scenarios/amend-price-change/runtime/measurements/2026-06-09_baseline.md) | [raw](./scenarios/amend-price-change/runtime/raw/2026-06-09_baseline.log) |

## 운영 규칙

- 새 측정은 [Stress 시나리오](./scenarios/README.md)의 구조에 맞춰 추가한다.
- 기준선 문서는 오탈자나 계산 오류를 제외하고 수정하지 않는다.
- 개선 후 측정은 새 문서로 추가하고, 기준선 대비 변화율을 기록한다.
- Runtime 측정의 `완료 여부 = 예`는 접수된 command만 완료됐다는 뜻이며, 거부된 command까지 처리됐다는 뜻이 아니다.
- Integration 측정의 `무손실 판정 = 통과`는 접수 성공한 명령이 Kafka 이벤트로 정확히 한 번 수신됐다는 뜻이다.
