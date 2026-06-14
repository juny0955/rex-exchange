# amend-decrease-qty Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`amend-decrease-qty`는 GTC 주문을 넣은 뒤 같은 가격에서 수량만 줄이고 다시 취소하는 시나리오다.
in-place 정정 비용을 관측한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-13 baseline](./runtime/measurements/2026-06-13_baseline.md) | 안정 130,000 TPS, 경계 140,000 TPS, 초과 250,000+ TPS |
| Integration | [2026-06-14 baseline-reset](./integration/measurements/2026-06-14_baseline-reset.md) | 안전 50,000 TPS, 경계 75,000 TPS, 포화 100,000+ (오더북 초기화 적용, 이전 20,000/25,000/30,000은 오더북 오염 상태) |
