# Runtime Stress 테스트

관련 문서:

- [Runtime Stress 위키 홈](./README.md)
- [full-fill-same-level 측정 요약](./scenarios/full-fill-same-level/README.md)
- [market-quote-sweep 측정 요약](./scenarios/market-quote-sweep/README.md)
- [partial-fill-rest 측정 요약](./scenarios/partial-fill-rest/README.md)
- [place-resting-limit 측정 요약](./scenarios/place-resting-limit/README.md)
- [cancel-resting-order 측정 요약](./scenarios/cancel-resting-order/README.md)
- [amend-decrease-qty 측정 요약](./scenarios/amend-decrease-qty/README.md)
- [amend-price-change 측정 요약](./scenarios/amend-price-change/README.md)

`runtime_stress`는 `EngineRuntime` 경로에 부하를 걸어 런타임 병목을 확인하는 수동 실행 도구다.
Criterion 벤치마크처럼 안정적인 기준선을 만들기 위한 도구가 아니라, 채널 포화, 결과 발행 지연,
심볼 수 확장성, 체결 결과 payload 증가에 따른 병목을 찾기 위해 사용한다.

측정 경로는 아래와 같다.

```text
EngineDispatcher -> symbol별 engine thread -> result channel -> EngineResultHandler -> CountingPublisher
```

gRPC와 Kafka는 포함하지 않는다. Kafka 지연은 `--publisher-delay-ms` 옵션으로 흉내낸다.

## 테스트 모드

`runtime_stress`는 두 가지 모드를 지원한다.

| 모드 | 활성 조건 | 용도 |
| --- | --- | --- |
| paced load | `--duration-sec`와 `--target-commands-per-sec` 지정 | k6처럼 warm-up 후 일정 시간 동안 목표 command 유입률을 유지하는 부하 테스트 |
| burst stress | `--duration-sec` 미지정. `--orders` 사용 | command를 가능한 빨리 밀어 넣어 채널 포화점과 순간 burst 취약성을 확인하는 스트레스 테스트 |

운영 부하에 가까운 결과가 필요하면 paced load 모드를 먼저 사용한다.
기존 `--orders` 방식은 순간 burst를 만들기 때문에 `채널 포화`가 쉽게 발생할 수 있으며, 이를 일반 부하 테스트 결과로 해석하면 안 된다.

## 빌드

측정 전 release binary를 먼저 빌드한다.

```powershell
cargo build --release --bin runtime_stress
```

이후 측정은 `cargo run`보다 binary를 직접 실행하는 편이 좋다. `cargo run`은 Cargo 빌드/실행 로그가 함께 섞인다.
Windows는 `.exe` 파일을, macOS/Linux는 확장자 없는 binary를 실행한다.

```powershell
.\target\release\runtime_stress.exe --duration-sec 1 --target-commands-per-sec 100 --sweep-depth 3 --timeout-sec 5
```

```bash
./target/release/runtime_stress --duration-sec 1 --target-commands-per-sec 100 --sweep-depth 3 --timeout-sec 5
```

## 옵션

| 옵션 | 기본값 | 의미 |
| --- | ---: | --- |
| `--scenario` | `full-fill-same-level` | 실행할 부하 시나리오 |
| `--orders` | `100000` | burst stress 모드의 워크로드 반복 수 |
| `--duration-sec` | 없음 | paced load 모드의 측정 시간. 지정하면 `--orders`와 함께 사용할 수 없다 |
| `--warmup-sec` | `0` | paced load 모드에서 측정 전 warm-up 시간 |
| `--target-commands-per-sec` | 없음 | paced load 모드의 목표 command 유입률. `--duration-sec` 사용 시 필수 |
| `--symbols` | `1` | 생성할 심볼 수. 심볼별 engine thread가 생성된다 |
| `--sweep-depth` | `10` | 체결 시나리오에서 taker 1개가 sweep할 maker 주문 수 |
| `--publisher-delay-ms` | `0` | 결과 발행마다 추가할 지연 시간(ms) |
| `--timeout-sec` | `30` | 접수된 명령의 결과 발행 완료를 기다릴 최대 시간 |

안전한 수동 실행을 위해 입력값에는 상한이 있다. `--orders`는 10,000,000 이하, `--symbols`는 1,024 이하,
`--sweep-depth`는 10,000 이하, `--duration-sec`, `--warmup-sec`, `--timeout-sec`는 86,400초 이하,
`--publisher-delay-ms`는 60,000ms 이하, `--target-commands-per-sec`는 1,000,000 이하로 제한한다.

`--target-commands-per-sec`는 workload 반복 수가 아니라 실제 `EngineCommand` 유입률이다.
예를 들어 `full-fill-same-level`에서 `--sweep-depth 10`이면 workload 1회는 maker 10개와 taker 1개,
즉 command 11개를 만든다. paced load 모드는 이 11개를 한 번에 넣지 않고 command 단위로 간격을 둔다.

