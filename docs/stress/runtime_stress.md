# Runtime Stress 테스트

`runtime_stress`는 `EngineRuntime` 경로에 부하를 걸어 런타임 병목을 확인하는 수동 실행 도구다.
Criterion 벤치마크처럼 안정적인 기준선을 만들기 위한 도구가 아니라, 채널 포화, 결과 발행 지연,
심볼 수 확장성, 체결 결과 payload 증가에 따른 병목을 찾기 위해 사용한다.

측정 경로는 아래와 같다.

```text
EngineDispatcher -> symbol별 engine thread -> result channel -> EngineResultHandler -> CountingPublisher
```

gRPC와 Kafka는 포함하지 않는다. Kafka 지연은 `--publisher-delay-ms` 옵션으로 흉내낸다.

## 빌드

측정 전 release binary를 먼저 빌드한다.

```powershell
cargo build --release --bin runtime_stress
```

이후 측정은 `cargo run`보다 binary를 직접 실행하는 편이 좋다. `cargo run`은 Cargo 빌드/실행 로그가 함께 섞인다.
Windows는 `.exe` 파일을, macOS/Linux는 확장자 없는 binary를 실행한다.

```powershell
.\target\release\runtime_stress.exe --orders 10 --sweep-depth 3 --timeout-sec 5
```

```bash
./target/release/runtime_stress --orders 10 --sweep-depth 3 --timeout-sec 5
```

## 옵션

| 옵션 | 기본값 | 의미 |
| --- | ---: | --- |
| `--scenario` | `full-fill-same-level` | 실행할 부하 시나리오 |
| `--orders` | `100000` | 워크로드 반복 수 |
| `--symbols` | `1` | 생성할 심볼 수. 심볼별 engine thread가 생성된다 |
| `--sweep-depth` | `10` | 체결 시나리오에서 taker 1개가 sweep할 maker 주문 수 |
| `--publisher-delay-ms` | `0` | 결과 발행마다 추가할 지연 시간(ms) |
| `--timeout-sec` | `30` | 접수된 명령의 결과 발행 완료를 기다릴 최대 시간 |

`--orders`는 command 수가 아니라 워크로드 반복 수다.
체결 시나리오에서는 한 번 반복할 때 maker 주문 `sweep-depth`개와 taker 주문 1개가 생성된다.

예를 들어 아래 명령은 반복 1,000회, sweep depth 10으로 실행한다.

```powershell
.\target\release\runtime_stress.exe --orders 1000 --sweep-depth 10
```

```bash
./target/release/runtime_stress --orders 1000 --sweep-depth 10
```

이 경우 시도한 명령 수는 대략 `1000 * (10 + 1) = 11000`개이고,
체결 수와 메이커 갱신 수는 정상 완료 기준 각각 `1000 * 10 = 10000`개가 된다.

## 시나리오

| 시나리오 | 출력명 | 용도 |
| --- | --- | --- |
| `full-fill-same-level` | 동일 호가 전량 체결 | 같은 가격의 maker 주문 N개를 taker 하나가 전량 체결한다. 기본 시나리오다 |
| `market-quote-sweep` | 시장가 금액 스윕 | 여러 가격대의 ask를 Market Buy Quote 주문이 sweep한다 |
| `partial-fill-rest` | 부분 체결 후 잔존 | maker N개를 체결한 뒤 taker 잔량이 orderbook에 남는다 |
| `place-resting-limit` | 미체결 지정가 잔존 | 체결 없이 GTC limit 주문이 orderbook에 쌓인다 |
| `cancel-missing` | 기본 경로 확인(미존재 취소) | 매칭 hot path가 아니라 dispatcher/result handler 기본 경로를 확인하는 보조 진단용이다 |

매칭엔진 부하를 보고 싶다면 `full-fill-same-level`, `market-quote-sweep`, `partial-fill-rest`를 우선 사용한다.
`cancel-missing`은 핵심 스트레스 시나리오가 아니다.

## 추천 실행 순서

1. 작은 입력으로 정상 동작을 확인한다.

```powershell
.\target\release\runtime_stress.exe --orders 10 --sweep-depth 3 --timeout-sec 5
```

