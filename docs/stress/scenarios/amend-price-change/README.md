# amend-price-change Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`amend-price-change`는 GTC 주문을 넣은 뒤 가격을 바꾸고 다시 취소하는 시나리오다.
cancel-replace 정정 비용을 관측한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-09 기준선](./runtime/measurements/2026-06-09_baseline.md) | 안정 300,000 commands/s, 경계 500,000-700,000 commands/s |
| Integration | 미측정 | 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
