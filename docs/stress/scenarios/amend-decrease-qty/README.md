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
| Runtime | 미측정 | 로컬 Docker 환경에서 첫 runtime 측정 후 `runtime/measurements/`와 `runtime/raw/`에 기록 |
| Integration | 미측정 | 로컬 Docker 환경에서 첫 E2E 측정 후 `integration/measurements/`와 `integration/raw/`에 기록 |
