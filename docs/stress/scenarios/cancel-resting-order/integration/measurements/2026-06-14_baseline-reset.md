# cancel-resting-order Integration Baseline (2026-06-14, 오더북 초기화)

관련 문서:

- [cancel-resting-order Integration Stress](../README.md)
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
| 시나리오 | `cancel-resting-order` (commands/unit = 20) |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| 동시 실행(units) | `512` |
| gRPC 연결 수 | `16` |
| 전송 방식 | `SubmitBatch` (unit 전체를 batch RPC 1콜로 전송, commands/unit=20으로 가장 큰 배치) |
| 오더북 초기화 | 매 target rate 실행 전 `docker compose restart matching-engine` 후 healthy 대기 (오더북 완전 초기화) |
| warm-up 시간 | `5s` |
| 측정 시간 | `30s` |
| settle 제한 시간 | `15s` |

대표 실행 명령:

```bash
docker compose restart matching-engine
# matching-engine이 healthy 상태가 될 때까지 대기 후 실행
docker compose run --rm integration-stress --grpc-endpoint http://matching-engine:50051 --kafka-brokers kafka:9092 --scenario cancel-resting-order --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 512 --connections 16 --settle-timeout-sec 15
```

## 결과 요약

| target/s | 판정 | 접수 성공률 | 채널 포화(503) | 503 비율 | target 달성 | 초당 접수 성공 | ack p50 | ack p99 | E2E p50 | E2E p99 | 무손실 |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 100,000 | 안전 | 100.00% | 0 | 0% | 예 (99.99%) | 99,985.9/s | 308µs | 997µs | 5.823ms | 10.217ms | 통과 |
| 150,000 | 경계 | 100.00% | 36 | 0.0008% | 예 (99.99%) | 149,977.4/s | 331µs | 980µs | 6.206ms | 37.352ms | 통과 |
| 200,000 | 포화 | 11.15% | 4,329,213 | 88.85% | 아니오 (81.21%) | 18,030.3/s | 73.791ms | 105.562ms | 101.645ms | 179.788ms | 실패 |

## 판정

- **안전 TPS = 100,000** (2026-06-13: 10,000 → **10배 상승**). `채널 포화 0건`, `target 달성률 99.99%`이며 ack p99 `997µs`, E2E p99 `10.217ms`로 매우 안정적이다.
- **경계 TPS = 150,000** (2026-06-13: 15,000 → **10배 상승**). `채널 포화 36건(0.0008%)`으로 503 자체는 거의 없지만, E2E p99가 `10.217ms → 37.352ms`(약 3.7배)로 급증한다(post-ack p99도 `9.730ms → 37.005ms`). ack p99는 `997µs → 980µs`로 거의 변화가 없어, 경계가 gRPC ack 단계가 아니라 **Kafka 적재/수신 경로**에서 먼저 나타난다.
- **포화 TPS = 200,000** (2026-06-13: 30,000 → **약 6.7배 상승**). `채널 포화 88.85%`, `접수 성공률 11.15%`로 시도한 명령의 90% 가까이 거부된다. `target 달성률 81.21%`로도 미달이며, settle 15초 내에 정착하지 못해 `settle 완료=아니오`, `무손실 판정=실패`(누락 163,958)다. 초당 접수 성공 수는 18,030.3/s(E2E 수신 8,402.0/s)로 150,000 target(149,977.4/s)보다 크게 낮아지는 역전 현상이 뚜렷하다.
- **오더북 초기화 효과**: 2026-06-13 baseline은 `place-resting-limit`이 가격 `10,000~10,049`에 남긴 잔존 주문(및 `cancel-resting-order` 자신의 반복 실행 잔존분) 위에서 측정되어 안전/경계/포화 지점이 10,000/15,000/30,000으로 비정상적으로 낮았다. 매 실행 전 `docker compose restart matching-engine`으로 오더북을 비운 결과, 안전/경계/포화가 모두 약 10배 수준으로 상승했다. 가격 `10,000` 레벨에 쌓인 잔존 주문 수가 cancel 처리(가격 레벨 탐색·연결리스트 순회) 비용에 직접적인 영향을 준다는 것을 시사한다. 가격 레벨이 비어 있는 클린 오더북에서는 commands/unit=11 계열(안전 80k~100k)과 동일한 수준의 안전 TPS(100,000)를 보인다([오더북 초기화 절차](../../../../integration_stress.md) 참고).
