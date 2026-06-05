# OrderBook 벤치마크 기준선

관련 문서:

- [벤치마크 위키 홈](../README.md)
- [OrderBook 벤치마크](./orderbook.md)

이 문서는 `benches/orderbook.rs`의 첫 로컬 기준선을 기록한다.
이후 `src/engine/orderbook.rs`를 최적화할 때 비교 기준으로 사용한다.

## 기준선 요약

아래 시간은 로컬 1회 실행에서 나온 Criterion mean point estimate 값이다.
절대적인 운영 환경 latency가 아니라, 이후 개선 전후를 비교하기 위한 기준선으로 사용한다.

| Benchmark | 100 orders | 1,000 orders | 10,000 orders |
| --- | ---: | ---: | ---: |
| `orderbook/add_order` | 8.36 us | 87.18 us | 809.98 us |
| `orderbook/get_best_opposite/buy_from_asks` | 15.66 ns | 26.55 ns | 77.99 ns |
| `orderbook/can_fully_fill_base/buy_across_levels` | 665.62 ns | 7.76 us | 108.81 us |
| `orderbook/can_fully_fill_quote/buy_quote` | 1.06 us | 10.81 us | 120.30 us |

| Benchmark | 100 orders | 1,000 orders | 10,000 orders |
| --- | ---: | ---: | ---: |
| `orderbook/remove_order/front` | 918.19 ns | 8.67 us | 99.99 us |
| `orderbook/remove_order/middle` | 907.57 ns | 8.66 us | 97.46 us |
| `orderbook/remove_order/back` | 891.29 ns | 8.40 us | 96.09 us |

## 해석 포인트

- `remove_order`는 현재 `retain` 기반이라 같은 price queue 전체를 스캔한다. front/middle/back 시간이 비슷하고 입력 크기에 따라 증가하면 O(n) 특성이 드러난다.
- `can_fully_fill_base`와 `can_fully_fill_quote`는 스캔하는 유동성 규모에 따라 증가한다. 이 경로가 병목이 되면 누적 잔량 캐시 같은 구조를 기준선과 비교한다.
- `get_best_opposite`는 현재 매우 작게 측정된다. 다만 현재 반환값에 `VecDeque<Uuid>` clone이 포함되므로 top-of-book queue가 커질 때 별도로 확인해야 한다.
