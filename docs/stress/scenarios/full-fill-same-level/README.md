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
| Runtime | [2026-06-09 기준선](./runtime/measurements/2026-06-09_baseline.md) | 안정 200,000 commands/s, 경계 300,000 commands/s |
| Integration | 미측정 | 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
