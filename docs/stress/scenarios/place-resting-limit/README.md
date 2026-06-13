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
| Runtime | 미측정 | 로컬 Docker 환경에서 첫 runtime 측정 후 `runtime/measurements/`와 `runtime/raw/`에 기록 |
| Integration | 미측정 | 로컬 Docker 환경에서 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
