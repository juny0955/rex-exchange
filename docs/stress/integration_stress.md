# Integration Stress 테스트

관련 문서:

- [Stress 테스트 홈](./README.md)
- [Runtime Stress 테스트](./runtime_stress.md)
- [Stress 시나리오](./scenarios/README.md)

`integration_stress`는 gRPC 인그레스부터 Kafka 발행까지 **실제 종단(E2E) 경로**에 부하를 거는 수동 실행 도구다.
`runtime_stress`가 `EngineRuntime`만 인프로세스로 측정하고 gRPC와 Kafka를 우회하는 것과 달리,
이 도구는 네트워크 직렬화, tonic HTTP/2, Kafka producer in-flight, 브로커 왕복, consumer 수신까지 포함한 실측을 만든다.

측정 경로는 아래와 같다.

```text
gRPC client -> :50051 tonic server -> EngineDispatcher -> MatchingEngine -> KafkaProducer -> Kafka broker -> Consumer
```

측정하는 4가지 지표는 다음과 같다.

| 지표 | 설명 |
| --- | --- |
| E2E 지연 | gRPC 송신 시각부터 해당 order_id의 Kafka 이벤트 수신 시각까지. order_id로 상관(correlate)해 백분위로 집계 |
| 처리량 / 포화점 | gRPC→엔진→Kafka 전체 경로가 견디는 지속 가능 초당 명령 수와 한계점 |
| 무손실 / 정합성 | 접수(SUBMITTED)된 명령이 Kafka에 정확히 1건씩 도착하는지(누락·중복) |
| 백프레셔 | 고부하 시 gRPC 503(RESOURCE_EXHAUSTED = 엔진 채널 포화) 발생 비율 |

`runtime_stress`는 매칭엔진 내부 처리량 한계를 빠르게 보는 데 쓰고, `integration_stress`는 운영에 가까운 종단 지연과 무손실을 검증하는 데 쓴다.
측정 결과와 기준선은 [Stress 시나리오](./scenarios/README.md) 아래에 시나리오별로 기록한다.

## 사전 준비

이 도구는 외부 의존(Kafka 브로커, 실행 중인 gRPC 서버)이 필요하다. 부하를 걸기 전에 둘 다 띄워야 한다.

1. Kafka 브로커를 띄운다.

```bash
docker compose up -d
```

2. SUT(System Under Test)인 매칭엔진 gRPC 서버를 띄운다. 부하 생성기와 자원이 섞이지 않게 별도 터미널에서 실행한다.

```bash
cargo run --release --bin matching-engine
```

`integration_stress`는 시작 시 대상 토픽(`matching-engine-events`)을 선생성한다(이미 있으면 무시).
컨슈머는 매 실행마다 고유 `group.id`를 쓰고 `auto.offset.reset=latest`로 시작하므로,
토픽에 남아 있는 이전 실행 이벤트는 읽지 않고 이번 실행에서 새로 발행된 이벤트만 측정한다.

## 테스트 모드

`runtime_stress`와 동일하게 두 가지 모드를 지원한다.

| 모드 | 활성 조건 | 용도 |
| --- | --- | --- |
| paced load | `--duration-sec`와 `--target-commands-per-sec` 지정 | 일정 시간 동안 목표 command 유입률을 유지하며 종단 지연·포화점을 본다 |
| burst stress | `--duration-sec` 미지정. `--orders` 사용 | command를 가능한 빨리 밀어 넣어 순간 burst와 채널 포화(503), 무손실을 본다 |

종단 지연·포화점을 보려면 paced load 모드를, 무손실·순간 한계를 보려면 burst 모드를 사용한다.

## 빌드

측정 전 release binary를 먼저 빌드한다.

```powershell
cargo build --release --bin integration_stress
```

이후 측정은 `cargo run`보다 binary를 직접 실행하는 편이 좋다. `cargo run`은 Cargo 빌드/실행 로그가 함께 섞인다.
Windows는 `.exe` 파일을, macOS/Linux는 확장자 없는 binary를 실행한다.

```powershell
.\target\release\integration_stress.exe --orders 100 --scenario place-resting-limit --concurrency 16
```

```bash
./target/release/integration_stress --orders 100 --scenario place-resting-limit --concurrency 16
```

## 옵션

| 옵션 | 기본값 | 의미 |
| --- | ---: | --- |
| `--scenario` | `full-fill-same-level` | 실행할 부하 시나리오 |
| `--orders` | `100000` | burst stress 모드의 워크로드 반복 수 |
| `--duration-sec` | 없음 | paced load 모드의 측정 시간. 지정하면 `--orders`와 함께 사용할 수 없다 |
| `--warmup-sec` | `0` | paced load 모드에서 측정 전 warm-up 시간. warm-up 구간 결과는 측정 전 초기화된다 |
| `--target-commands-per-sec` | 없음 | paced load 모드의 목표 command 유입률. `--duration-sec` 사용 시 필수 |
| `--symbols` | `1` | 생성할 심볼 수. 심볼별 engine thread가 생성된다 |
| `--sweep-depth` | `10` | 체결 시나리오에서 taker 1개가 sweep할 maker 주문 수 |
| `--concurrency` | `64` | 동시에 실행하는 워크로드 unit 수(동시 in-flight 정도) |
| `--timeout-sec` | `30` | (예약) 명령 처리 제한 시간 |
| `--settle-timeout-sec` | `15` | 부하 종료 후 잔여 Kafka 이벤트 수신을 기다릴 최대 시간 |
| `--grpc-endpoint` | `http://localhost:50051` | SUT gRPC 엔드포인트 |
| `--kafka-brokers` | `localhost:9092` | Kafka bootstrap 브로커 |
| `--kafka-topic` | `matching-engine-events` | 구독할 결과 이벤트 토픽 |

