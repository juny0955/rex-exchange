# partial-fill-rest Runtime Stress

관련 문서:

- [Stress 테스트 홈](../../../README.md)
- [Runtime Stress 테스트](../../../runtime_stress.md)

`partial-fill-rest`는 maker N개를 체결한 뒤 GTC taker 잔량을 orderbook에 남기는 시나리오다.
부분 체결 후 잔존 경로 자체는 정상 동작이지만, 반복 실행 중 잔존 주문이 다음 반복의 반대편 주문과 교차되므로 orderbook 상태가 시간에 따라 drift한다.
따라서 이 시나리오는 잔존 주문 누적이 있는 런타임 부하 관측값으로 관리하고, `full-fill-same-level`, `market-quote-sweep`처럼 반복마다 book이 비워지는 기준선과 직접 비교하지 않는다.

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
docker compose run --rm runtime-stress --scenario partial-fill-rest --warmup-sec 10 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| 미측정 | - | - | - |

새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
