# Matching Engine Contract

이 문서는 주문 서비스가 매칭엔진을 호출할 때 따라야 하는 계약을 정의한다.
현재 매칭엔진은 production-lite 실행 컴포넌트이며, durable order store는 주문 서비스 책임이다.

## Command ID

`command_id`는 매칭엔진 클라이언트가 생성한다. 운영 경로에서는 주문 서비스가 유일한 매칭엔진 클라이언트이므로 주문 서비스가 생성한다.

- 외부 client가 생성하는 값은 `client_order_id`다.
- 주문 서비스는 내부 `order_id`를 생성한다.
- 주문 서비스는 매칭엔진으로 보내는 place/cancel/amend 명령마다 새 `command_id`를 생성한다.
- gRPC timeout이나 ACK 유실 때문에 같은 명령을 재시도할 때는 같은 `command_id`를 재사용한다.

Durable idempotency, command replay, 주문 상태 복구는 주문 서비스가 담당한다.

## ACK Semantics

gRPC `CommandAck`는 최종 체결 결과가 아니다.

- `ACK_STATUS_ACCEPTED`: 매칭엔진 경계에 접수됐다.
- `ACK_STATUS_REJECTED`: 엔진 진입 전 boundary reject다.
- `ACK_STATUS_RESOURCE_EXHAUSTED`: 엔진 채널 포화다. 재시도 가능하다.
- `ACK_STATUS_UNAVAILABLE`: 엔진 중지 또는 결과 발행기 비정상 상태다. 재시도 가능하다.

주문 서비스는 `ACCEPTED`를 주문 최종 상태로 저장하면 안 된다. 최종 상태는 Kafka execution report로만 갱신한다.

## Execution Report

매칭엔진까지 들어간 명령은 처리 결과를 Kafka execution report로 발행한다.

- Kafka topic 기본값은 `matching-engine-events`다.
- Kafka key는 `symbol`이다.
- `schema_version`은 운영 전 안정화 버전이므로 `1`로 고정한다.
- event envelope에는 `command_id`, `order_id`, `symbol`, `engine_sequence`, `processed_at`이 포함된다.
- `engine_sequence`는 심볼별 단조 증가 값이다.

Boundary validation 실패는 엔진 실행 결과가 아니므로 Kafka execution report를 만들지 않는다.

## In-Memory Limits

매칭엔진은 in-memory order book을 사용한다.

- 프로세스 재시작 시 오더북 상태는 사라진다.
- duplicate live `order_id`는 오더북 불변식을 깨지 않도록 실행 결과에서 reject한다.
- 주문 조회, 계정, 잔고, 리스크 체크, client order id 관리는 매칭엔진 범위가 아니다.

## Quantity Policy

`LIMIT BUY + quote_qty`는 엔진 진입 전 주문 서비스가 base quantity로 변환한다.
매칭엔진 API에서 `quote_qty`는 `MARKET BUY`에만 허용한다.
