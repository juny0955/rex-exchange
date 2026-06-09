# market-quote-sweep Runtime Stress

관련 문서:

- [Runtime Stress 위키 홈](../../README.md)
- [Runtime Stress 테스트](../../runtime_stress.md)
- [2026-06-09 기준선](./measurements/2026-06-09_baseline.md)

`market-quote-sweep`은 여러 가격대의 ask를 Market Buy Quote 주문이 sweep하는 시나리오다.
가격 레벨을 건너뛰며 quote 기준 체결 수량을 계산하므로, `full-fill-same-level`과 함께 체결 hot path를 비교하는 기준 시나리오로 사용한다.

## 최신 TPS

| 항목 | 값 |
| --- | ---: |
| 최신 안정 TPS | 300,000 commands/s |
| 경계 TPS | 330,000 commands/s |
| 초과 구간 | 370,000 commands/s |

최신 안정 TPS는 같은 target/s의 기록된 run이 모두 `채널 포화 = 0`, `완료 여부 = 예`, `발행 결과 ~= 접수 성공`을 만족한 가장 높은 target/s이다.

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
./target/release/runtime_stress --scenario market-quote-sweep --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 300000 --symbols 1 --sweep-depth 10 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| [2026-06-09 기준선](./measurements/2026-06-09_baseline.md) | 300,000 commands/s | 330,000 commands/s | [raw](./raw/2026-06-09_baseline.log) |

새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
