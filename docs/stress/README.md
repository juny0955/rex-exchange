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

## 로컬 Docker 고정 측정 환경

Runtime stress와 Integration stress의 공식 기준선과 개선 비교는 로컬 Docker 환경에서만 측정한다.
호스트에서 직접 실행한 결과나 다른 클라우드/VM에서 실행한 결과는 참고값으로만 다루고 공식 표에는 싣지 않는다.

| 항목 | 값 |
| --- | --- |
| Runtime | repo 루트 `Dockerfile`의 `runtime-stress` target |
| Runtime resource limit | CPU 1, memory 1GB, swap 1GB |
| Local host | MacBook Pro Mac16,8, Apple M4 Pro 12-core CPU(8P+4E), memory 24GB |
| Host OS | macOS 26.3.1(a), build 25D771280a |
| Docker Desktop | Docker 27.3.1, Linux arm64/aarch64, cgroup v2, overlay2 |
| Docker VM allocation | 8 CPUs, 4,109,737,984 bytes memory |
| Integration Kafka | repo 루트 `docker-compose.yml`의 `kafka` service |
| Integration Kafka resource limit | CPU 2, memory 2GB, swap 2GB |
| Integration SUT | repo 루트 `docker-compose.yml`의 `matching-engine` service |
| Integration SUT resource limit | CPU 1, memory 1GB, swap 1GB |
| Integration load generator | repo 루트 `docker-compose.yml`의 `integration-stress` service, 리소스 제한 없음 |

실행 방법은 [Runtime Stress 테스트](./runtime_stress.md)의 "Docker 제한 환경 실행"과
[Integration Stress 테스트](./integration_stress.md)의 사전 준비 절차를 따른다.

## 최신 Runtime TPS 요약

`최신 안정 TPS`는 로컬 Docker 환경에서 같은 target/s의 기록된 run이 모두 `채널 포화 = 0`, `완료 여부 = 예`, `발행 결과 ~= 접수 성공`을 만족할 때 인정한다.
일부 run에서만 channel full이 발생한 target/s는 `경계 TPS`로 분리한다.

| 시나리오 | 최신 측정 | 최신 안정 TPS | 경계 TPS | 초과 구간 | 요약 | raw |
| --- | --- | ---: | ---: | --- | --- | --- |
| `full-fill-same-level` | [2026-06-13 baseline](./scenarios/full-fill-same-level/runtime/measurements/2026-06-13_baseline.md) | 100,000 | - | 120,000+ | 120,000부터 channel full 발생 | [raw](./scenarios/full-fill-same-level/runtime/raw/2026-06-13_baseline.log) |
| `market-quote-sweep` | [2026-06-13 baseline](./scenarios/market-quote-sweep/runtime/measurements/2026-06-13_baseline.md) | 120,000 | - | 150,000+ | 150,000부터 channel full 발생 | [raw](./scenarios/market-quote-sweep/runtime/raw/2026-06-13_baseline.log) |
| `partial-fill-rest` | [2026-06-13 baseline](./scenarios/partial-fill-rest/runtime/measurements/2026-06-13_baseline.md) | 100,000 | 110,000 | 120,000+ | 110,000부터 channel full 발생 | [raw](./scenarios/partial-fill-rest/runtime/raw/2026-06-13_baseline.log) |
| `place-resting-limit` | [2026-06-13 baseline](./scenarios/place-resting-limit/runtime/measurements/2026-06-13_baseline.md) | 20,000 | 30,000 | 50,000+ | 30,000부터 channel full 발생 | [raw](./scenarios/place-resting-limit/runtime/raw/2026-06-13_baseline.log) |
| `cancel-resting-order` | [2026-06-13 baseline](./scenarios/cancel-resting-order/runtime/measurements/2026-06-13_baseline.md) | 90,000 | 100,000 | 200,000+ | 100,000부터 channel full 발생 | [raw](./scenarios/cancel-resting-order/runtime/raw/2026-06-13_baseline.log) |
| `amend-decrease-qty` | [2026-06-13 baseline](./scenarios/amend-decrease-qty/runtime/measurements/2026-06-13_baseline.md) | 130,000 | 140,000 | 250,000+ | 140,000부터 channel full 발생 | [raw](./scenarios/amend-decrease-qty/runtime/raw/2026-06-13_baseline.log) |
| `amend-price-change` | [2026-06-13 baseline](./scenarios/amend-price-change/runtime/measurements/2026-06-13_baseline.md) | 130,000 | 140,000 | 150,000+ | 140,000부터 channel full 발생 | [raw](./scenarios/amend-price-change/runtime/raw/2026-06-13_baseline.log) |

