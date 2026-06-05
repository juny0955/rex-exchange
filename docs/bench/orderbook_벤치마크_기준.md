# OrderBook 벤치마크 기준선

이 문서는 `benches/orderbook.rs`의 첫 로컬 기준선을 기록한다.
이후 `src/engine/orderbook.rs`를 최적화할 때 비교 기준으로 사용한다.

## 측정 환경

- 측정 시각: 2026-06-06 01:27:55 KST
- OS: Darwin 25.3.0 arm64
- Rust: `rustc 1.95.0 (59807616e 2026-04-14)`
- Cargo: `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- 벤치마크 실행 명령:

```bash
cargo bench --features bench-internals --bench orderbook
```

Criterion HTML 리포트는 아래 경로에 생성된다.

```text
target/criterion/report/index.html
```

개별 벤치마크의 JSON 추정치는 아래 경로에 생성된다.

```text
target/criterion/<benchmark-name>/.../new/estimates.json
```

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

## 최적화 후 비교 방법

1. `src/engine/orderbook.rs`를 변경하기 전에 clean working tree에서 기준 벤치를 한 번 실행한다.

```bash
cargo bench --features bench-internals --bench orderbook
```

2. 최적화를 적용한다.

3. 정확성 검증을 실행한다.

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

4. 동일한 벤치마크 명령을 다시 실행한다.

```bash
cargo bench --features bench-internals --bench orderbook
```

5. Criterion 리포트를 연다.

```text
target/criterion/report/index.html
```

Criterion은 이전 실행을 `base`, 최신 실행을 `new`로 보관하고 변화율을 출력한다.
최적화가 실제로 도움이 됐는지는 이 비교 결과를 1차 근거로 판단한다.

## 중점 확인 항목

- `remove_order`가 가장 우선적인 최적화 후보이다. front/middle/back 시간이 비슷하다는 것은 현재 `retain` 기반 삭제가 price queue 전체를 스캔한다는 뜻이다.
- `can_fully_fill_base`와 `can_fully_fill_quote`는 스캔하는 유동성 규모에 따라 증가한다. 이 경로가 병목이 되면 누적 잔량 캐시 같은 구조를 이 기준선과 비교한다.
- `get_best_opposite`는 현재 매우 작지만 `VecDeque<Uuid>`를 clone해서 반환한다. top-of-book queue가 커지는 상황을 다룰 때는 API를 바꾸기 전에 먼저 측정한다.

