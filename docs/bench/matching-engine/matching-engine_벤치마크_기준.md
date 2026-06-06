# MatchingEngine 벤치마크 기준선

관련 문서:

- [벤치마크 위키 홈](../README.md)
- [MatchingEngine 벤치마크](./matching-engine.md)

이 문서는 `benches/matching-engine.rs`의 첫 로컬 기준선을 기록하는 공간이다.
이후 `src/engine/matching_engine.rs`를 최적화할 때 비교 기준으로 사용한다.

## 기준선 요약

아래 시간은 로컬 1회 실행에서 나온 Criterion mean point estimate 값이다.
절대적인 운영 환경 latency가 아니라, 이후 개선 전후를 비교하기 위한 기준선으로 사용한다.

| Benchmark | 10 orders | 100 orders | 1,000 orders |
| --- | ---: | ---: | ---: |
| `matching_engine/place_order/resting_limit_no_cross` | 355.77 ns | 1.47 us | 8.74 us |
| `matching_engine/place_order/full_fill_same_level` | 10.22 us | 104.91 us | 1.40 ms |
| `matching_engine/place_order/market_quote_sweep` | 10.98 us | 107.38 us | 1.05 ms |
| `matching_engine/place_order/fok_reject_insufficient_liquidity` | 332.73 ns | 2.29 us | 22.23 us |
| `matching_engine/cancel_order/existing_middle_same_price` | 258.98 ns | 1.07 us | 9.14 us |
| `matching_engine/cancel_order/missing_order` | 194.70 ns | 949.01 ns | 8.41 us |
| `matching_engine/amend_order/decrease_qty_in_place` | 273.48 ns | 1.03 us | 8.61 us |
| `matching_engine/amend_order/price_change_cancel_replace` | 407.47 ns | 1.23 us | 9.37 us |

## 해석 포인트

- `place_order/full_fill_same_level`과 `market_quote_sweep`은 maker 주문 수만큼 체결 결과와 maker snapshot을 생성한다.
- `place_order/fok_reject_insufficient_liquidity`은 실제 체결 없이 FOK 사전 유동성 검증 경로를 측정한다.
- `cancel_order/existing_middle_same_price`와 `amend_order/price_change_cancel_replace`는 현재 오더북의 price queue 스캔 비용을 포함한다.
- 엔진 내부의 `Uuid::now_v7()` trade id 생성과 `Utc::now()` 상태 갱신은 production path 비용으로 포함된다.