### 최신 Integration TPS 요약 (gRPC + Kafka)

`integration_stress`를 통해 측정한 종단(E2E) 성능 결과다. 전송은 모두 `SubmitBatch`(unit 전체를 batch RPC 1콜로 전송)를 사용한다.
**안전**은 `채널 포화(503) = 0` 또는 무시 가능한 수준이고 `target 달성률 ~= 100%`인 구간, **경계**는 `target 달성률`은 유지되지만 503 비율 또는 ack/E2E p99 지연이 급증하는 구간, **포화**는 503 비율이 두 자릿수%대이거나 `target 달성률`이 100% 미달인 구간을 가리킨다. 각 시나리오의 판정 근거는 해당 측정 문서의 "판정" 절을 따른다.

| 시나리오 | 최신 측정 | commands/unit | 안전 TPS | 경계 TPS | 포화 TPS | p99 지연(E2E, 안전) | 무손실 |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `full-fill-same-level` | [2026-06-13 baseline](./scenarios/full-fill-same-level/integration/measurements/2026-06-13_baseline.md) | 11 | 100,000 | 120,000 | 150,000+ | 22.910ms | 통과 |
| `market-quote-sweep` | [2026-06-13 baseline](./scenarios/market-quote-sweep/integration/measurements/2026-06-13_baseline.md) | 11 | 80,000 | 100,000 | 130,000+ | 14.511ms | 통과(≤100k) / 실패(130k) |
| `partial-fill-rest` | [2026-06-13 baseline](./scenarios/partial-fill-rest/integration/measurements/2026-06-13_baseline.md) | 11 | 100,000 | 120,000 | 150,000+ | 21.174ms | 통과 |
| `place-resting-limit` | [2026-06-13 baseline](./scenarios/place-resting-limit/integration/measurements/2026-06-13_baseline.md) | 1 | 25,000 | 40,000 | 50,000+ | 8.281ms | 통과 |
| `cancel-resting-order` | [2026-06-13 baseline](./scenarios/cancel-resting-order/integration/measurements/2026-06-13_baseline.md) | 20 | 10,000 | 15,000 | 30,000+ | 8.777ms | 통과 |
| `amend-decrease-qty` | [2026-06-13 baseline](./scenarios/amend-decrease-qty/integration/measurements/2026-06-13_baseline.md) | 3 | 20,000 | 25,000 | 30,000+ | 8.294ms | 통과 |
| `amend-price-change` | [2026-06-13 baseline](./scenarios/amend-price-change/integration/measurements/2026-06-13_baseline.md) | 3 | 5,000 | 10,000 | 20,000+ | 8.113ms | 통과 |

- commands/unit=11(`full-fill-same-level`, `market-quote-sweep`, `partial-fill-rest`)이 batch RPC 효과를 가장 크게 받아 안전 TPS가 80k~100k로 가장 높다.
- commands/unit=1(`place-resting-limit`)은 batch RPC가 RPC 콜 수를 줄이지 못해 안전 TPS가 25,000에 그치고, 포화도 503이 아닌 target 미달성(pacing 지연) 형태로 나타난다.
- commands/unit=20(`cancel-resting-order`)은 batch RPC 콜 절감 효과는 가장 크지만 단일 RPC가 채널에 던지는 명령 burst(512 units × 20)도 가장 커서 안전 TPS가 가장 낮다(10,000).
- commands/unit=3인 `amend-decrease-qty`(안전 20,000)와 `amend-price-change`(안전 5,000)는 commands/unit이 같아도 엔진 처리 비용(in-place 수량 갱신 vs cancel-replace 가격 레벨 이동) 차이로 안전선이 약 4배 차이난다.

## 운영 규칙

- 새 측정은 [Stress 시나리오](./scenarios/README.md)의 구조에 맞춰 추가한다.
- 측정 문서에는 로컬 호스트 스펙, Docker VM 할당, Docker 리소스 제한, 실행 옵션을 함께 기록한다.
- 개선 후 측정은 새 문서로 추가하고, 기준선 대비 변화율을 기록한다.
- Runtime 측정의 `완료 여부 = 예`는 접수된 command만 완료됐다는 뜻이며, 거부된 command까지 처리됐다는 뜻이 아니다.
- Integration 측정의 `무손실 판정 = 통과`는 접수 성공한 명령이 Kafka 이벤트로 정확히 한 번 수신됐다는 뜻이다.
