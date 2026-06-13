# place-resting-limit Runtime Stress

관련 문서:

- [Stress 테스트 홈](../../../README.md)
- [Runtime Stress 테스트](../../../runtime_stress.md)

`place-resting-limit`는 체결되지 않는 GTC limit 주문을 orderbook에 계속 추가하는 시나리오다.
체결 수와 maker 갱신 수는 발생하지 않지만, 반복 실행 중 잔존 주문 수가 계속 증가하므로 orderbook 상태가 시간에 따라 drift한다.
따라서 이 시나리오는 미체결 주문 누적이 있는 런타임 부하 관측값으로 관리하고, `full-fill-same-level`, `market-quote-sweep`처럼 반복마다 book이 비워지는 기준선과 직접 비교하지 않는다.

## 최신 TPS

| 항목 | 값 |
| --- | --- |
| 최신 안정 TPS | 미측정 |
| 경계 TPS | 미측정 |
| 초과 구간 | 미측정 |

로컬 Docker 환경에서 새 측정을 진행한 뒤 이 표를 갱신한다.

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
docker compose run --rm runtime-stress --scenario place-resting-limit --warmup-sec 10 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| 미측정 | - | - | - |

새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
