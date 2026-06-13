# amend-price-change Runtime Stress

관련 문서:

- [Stress 테스트 홈](../../../README.md)
- [Runtime Stress 테스트](../../../runtime_stress.md)

`amend-price-change`는 GTC 주문을 넣은 뒤 가격을 바꾸고, 같은 주문 ID를 다시 취소하는 시나리오다.
가격 변경 정정은 orderbook에서 기존 주문을 제거하고 새 가격 레벨에 다시 넣는 cancel-replace 경로를 탄다.
Place/Amend/Cancel 3-command 묶음으로 구성되어 반복 종료 시 orderbook이 비워지므로 stationary 정정 경로 기준선으로 사용한다.

## 최신 TPS

## 최신 TPS

| 항목 | 값 |
| --- | --- |
| 최신 안정 TPS | `130,000` |
| 경계 TPS | `140,000` |
| 초과 구간 | `150,000+` |

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
docker compose run --rm runtime-stress --scenario amend-price-change --warmup-sec 10 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| [2026-06-13 baseline](./measurements/2026-06-13_baseline.md) | 130,000 | 140,000 | [raw](./raw/2026-06-13_baseline.log) |


새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
