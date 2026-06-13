# place-resting-limit Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`place-resting-limit`은 체결 없이 GTC limit 주문을 orderbook에 계속 쌓는 시나리오다.
잔존 주문이 누적되므로 stationary 기준선과 직접 비교하지 않고 주문 누적 부하 관측값으로 본다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-13 baseline](./runtime/measurements/2026-06-13_baseline.md) | 안정 20,000 TPS, 경계 30,000 TPS, 초과 50,000+ TPS |
| Integration | [2026-06-13 baseline](./integration/measurements/2026-06-13_baseline.md) | 안전 25,000 TPS, 경계 40,000 TPS, 포화 50,000+ |