`--orders`는 command 수가 아니라 워크로드 반복 수다.
체결 시나리오에서는 한 번 반복할 때 maker 주문 `sweep-depth`개와 taker 주문 1개가 생성된다.

실수로 과도한 thread, 메모리, wall-clock 실행을 만들지 않도록 입력값은 아래 상한을 둔다.

| 옵션 | 최대값 |
| --- | ---: |
| `--orders` | `10,000,000` |
| `--symbols` | `1,024` |
| `--sweep-depth` | `10,000` |
| `--duration-sec`, `--warmup-sec`, `--timeout-sec` | `86,400` |
| `--publisher-delay-ms` | `60,000` |
| `--target-commands-per-sec` | `1,000,000` |

## 시나리오

| 시나리오 | 출력명 | 용도 |
| --- | --- | --- |
| `full-fill-same-level` | 동일 호가 전량 체결 | 같은 가격의 maker 주문 N개를 taker 하나가 전량 체결한다. 기본 시나리오다 |
| `market-quote-sweep` | 시장가 금액 스윕 | 여러 가격대의 ask를 Market Buy Quote 주문이 sweep한다 |
| `partial-fill-rest` | 부분 체결 후 잔존 | maker N개를 체결한 뒤 taker 잔량이 orderbook에 남는다. 반복 실행 중 잔존 주문이 누적되는 non-stationary 시나리오다 |
| `place-resting-limit` | 미체결 지정가 잔존 | 체결 없이 GTC limit 주문이 orderbook에 쌓인다. 반복 실행 중 잔존 주문이 누적되는 non-stationary 시나리오다 |
| `cancel-resting-order` | 잔존 주문 취소 | 같은 가격에 쌓은 GTC 주문을 다시 취소해 orderbook 삽입/삭제 비용을 확인한다. 반복 종료 시 book을 비우는 stationary 시나리오다 |
| `amend-decrease-qty` | 수량 감소 정정 | GTC 주문을 넣은 뒤 같은 가격에서 수량만 줄이고 다시 취소해 in-place 정정 비용을 확인한다. 반복 종료 시 book을 비우는 stationary 시나리오다 |
| `amend-price-change` | 가격 변경 정정 | GTC 주문을 넣은 뒤 가격을 바꾸고 다시 취소해 cancel-replace 정정 비용을 확인한다. 반복 종료 시 book을 비우는 stationary 시나리오다 |
| `cancel-missing` | 기본 경로 확인(미존재 취소) | 매칭 hot path가 아니라 dispatcher/result handler 기본 경로를 확인하는 보조 진단용이다 |

반복마다 orderbook이 비워지는 체결 hot path를 비교하려면 `full-fill-same-level`, `market-quote-sweep`를 우선 사용한다.
`partial-fill-rest`는 GTC taker 잔량이 남는 정상 동작을 포함하지만, 반복 실행 중 이전 잔존 주문이 다음 반복과 교차되어 book 상태가 drift한다.
`place-resting-limit`는 미체결 GTC 주문을 계속 추가하므로 orderbook 크기가 시간에 따라 증가한다.
따라서 두 시나리오는 stationary 기준선과 직접 비교하지 않고 잔존 주문 누적이 있는 부하 관측 시나리오로 해석한다.
취소/정정 병목을 보고 싶다면 `cancel-resting-order`, `amend-decrease-qty`, `amend-price-change`를 함께 비교한다.
`amend-decrease-qty`와 `amend-price-change`는 `--sweep-depth`를 사용하지 않고 workload 1회마다 Place, Amend, Cancel command를 만든다.
`cancel-missing`은 핵심 스트레스 시나리오가 아니다.

## 추천 실행 순서

1. 작은 입력으로 paced load 모드가 정상 동작하는지 확인한다.

```powershell
.\target\release\runtime_stress.exe --duration-sec 1 --target-commands-per-sec 100 --sweep-depth 3 --timeout-sec 5
```

```bash
./target/release/runtime_stress --duration-sec 1 --target-commands-per-sec 100 --sweep-depth 3 --timeout-sec 5
```

2. warm-up을 둔 기본 부하 테스트를 실행한다.

```powershell
.\target\release\runtime_stress.exe --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 5000 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 5000 --sweep-depth 10 --timeout-sec 30
```

3. 반복마다 orderbook이 비워지는 체결 시나리오를 같은 command 유입률에서 비교한다.

```powershell
.\target\release\runtime_stress.exe --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario market-quote-sweep --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario full-fill-same-level --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario market-quote-sweep --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
```

4. 잔존 주문이 누적되는 부하를 별도로 관측한다.

```powershell
.\target\release\runtime_stress.exe --scenario partial-fill-rest --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario place-resting-limit --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario partial-fill-rest --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario place-resting-limit --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
```

이 결과는 잔존 주문이 누적되는 non-stationary workload의 관측값이다. `full-fill-same-level`, `market-quote-sweep`의 안정 TPS와 직접 비교하지 않는다.

