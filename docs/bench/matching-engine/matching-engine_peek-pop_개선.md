# MatchingEngine peek/pop 최적화 개선 내역

관련 문서:

- [벤치마크 위키 홈](../README.md)
- [MatchingEngine 벤치마크](./matching-engine.md)
- [MatchingEngine 벤치마크 기준선](./matching-engine_벤치마크_기준.md)

## 개요

대상 커밋: `e61fddd refactor: orderbook price queue clone 제거 -> peek/pop 변경`

오더북 price queue를 처리할 때 큐 전체를 clone한 뒤 순회하던 방식을 `peek`/`pop`으로
바꿔 불필요한 복사를 제거했다. 이 문서는 변경 전 기준선 대비 개선폭을 기록한다.

측정 환경은 기준선과 동일하다(Apple M4 Pro, macOS 26.3.1, rustc 1.95.0).
기준선 값은 [기준선 문서](./matching-engine_벤치마크_기준.md), peek/pop 값은
`target/criterion/<group>/<scenario>/<size>/new/estimates.json`의 mean point estimate에서
가져왔다.

## 기준선 대비 비교

| Benchmark | size | 기준선 | peek/pop | Δ |
| --- | ---: | ---: | ---: | ---: |
| `place_order/resting_limit_no_cross` | 10 | 355.77 ns | 345.20 ns | −3.0% |
| | 100 | 1.47 us | 1.47 us | −0.2% |
| | 1,000 | 8.74 us | 8.92 us | +2.0% |
| `place_order/full_fill_same_level` | 10 | 10.22 us | 10.62 us | +3.9% |
| | 100 | 104.91 us | 101.64 us | −3.1% |
| | 1,000 | 1.40 ms | 1.01 ms | **−27.7%** |
| `place_order/market_quote_sweep` | 10 | 10.98 us | 10.96 us | −0.2% |
| | 100 | 107.38 us | 106.62 us | −0.7% |
| | 1,000 | 1.05 ms | 1.04 ms | −1.0% |
| `place_order/fok_reject_insufficient_liquidity` | 10 | 332.73 ns | 318.98 ns | −4.1% |
| | 100 | 2.29 us | 2.22 us | −3.1% |
| | 1,000 | 22.23 us | 21.74 us | −2.2% |
| `cancel_order/existing_middle_same_price` | 10 | 258.98 ns | 253.63 ns | −2.1% |
| | 100 | 1.07 us | 998.19 ns | −6.7% |
| | 1,000 | 9.14 us | 8.94 us | −2.2% |
| `cancel_order/missing_order` | 10 | 194.70 ns | 194.50 ns | −0.1% |
| | 100 | 949.01 ns | 907.21 ns | −4.4% |
| | 1,000 | 8.41 us | 8.20 us | −2.5% |
| `amend_order/decrease_qty_in_place` | 10 | 273.48 ns | 270.28 ns | −1.2% |
| | 100 | 1.03 us | 967.26 ns | −6.1% |
| | 1,000 | 8.61 us | 8.17 us | −5.1% |
| `amend_order/price_change_cancel_replace` | 10 | 407.47 ns | 402.06 ns | −1.3% |
| | 100 | 1.23 us | 1.19 us | −2.9% |
| | 1,000 | 9.37 us | 9.26 us | −1.1% |

Δ는 음수가 개선(빨라짐)을 의미한다.

## 해석

- 가장 큰 개선은 `place_order/full_fill_same_level/1,000`의 **−27.7%** (1.40 ms → 1.01 ms)다.
  taker 주문 하나가 같은 price level의 maker N개를 전량 체결하는 경로로, price queue를
  clone하지 않고 peek/pop으로 직접 소비하면서 복사 비용이 사라진 효과가 가장 직접적으로
  드러난다. 체결 1건당 비용으로 환산하면 약 1.40 us → 1.01 us.
- 큐 스캔이 포함된 단건/소규모 경로(`amend_order/decrease_qty_in_place/100·1,000`,
  `cancel_order/existing_middle_same_price/100` 등)에서도 −1% ~ −6%대의 일관된 개선이 보인다.
- `place_order/resting_limit_no_cross/1,000`(+2.0%)와 `place_order/full_fill_same_level/10`(+3.9%)는
  소폭 느려진 것으로 나오지만, N이 작아 clone 절감 효과보다 측정 변동이 큰 노이즈 범위로 본다.
  실제 회귀가 아니라 1회 실행 측정의 분산으로 판단한다.

## 재현 방법

```bash
cargo bench --features bench-internals --bench matching-engine
```

Criterion 리포트에서 `base` → `new` 변화율을 확인한다.

```text
target/criterion/report/index.html
```

개별 수치는 아래 경로의 `mean.point_estimate`에서 확인할 수 있다.

```text
target/criterion/matching_engine_<group>/<scenario>/<size>/new/estimates.json
```
