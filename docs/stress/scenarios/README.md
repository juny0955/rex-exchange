# Stress 시나리오

이 디렉터리는 stress 측정 결과를 시나리오 중심으로 관리한다.
각 시나리오 아래에서 `runtime_stress`와 `integration_stress` 결과를 분리해 기록한다.

## 문서 구조

```text
docs/stress/scenarios/<scenario>/
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

- `README.md`: 시나리오 목적과 runtime/integration 측정 링크를 함께 보여준다.
- `runtime/`: `EngineRuntime` 인프로세스 경로의 처리량, 채널 포화, result publish 결과를 기록한다.
- `integration/`: gRPC 인그레스부터 Kafka 수신까지의 E2E 지연, 무손실, 백프레셔 결과를 기록한다.

## 시나리오 목록

| 시나리오 | 용도 | Runtime | Integration |
| --- | --- | --- | --- |
| `full-fill-same-level` | 동일 호가 전량 체결 | [runtime](./full-fill-same-level/runtime/README.md) | [integration](./full-fill-same-level/integration/README.md) |
| `market-quote-sweep` | 시장가 금액 스윕 | [runtime](./market-quote-sweep/runtime/README.md) | [integration](./market-quote-sweep/integration/README.md) |
| `partial-fill-rest` | 부분 체결 후 잔존 | [runtime](./partial-fill-rest/runtime/README.md) | [integration](./partial-fill-rest/integration/README.md) |
| `place-resting-limit` | 미체결 지정가 잔존 | [runtime](./place-resting-limit/runtime/README.md) | [integration](./place-resting-limit/integration/README.md) |
| `cancel-resting-order` | 잔존 주문 취소 | [runtime](./cancel-resting-order/runtime/README.md) | [integration](./cancel-resting-order/integration/README.md) |
| `amend-decrease-qty` | 수량 감소 정정 | [runtime](./amend-decrease-qty/runtime/README.md) | [integration](./amend-decrease-qty/integration/README.md) |
| `amend-price-change` | 가격 변경 정정 | [runtime](./amend-price-change/runtime/README.md) | [integration](./amend-price-change/integration/README.md) |

`cancel-missing`는 보조 진단 시나리오다. 기준선 측정이 필요해지면 같은 구조로 디렉터리를 추가한다.

## 측정 추가 규칙

- 새 runtime 측정은 `scenarios/<scenario>/runtime/measurements/`와 `runtime/raw/`에 같은 날짜/목적 이름으로 추가한다.
- 새 integration 측정은 `scenarios/<scenario>/integration/measurements/`와 `integration/raw/`에 같은 날짜/목적 이름으로 추가한다.
- Runtime TPS와 Integration TPS는 측정 경로와 병목이 다르므로 직접 비교하지 않는다.
- 측정 문서에는 로컬 Docker 환경과 실행 옵션을 함께 기록한다.
