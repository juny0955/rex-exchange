# partial-fill-rest Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`partial-fill-rest`는 maker 주문 N개를 체결한 뒤 taker 잔량이 orderbook에 남는 시나리오다.
반복 실행 중 잔존 주문이 누적되는 non-stationary workload로 해석한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-09 기준선](./runtime/measurements/2026-06-09_baseline.md) | 안정 50,000 commands/s, 경계 55,000 commands/s |
| Integration | 미측정 | 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