안전한 수동 실행을 위해 입력값에는 상한이 있다. `--orders`는 10,000,000 이하, `--symbols`는 1,024 이하,
`--sweep-depth`는 10,000 이하, `--concurrency`는 8,192 이하,
`--duration-sec`, `--warmup-sec`, `--timeout-sec`, `--settle-timeout-sec`는 86,400초 이하,
`--target-commands-per-sec`는 1,000,000 이하로 제한한다.

`--target-commands-per-sec`는 워크로드 반복 수가 아니라 실제 `EngineCommand` 유입률이다.
예를 들어 `full-fill-same-level`에서 `--sweep-depth 10`이면 워크로드 1회는 maker 10개와 taker 1개,
즉 command 11개를 만든다.

`--orders`는 command 수가 아니라 워크로드 반복(unit) 수다.

## 동시성과 순서 보존

`integration_stress`는 워크로드 **unit** 단위로 비동기 태스크를 띄운다. unit 하나는 한 번의 워크로드 반복이 만든
command 묶음이다(예: `cancel-resting-order`의 place N개 + cancel N개).

- unit **내부** command는 직렬로 전송한다. place를 보낸 뒤 응답을 받고 cancel/amend를 보내므로,
  같은 심볼 큐에 place가 먼저 들어가 의존성 순서(place → cancel/amend)가 보존된다.
- unit **간**에는 `--concurrency`만큼 병렬로 실행한다.

따라서 단일 심볼이라도 unit 간 병렬성으로 처리량을 올릴 수 있고, 같은 order_id가 재사용되는 시나리오에서도
이벤트가 명령 순서대로 발행된다. 상관 집계는 order_id별 FIFO로 송신↔수신을 매칭한다.

## 시나리오

워크로드 생성기는 `runtime_stress`와 동일하다.

| 시나리오 | 출력명 | 용도 |
| --- | --- | --- |
| `full-fill-same-level` | 동일 호가 전량 체결 | 같은 가격의 maker 주문 N개를 taker 하나가 전량 체결한다. 기본 시나리오다 |
| `market-quote-sweep` | 시장가 금액 스윕 | 여러 가격대의 ask를 Market Buy Quote 주문이 sweep한다 |
| `partial-fill-rest` | 부분 체결 후 잔존 | maker N개를 체결한 뒤 taker 잔량이 orderbook에 남는다 |
| `place-resting-limit` | 미체결 지정가 잔존 | 체결 없이 GTC limit 주문이 orderbook에 쌓인다. Buy 주문만 생성하므로 스모크용으로 적합하다 |
| `cancel-resting-order` | 잔존 주문 취소 | 같은 가격에 쌓은 GTC 주문을 다시 취소한다. unit당 place N개 + cancel N개 |
| `amend-decrease-qty` | 수량 감소 정정 | GTC 주문을 넣고 수량만 줄인 뒤 취소한다. unit당 Place, Amend, Cancel |
| `amend-price-change` | 가격 변경 정정 | GTC 주문을 넣고 가격을 바꾼 뒤 취소한다. unit당 Place, Amend, Cancel |
| `cancel-missing` | 기본 경로 확인(미존재 취소) | 존재하지 않는 주문을 취소한다. 접수는 성공하고 Kafka에는 rejected outcome 이벤트가 발행된다 |

> gRPC 응답은 접수에 성공하면 모든 시나리오에서 `SUBMITTED`를 반환한다. 비즈니스 거부(예: `cancel-missing`)는
> gRPC 상태가 아니라 Kafka 이벤트의 `outcome=rejected`로 나타난다. 따라서 **접수(SUBMITTED) 1건 = Kafka 이벤트 1건**이
> 모든 시나리오에서 성립하며, 무손실 판정은 이 관계를 기준으로 한다.

## 추천 실행 순서

1. 소량 스모크로 파이프라인과 무손실을 확인한다. `place-resting-limit`은 Buy 주문만 만들어 검증이 단순하다.

```powershell
.\target\release\integration_stress.exe --orders 100 --scenario place-resting-limit --concurrency 16
```

```bash
./target/release/integration_stress --orders 100 --scenario place-resting-limit --concurrency 16
```

`누락 = 0`, `중복 수신 = 0`, `무손실 판정 = 통과`, `settle 완료 = 예`를 확인한다.

2. 저부하에서 E2E 지연 기준선을 잡는다(503이 0인 구간).

```powershell
.\target\release\integration_stress.exe --duration-sec 30 --target-commands-per-sec 1000 --scenario full-fill-same-level --concurrency 64
```

