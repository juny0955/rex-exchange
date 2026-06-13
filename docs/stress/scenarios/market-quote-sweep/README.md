# market-quote-sweep Stress Scenario

관련 문서:

- [Stress 테스트 홈](../../README.md)
- [Runtime 측정](./runtime/README.md)
- [Integration 측정](./integration/README.md)

`market-quote-sweep`은 여러 가격대의 ask를 Market Buy Quote 주문이 sweep하는 시나리오다.
가격 레벨을 지나며 체결 결과 payload가 커지는 경로를 관측한다.

## 측정 현황

| 측정 | 최신 상태 | 요약 |
| --- | --- | --- |
| Runtime | [2026-06-13 baseline](./runtime/measurements/2026-06-13_baseline.md) | 안정 120,000 TPS, 초과 150,000+ TPS |
| Integration | 미측정 | 로컬 Docker 환경에서 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
