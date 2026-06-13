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
| Runtime | repo 루트 `Dockerfile`로 빌드한 `runtime_stress` 이미지 |
| Runtime resource limit | `--cpus=1 --memory=1g --memory-swap=1g` |
| Integration Kafka | repo 루트 `docker-compose.yml`의 Kafka 브로커 |
| Integration SUT / load generator | 같은 로컬 Docker 환경의 배치와 실행 옵션을 측정 문서에 기록 |

실행 방법은 [Runtime Stress 테스트](./runtime_stress.md)의 "Docker 제한 환경 실행"과
[Integration Stress 테스트](./integration_stress.md)의 사전 준비 절차를 따른다.

## 최신 Runtime TPS 요약

`최신 안정 TPS`는 로컬 Docker 환경에서 같은 target/s의 기록된 run이 모두 `채널 포화 = 0`, `완료 여부 = 예`, `발행 결과 ~= 접수 성공`을 만족할 때 인정한다.
일부 run에서만 channel full이 발생한 target/s는 `경계 TPS`로 분리한다.

| 시나리오 | 최신 측정 | 최신 안정 TPS | 경계 TPS | 초과 구간 | 요약 | raw |
| --- | --- | ---: | ---: | --- | --- | --- |
| `full-fill-same-level` | 미측정 | - | - | - | - | - |
| `market-quote-sweep` | 미측정 | - | - | - | - | - |
| `partial-fill-rest` | 미측정 | - | - | - | - | - |
| `place-resting-limit` | 미측정 | - | - | - | - | - |
| `cancel-resting-order` | 미측정 | - | - | - | - | - |
| `amend-decrease-qty` | 미측정 | - | - | - | - | - |
| `amend-price-change` | 미측정 | - | - | - | - | - |

## 운영 규칙

- 새 측정은 [Stress 시나리오](./scenarios/README.md)의 구조에 맞춰 추가한다.
- 측정 문서에는 로컬 Docker 환경과 실행 옵션을 함께 기록한다.
- 개선 후 측정은 새 문서로 추가하고, 기준선 대비 변화율을 기록한다.
- Runtime 측정의 `완료 여부 = 예`는 접수된 command만 완료됐다는 뜻이며, 거부된 command까지 처리됐다는 뜻이 아니다.
- Integration 측정의 `무손실 판정 = 통과`는 접수 성공한 명령이 Kafka 이벤트로 정확히 한 번 수신됐다는 뜻이다.
