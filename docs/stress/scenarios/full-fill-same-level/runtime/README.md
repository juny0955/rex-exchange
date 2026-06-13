# full-fill-same-level Runtime Stress

관련 문서:

- [Stress 테스트 홈](../../../README.md)
- [Runtime Stress 테스트](../../../runtime_stress.md)

`full-fill-same-level`은 같은 가격의 maker 주문 N개를 taker 하나가 전량 체결하는 시나리오다.
체결 결과와 maker snapshot payload가 sweep depth에 비례해서 증가하므로, 매칭 hot path와 result publish 경로를 함께 보는 기준 시나리오로 사용한다.

## 최신 TPS

| 항목 | 값 |
| --- | --- |
| 최신 안정 TPS | `100,000` |
| 경계 TPS | 미측정 |
| 초과 구간 | `120,000+` |

최신 기준선은 [2026-06-13 baseline](./measurements/2026-06-13_baseline.md)이다.

## 고정 실행 조건

| 항목 | 값 |
| --- | --- |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| 발행 지연 | `0ms` |
| warm-up 시간 | `10s` |
| 측정 시간 | `30s` |
| 제한 시간 | `30s` |
| 실행 모드 | `paced` |

대표 실행 명령:

```bash
docker compose run --rm runtime-stress --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| [2026-06-13 baseline](./measurements/2026-06-13_baseline.md) | 80,000 | 100,000 | [raw](./raw/2026-06-13_baseline.log) |

새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
