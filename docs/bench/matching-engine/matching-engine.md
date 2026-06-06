# MatchingEngine 벤치마크

`MatchingEngine` 벤치마크는 `src/engine/matching_engine.rs`의 주문 처리 경로를 측정한다.
채널 송수신이나 엔진 스레드 실행 비용이 아니라, 엔진 내부 로직의 비용을 보기 위한 벤치마크이다.

## 관련 문서

- [벤치마크 위키 홈](../README.md)
- [MatchingEngine 벤치마크 기준선](./matching-engine_벤치마크_기준.md)

## 측정 내역

| 문서 | 용도 |
| --- | --- |
| [MatchingEngine 벤치마크 기준선](./matching-engine_벤치마크_기준.md) | 기준 수치, 해석 포인트 |

새로운 `MatchingEngine` 최적화 측정 결과가 생기면 이 표에 개선 내역 문서를 추가한다.

## 측정 대상

`benches/matching-engine.rs`는 아래 작업을 측정한다.

| 그룹 | 측정 내용 |
| --- | --- |
| `matching_engine/place_order/resting_limit_no_cross` | 반대 호가가 있지만 가격 조건이 맞지 않아 GTC 주문이 잔존하는 비용 |
| `matching_engine/place_order/full_fill_same_level` | 같은 price level의 maker 주문 N개를 taker 주문 하나가 전량 체결하는 비용 |
| `matching_engine/place_order/market_quote_sweep` | Market Buy Quote 주문이 여러 maker 주문을 sweep하는 비용 |
| `matching_engine/place_order/fok_reject_insufficient_liquidity` | FOK 주문이 전량 체결 가능성 검증에서 취소되는 비용 |
| `matching_engine/cancel_order/existing_middle_same_price` | 같은 price level 중간 주문을 취소하는 비용 |
| `matching_engine/cancel_order/missing_order` | 존재하지 않는 주문 취소가 거부되는 비용 |
| `matching_engine/amend_order/decrease_qty_in_place` | 가격 유지와 수량 감소로 우선순위를 유지하는 정정 비용 |
| `matching_engine/amend_order/price_change_cancel_replace` | 가격 변경으로 기존 주문을 취소하고 재등록하는 정정 비용 |

입력 크기는 `10`, `100`, `1_000` maker orders 세 가지이다.

## 실행 방법

`MatchingEngine` 내부 API는 일반 빌드에서 private으로 유지된다.
벤치마크에서만 접근할 수 있도록 `bench-internals` feature를 켜서 실행한다.

```bash
cargo bench --features bench-internals --bench matching-engine
```

벤치마크 목록만 확인하려면 아래 명령을 사용한다.

```bash
cargo bench --features bench-internals --bench matching-engine -- --list
```
