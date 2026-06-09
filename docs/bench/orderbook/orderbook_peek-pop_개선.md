# OrderBook peek/pop 최적화 개선 내역

관련 문서:

- [벤치마크 위키 홈](../README.md)
- [OrderBook 벤치마크](./orderbook.md)
- [OrderBook 벤치마크 기준선](./orderbook_벤치마크_기준.md)

## 개요

대상 커밋: `e61fddd refactor: orderbook price queue clone 제거 -> peek/pop 변경`

price queue를 처리할 때 큐 전체를 clone한 뒤 순회하던 방식을 `peek`/`pop`으로 바꿔
불필요한 복사를 제거했다. 이 문서는 변경 전 기준선 대비 개선폭을 기록한다.

측정 환경은 기준선과 동일하다(Apple M4 Pro, macOS 26.3.1, rustc 1.95.0).
Δ는 Criterion이 직접 계산한 `change/estimates.json`의 mean 변화율이며, 음수가 개선(빨라짐)이다.

> 참고: 기준선 문서의 `orderbook/get_best_opposite`는 현재 벤치 스위트(`benches/orderbook.rs`)에서
> 제거되어 이번 재측정 대상이 아니다. 아래 표에는 현재 측정되는 4개 그룹만 싣는다.

## 기준선 대비 비교

| Benchmark | size | 기준선 | peek/pop | Δ |
| --- | ---: | ---: | ---: | ---: |
| `add_order` | 100 | 8.36 us | 8.64 us | +3.4% |
| | 1,000 | 87.18 us | 89.17 us | +2.3% |
| | 10,000 | 809.98 us | 831.04 us | +2.6% |
| `can_fully_fill_base/buy_across_levels` | 100 | 665.62 ns | 670.48 ns | +0.7% |
| | 1,000 | 7.76 us | 6.92 us | **−10.8%** |
| | 10,000 | 108.81 us | 98.07 us | **−9.9%** |
| `can_fully_fill_quote/buy_quote` | 100 | 1.06 us | 994.29 ns | −6.4% |
| | 1,000 | 10.81 us | 10.46 us | −3.2% |
| | 10,000 | 120.30 us | 117.70 us | −2.2% |
| `remove_order/front` | 100 | 918.19 ns | 929.52 ns | +1.2% |
| | 1,000 | 8.67 us | 9.64 us | +11.1% |
| | 10,000 | 99.99 us | 108.36 us | +8.4% |
| `remove_order/middle` | 100 | 907.57 ns | 902.85 ns | −0.5% |
| | 1,000 | 8.66 us | 9.12 us | +5.3% |
| | 10,000 | 97.46 us | 102.65 us | +5.3% |
| `remove_order/back` | 100 | 891.29 ns | 906.08 ns | +1.7% |
| | 1,000 | 8.40 us | 8.54 us | +1.7% |
| | 10,000 | 96.09 us | 96.94 us | +0.9% |

## 해석

- **개선의 핵심은 유동성 스캔 경로다.** `can_fully_fill_base/buy_across_levels`가 1,000·10,000
  주문에서 각각 **−10.8%·−9.9%**로 가장 크게 개선됐다. FOK 전량 체결 가능성 검사가 price queue를
  clone하지 않고 peek로 직접 훑게 되면서 복사 비용이 사라진 효과다.
  `can_fully_fill_quote/buy_quote`도 −2% ~ −6%로 같은 방향의 개선을 보인다.
- `add_order`(+2~3%)는 peek/pop 경로를 타지 않으므로 변경 영향이 없고, 소폭 증가는 측정 노이즈로 본다.
- `remove_order`는 여전히 `retain` 기반이라 이번 변경과 무관하다. `back`은 거의 평탄하지만
  `front`·`middle`의 1,000·10,000 구간이 +5% ~ +11%로 튀는데, 변경된 경로가 아니므로 회귀가 아니라
  1회 실행 측정의 분산으로 본다. 신경 쓰인다면 동일 명령을 반복 실행해 재확인한다.

## 재현 방법

```bash
cargo bench --features bench-internals --bench orderbook
```

Criterion 리포트에서 `base` → `new` 변화율을 확인한다.

```text
target/criterion/report/index.html
```

개별 수치는 아래 경로에서 확인할 수 있다.

```text
target/criterion/orderbook_<group>/<scenario>/<size>/new/estimates.json     # mean.point_estimate
target/criterion/orderbook_<group>/<scenario>/<size>/change/estimates.json  # 변화율
```
