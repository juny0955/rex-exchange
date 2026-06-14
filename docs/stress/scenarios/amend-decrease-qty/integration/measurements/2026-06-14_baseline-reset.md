# amend-decrease-qty Integration Baseline (2026-06-14, 오더북 초기화)

관련 문서:

- [amend-decrease-qty Integration Stress](../README.md)
- [2026-06-13 baseline (오더북 오염 상태)](./2026-06-13_baseline.md)
- [raw log](../raw/2026-06-14_baseline-reset.log)
- [오더북 초기화 절차](../../../../integration_stress.md)

## 측정 환경

| 항목 | 값 |
| --- | --- |
| 실행 환경 | 로컬 Docker `integration-stress` → `matching-engine` (gRPC) → `kafka` |
| 로컬 호스트 | MacBook Pro Mac16,8, Apple M4 Pro 12-core CPU(8P+4E), memory 24GB |
| Host OS | macOS 26.3.1(a), build 25D771280a |
| Docker Desktop | Docker 27.3.1, Linux arm64/aarch64, cgroup v2, overlay2 |
| Docker VM 할당 | 8 CPUs, 4,109,737,984 bytes memory |
| SUT 리소스 제한 | CPU 1, memory 1GB, swap 1GB |
| Kafka 리소스 제한 | CPU 2, memory 2GB, swap 2GB |
| 부하 생성기 리소스 제한 | 없음 |
| 시나리오 | `amend-decrease-qty` (commands/unit = 3) |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| 동시 실행(units) | `512` |
| gRPC 연결 수 | `16` |
| 전송 방식 | `SubmitBatch` (unit 전체를 batch RPC 1콜로 전송) |
| 오더북 초기화 | 매 target rate 실행 전 `docker compose restart matching-engine` 후 healthy 대기 (오더북 완전 초기화) |
| warm-up 시간 | `5s` |
| 측정 시간 | `30s` |
| settle 제한 시간 | `15s` |

대표 실행 명령:

```bash
docker compose restart matching-engine
# matching-engine이 healthy 상태가 될 때까지 대기 후 실행
docker compose run --rm integration-stress --grpc-endpoint http://matching-engine:50051 --kafka-brokers kafka:9092 --scenario amend-decrease-qty --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 512 --connections 16 --settle-timeout-sec 15
```

## 결과 요약

| target/s | 판정 | 접수 성공률 | 채널 포화(503) | 503 비율 | target 달성 | 초당 접수 성공 | ack p50 | ack p99 | E2E p50 | E2E p99 | 무손실 |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 50,000 | 안전 | 100.00% | 0 | 0% | 예 (100.00%) | 49,994.9/s | 368µs | 1.785ms | 5.583ms | 10.769ms | 통과 |
| 75,000 | 경계 | 99.98% | 502 | 0.022% | 예 (100.00%) | 74,980.7/s | 502µs | 6.527ms | 5.574ms | 28.024ms | 통과 |
| 100,000 | 포화 | 99.04% | 26,200 | 0.96% | 아니오 (90.78%) | 89,742.3/s | 2.127ms | 73.771ms | 12.076ms | 82.418ms | 실패* |

\* 이 실행 중 Kafka 브로커가 디스크 고갈로 인한 장애(`BrokerTransportFailure`, 직후 컨테이너 크래시)를 겪고 있었다. gRPC 인그레스 측 503/target 달성률 수치는 엔진 포화를 가리키지만, Kafka 수신 측 `누락 2,695,973`/`무손실 판정=실패`는 Kafka 브로커 장애가 주된 원인이며 엔진 자체의 정합성 문제는 아니다. 자세한 내용은 "판정" 절 참고.

## 판정

- **안전 TPS = 50,000** (2026-06-13: 20,000 → **2.5배 상승**). `채널 포화 0건`, `target 달성률 100%`이며 ack p99 `1.785ms`, E2E p99 `10.769ms`로 안정적이다.
- **경계 TPS = 75,000** (2026-06-13: 25,000 → **3배 상승**). `target 달성률 100%`는 유지하지만 `채널 포화 0.022%`가 발생하고 ack p99가 `1.785ms → 6.527ms`(약 3.7배), E2E p99가 `10.769ms → 28.024ms`(약 2.6배)로 상승한다.
- **포화 TPS = 100,000** (2026-06-13: 30,000 → **약 3.3배 상승**). `채널 포화 0.96%`, `target 달성률 90.78%`로 엔진 채널 백프레셔가 시작된다. ack p99 `73.771ms`, E2E p99 `82.418ms`로 급증한다. 다만 이 실행 시점에는 Kafka 컨테이너가 디스크 고갈로 인해 `BrokerTransportFailure`를 반복 출력하며 곧 크래시할 정도로 불안정한 상태였고, 이로 인해 Kafka 수신 이벤트가 1,215건만 기록되어 `누락 2,695,973`, `무손실 판정 실패`로 나타났다. 즉 **무손실 판정 실패는 Kafka 브로커 장애에 의한 것**이며, gRPC 인그레스 지표(503=26,200건/0.96%, target 달성률 90.78%, ack p99 급증)만으로도 100,000이 포화 구간임은 충분히 확인된다.
- **오더북 초기화 효과**: 2026-06-13 baseline은 `place-resting-limit`이 가격 `10,000~10,049`에 남긴 잔존 주문 위에서 측정되어 안전/경계/포화 지점이 20,000/25,000/30,000으로 낮게 나왔다. 매 실행 전 `docker compose restart matching-engine`으로 오더북을 비운 결과, 안전/경계/포화가 모두 2.5~3.3배 수준으로 상승했다. `amend-decrease-qty`는 가격 `10,000`의 in-place 수량 갱신이라, 같은 가격 레벨에 잔존 주문이 쌓여 있을수록 amend 대상 주문을 찾는 비용이 커진 것으로 보인다([오더북 초기화 절차](../../../../integration_stress.md) 참고).