```bash
./target/release/runtime_stress --orders 10 --sweep-depth 3 --timeout-sec 5
```

2. 기본 체결 부하를 점진적으로 올린다.

```powershell
.\target\release\runtime_stress.exe --orders 1000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --orders 10000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --orders 100000 --sweep-depth 10 --timeout-sec 60
```

```bash
./target/release/runtime_stress --orders 1000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --orders 10000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --orders 100000 --sweep-depth 10 --timeout-sec 60
```

3. 체결 시나리오별로 비교한다.

```powershell
.\target\release\runtime_stress.exe --scenario full-fill-same-level --orders 10000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario market-quote-sweep --orders 10000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario partial-fill-rest --orders 10000 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario full-fill-same-level --orders 10000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario market-quote-sweep --orders 10000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario partial-fill-rest --orders 10000 --sweep-depth 10 --timeout-sec 30
```

4. 심볼 수를 늘려 확장성을 확인한다.

```powershell
.\target\release\runtime_stress.exe --orders 10000 --symbols 1 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --orders 10000 --symbols 2 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --orders 10000 --symbols 4 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --orders 10000 --symbols 1 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --orders 10000 --symbols 2 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --orders 10000 --symbols 4 --sweep-depth 10 --timeout-sec 30
```

5. publisher 지연을 넣어 결과 발행 병목을 확인한다.

```powershell
.\target\release\runtime_stress.exe --orders 1000 --sweep-depth 10 --publisher-delay-ms 1 --timeout-sec 30
.\target\release\runtime_stress.exe --orders 1000 --sweep-depth 10 --publisher-delay-ms 5 --timeout-sec 30
```

```bash
./target/release/runtime_stress --orders 1000 --sweep-depth 10 --publisher-delay-ms 1 --timeout-sec 30
./target/release/runtime_stress --orders 1000 --sweep-depth 10 --publisher-delay-ms 5 --timeout-sec 30
```

## 결과 해석

| 출력 항목 | 해석 |
| --- | --- |
| `접수 성공` | dispatcher가 engine channel에 넣는 데 성공한 명령 수 |
| `채널 포화` | engine command channel이 가득 차서 거부된 명령 수 |
| `발행 결과` | result handler가 publisher까지 전달한 결과 수 |
| `완료 여부` | 제한 시간 안에 접수된 모든 명령의 결과가 발행됐는지 여부 |
| `체결 수` | 발행된 `Place` 결과에 포함된 trade 수 합계 |
| `메이커 갱신 수` | 발행된 `Place` 결과에 포함된 maker snapshot 수 합계 |
| `초당 접수 성공 수` | dispatch loop 기준 처리량 |
| `초당 발행 결과 수` | result handler와 publisher까지 포함한 처리량 |

주요 해석 기준은 아래와 같다.

- `채널 포화`가 증가하면 dispatcher가 engine thread 소비 속도보다 빠르게 명령을 밀어 넣고 있다.
- `접수 성공`은 높은데 `발행 결과`가 늦거나 `완료 여부`가 `아니오`면 result handler 또는 publisher 경로가 병목일 수 있다.
- `--symbols`를 늘려도 `초당 발행 결과 수`가 거의 늘지 않으면 단일 result handler나 공통 result channel을 의심한다.
- `--publisher-delay-ms`를 조금만 올려도 전체 완료가 밀리면 실제 Kafka publish 지연에 취약할 가능성이 있다.
- `체결 수`와 `메이커 갱신 수`가 큰 시나리오에서만 급격히 느려지면 trade/result payload 생성 비용을 본다.

## 보조 진단

런타임 처리 경로의 순수 처리량만 확인하고 싶을 때는 `cancel-missing`을 사용한다.

```powershell
.\target\release\runtime_stress.exe --scenario cancel-missing --orders 100000 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario cancel-missing --orders 100000 --timeout-sec 30
```

이 시나리오는 존재하지 않는 주문 취소를 반복하므로 실제 체결, maker 갱신, orderbook sweep 비용을 만들지 않는다.
따라서 매칭엔진 hot path 스트레스 결과로 해석하면 안 된다.