```bash
./target/release/integration_stress --duration-sec 30 --target-commands-per-sec 1000 --scenario full-fill-same-level --concurrency 64
```

3. 목표 유입률을 단계적으로 올려 포화점을 찾는다.

```powershell
.\target\release\integration_stress.exe --duration-sec 30 --warmup-sec 5 --target-commands-per-sec 10000 --scenario full-fill-same-level --concurrency 128
.\target\release\integration_stress.exe --duration-sec 30 --warmup-sec 5 --target-commands-per-sec 50000 --scenario full-fill-same-level --concurrency 128
```

```bash
./target/release/integration_stress --duration-sec 30 --warmup-sec 5 --target-commands-per-sec 10000 --scenario full-fill-same-level --concurrency 128
./target/release/integration_stress --duration-sec 30 --warmup-sec 5 --target-commands-per-sec 50000 --scenario full-fill-same-level --concurrency 128
```

다음 신호가 처음 나타나는 지점을 포화점으로 본다.

- `초당 종단 수신 수(E2E)`가 더 이상 늘지 않고 평탄해진다.
- `채널 포화(503)`가 0에서 증가하기 시작한다.
- `E2E 지연 p99`가 급격히 커진다.

4. 순간 burst 한계를 보고 싶을 때만 `--orders` 모드를 사용한다.

```powershell
.\target\release\integration_stress.exe --orders 2000 --scenario full-fill-same-level --concurrency 128
```

```bash
./target/release/integration_stress --orders 2000 --scenario full-fill-same-level --concurrency 128
```

## 결과 해석

출력은 `[gRPC 접수]`, `[Kafka 수신 / 정합성]`, `[E2E 지연]`, `[처리량]` 네 섹션으로 나뉜다.

| 출력 항목 | 해석 |
| --- | --- |
| `접수 성공(SUBMITTED)` | gRPC가 접수에 성공한 명령 수. 각 1건이 Kafka 이벤트 1건을 만든다 |
| `채널 포화(503)` | 엔진 command channel이 가득 차 gRPC가 `RESOURCE_EXHAUSTED`로 거부한 수(백프레셔) |
| `거부(REJECTED)` | gRPC가 SUBMITTED 외 상태를 반환한 수(현재 서버 경로상 거의 없음) |
| `기타 에러` | 연결/타임아웃/`invalid_argument` 등 그 밖의 gRPC 에러 수 |
| `수신 이벤트(중복 포함)` | Kafka에서 받은 총 메시지 수 |
| `고유 이벤트` | 중복을 제외한 event_id 수 |
| `상관 매칭` | 송신과 매칭되어 지연이 측정된 수 |
| `누락(접수했으나 미수신)` | 접수됐지만 settle 시간 안에 이벤트가 오지 않은 수 |
| `중복 수신` | 동일 event_id가 두 번 이상 온 수 |
| `미상관 수신` | 대응 송신을 찾지 못한 수신 수. `latest` 오프셋에서는 0이어야 한다 |
| `settle 완료` | 부하 종료 후 제한 시간 안에 접수 수만큼 수신했는지 여부 |
| `무손실 판정` | settle 완료 + 누락·중복·미상관 0 + 고유 이벤트 = 접수 성공일 때 통과 |
| `E2E 지연 p50/p99/max` | gRPC 송신부터 Kafka 수신까지 종단 지연 분포 |
| `초당 접수 성공 수` | 접수(dispatch) 구간 기준 처리량 |
| `초당 종단 수신 수(E2E)` | 접수+settle 전체 구간 기준 종단 처리량 |

주요 해석 기준은 아래와 같다.

- `무손실 판정 = 통과`이고 `채널 포화 = 0`이면 현재 유입률은 종단 경로가 안전하게 수용하는 범위다.
- `채널 포화(503)`가 증가하면 gRPC가 받아낸 속도를 엔진이 소비하지 못하는 백프레셔 신호다.
- `누락`이 발생하거나 `settle 완료 = 아니오`면 포화 부근에서 발행이 밀리거나 손실이 있는지 확인한다.
- `E2E 지연 p99`가 부하와 무관하게 일정하게 높으면 Kafka publish 경로(producer 배칭/poll cadence, broker)를 의심한다.
- 절대 수치보다 유입률 단계별 추세와 포화점 위치로 해석한다(아래 주의 참고).

## 주의

- **로컬 자원 경쟁**: 부하 생성기, SUT, Kafka를 한 머신에서 돌리면 CPU를 공유한다. 절대 지연·처리량 수치보다
  유입률 단계별 *상대 추세*와 포화점 해석에 무게를 둔다. 정밀 측정이 필요하면 부하 생성기를 별도 머신으로 분리한다.
- **컨슈머 지연 편향**: 컨슈머 폴링이 느리면 E2E 지연에 컨슈머측 지연이 섞인다. 충분한 `--settle-timeout-sec`를 둔다.
- **시계**: 송신·수신 시각 모두 동일 프로세스의 단조 시계(`Instant`)를 쓰므로 상관 지연은 음수가 되지 않는다.
