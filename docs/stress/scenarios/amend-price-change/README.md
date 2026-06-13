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
| Runtime | [2026-06-13 baseline](./runtime/measurements/2026-06-13_baseline.md) | 안정 130,000 TPS, 경계 140,000 TPS, 초과 150,000+ TPS |
| Integration | 미측정 | 로컬 Docker 환경에서 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
