# cancel-resting-order Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`cancel-resting-order`는 같은 가격에 쌓은 GTC 주문을 다시 취소하는 시나리오다.
orderbook 삽입/삭제 비용과 성공 취소 경로를 관측한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-09 기준선](./runtime/measurements/2026-06-09_baseline.md) | 안정 200,000 commands/s, 경계 300,000-900,000 commands/s |
| Integration | 미측정 | 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