5. 성공 취소/정정 경로의 병목을 비교한다.

```powershell
.\target\release\runtime_stress.exe --scenario cancel-resting-order --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario amend-decrease-qty --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --timeout-sec 30
.\target\release\runtime_stress.exe --scenario amend-price-change --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario cancel-resting-order --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --scenario amend-decrease-qty --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --timeout-sec 30
./target/release/runtime_stress --scenario amend-price-change --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --timeout-sec 30
```

6. 심볼 수를 늘려 확장성을 확인한다.

```powershell
.\target\release\runtime_stress.exe --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 1 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 2 --sweep-depth 10 --timeout-sec 30
.\target\release\runtime_stress.exe --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 4 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 1 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 2 --sweep-depth 10 --timeout-sec 30
./target/release/runtime_stress --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --symbols 4 --sweep-depth 10 --timeout-sec 30
```

7. publisher 지연을 넣어 결과 발행 병목을 확인한다.

```powershell
.\target\release\runtime_stress.exe --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --publisher-delay-ms 1 --timeout-sec 30
.\target\release\runtime_stress.exe --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --publisher-delay-ms 5 --timeout-sec 30
```

```bash
./target/release/runtime_stress --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --publisher-delay-ms 1 --timeout-sec 30
./target/release/runtime_stress --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 1000 --sweep-depth 10 --publisher-delay-ms 5 --timeout-sec 30
```

8. 순간 burst 한계를 보고 싶을 때만 기존 `--orders` 모드를 사용한다.

```powershell
.\target\release\runtime_stress.exe --orders 1000 --sweep-depth 10 --timeout-sec 30
```

```bash
./target/release/runtime_stress --orders 1000 --sweep-depth 10 --timeout-sec 30
```

이 명령은 11,000개 command를 가능한 빨리 단일 심볼 큐에 넣는다. `채널 포화`가 높게 나오는 것은 burst stress 결과로 해석한다.

## 결과 해석

| 출력 항목 | 해석 |
| --- | --- |
| `목표 명령 수/s` | paced load 모드에서 의도한 `EngineCommand` 유입률 |
| `실제 시도 수/s` | 측정 구간에서 실제로 dispatch를 시도한 command/sec |
| `접수 성공` | dispatcher가 engine channel에 넣는 데 성공한 명령 수 |
| `채널 포화` | engine command channel이 가득 차서 거부된 명령 수 |
| `발행 결과` | result handler가 publisher까지 전달한 결과 수 |
| `완료 여부` | 제한 시간 안에 접수된 모든 명령의 결과가 발행됐는지 여부 |
| `체결 수` | 발행된 `Place` 결과에 포함된 trade 수 합계 |
| `메이커 갱신 수` | 발행된 `Place` 결과에 포함된 maker snapshot 수 합계 |
| `pacing 지연 횟수` | 목표 command 간격보다 dispatch 루프가 늦어진 횟수 |
| `초당 접수 성공 수` | dispatch loop 기준 처리량 |
| `초당 발행 결과 수` | result handler와 publisher까지 포함한 처리량 |

주요 해석 기준은 아래와 같다.

- paced load 모드에서 `채널 포화`가 0이면 목표 유입률은 현재 런타임이 수용 가능한 범위다.
- paced load 모드에서 `채널 포화`가 증가하면 목표 command/sec가 engine thread 소비 속도보다 높을 가능성이 있다.
- `접수 성공`은 높은데 `발행 결과`가 늦거나 `완료 여부`가 `아니오`면 result handler 또는 publisher 경로가 병목일 수 있다.
- `--symbols`를 늘려도 `초당 발행 결과 수`가 거의 늘지 않으면 단일 result handler나 공통 result channel을 의심한다.
- `--publisher-delay-ms`를 조금만 올려도 전체 완료가 밀리면 실제 Kafka publish 지연에 취약할 가능성이 있다.
- `체결 수`와 `메이커 갱신 수`가 큰 시나리오에서만 급격히 느려지면 trade/result payload 생성 비용을 본다.
- burst stress 모드에서 높은 `채널 포화`는 순간 유입량 한계를 의미한다. 이를 운영 부하 처리량 부족으로 바로 해석하지 않는다.

## 보조 진단

런타임 처리 경로의 순수 처리량만 확인하고 싶을 때는 `cancel-missing`을 사용한다.

```powershell
.\target\release\runtime_stress.exe --scenario cancel-missing --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 10000 --timeout-sec 30
```

```bash
./target/release/runtime_stress --scenario cancel-missing --warmup-sec 10 --duration-sec 30 --target-commands-per-sec 10000 --timeout-sec 30
```

이 시나리오는 존재하지 않는 주문 취소를 반복하므로 실제 체결, maker 갱신, orderbook sweep 비용을 만들지 않는다.
따라서 매칭엔진 hot path 스트레스 결과로 해석하면 안 된다.
