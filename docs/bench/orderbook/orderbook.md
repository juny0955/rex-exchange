# OrderBook 벤치마크

`OrderBook` 벤치마크는 `src/engine/orderbook.rs`의 자료구조 비용을 직접 측정한다.
엔진 전체 처리량이 아니라 오더북 내부 hot path를 분리해서 보기 위한 벤치마크이다.

## 관련 문서

- [벤치마크 위키 홈](../README.md)
- [OrderBook 벤치마크 기준선](./orderbook_벤치마크_기준.md)

## 측정 내역

| 문서 | 용도 |
| --- | --- |
| [OrderBook 벤치마크 기준선](./orderbook_벤치마크_기준.md) | 기준 수치, 해석 포인트 |

새로운 `OrderBook` 최적화 측정 결과가 생기면 이 표에 개선 내역 문서를 추가한다.

## 측정 대상

`benches/orderbook.rs`는 아래 작업을 측정한다.

| 그룹 | 측정 내용 |
| --- | --- |
| `orderbook/add_order` | 빈 오더북에 N개 limit order를 추가하는 비용 |
| `orderbook/get_best_opposite` | 최우선 반대 호가를 조회하는 비용 |
| `orderbook/can_fully_fill_base` | base 수량 기준 FOK 전량 체결 가능 여부 확인 비용 |
| `orderbook/can_fully_fill_quote` | quote 금액 기준 FOK 전량 체결 가능 여부 확인 비용 |
| `orderbook/remove_order` | 같은 price level 안에서 front/middle/back 주문을 삭제하는 비용 |

입력 크기는 `100`, `1_000`, `10_000` orders 세 가지이다.

## 실행 방법

`OrderBook`은 일반 빌드에서 내부 모듈로 유지된다.
벤치마크에서만 접근할 수 있도록 `bench-internals` feature를 켜서 실행한다.

```bash
cargo bench --features bench-internals --bench orderbook
```
