# full-fill-same-level Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`full-fill-same-level`은 같은 가격의 maker 주문 N개를 taker 하나가 전량 체결하는 기본 시나리오다.
체결 결과와 maker snapshot payload가 sweep depth에 비례해 증가하므로, 매칭 hot path와 결과 발행 경로를 함께 보는 기준 시나리오로 사용한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-13 baseline](./runtime/measurements/2026-06-13_baseline.md) | 안정 100,000 TPS, 초과 120,000+ TPS |
| Integration | [2026-06-13 baseline](./integration/measurements/2026-06-13_baseline.md) | 안전 100,000 TPS, 경계 120,000 TPS, 포화 150,000+ |
