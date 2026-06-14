# amend-price-change Integration Baseline (2026-06-14, 오더북 초기화)

관련 문서:

- [amend-price-change Integration Stress](../README.md)
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
| 시나리오 | `amend-price-change` (commands/unit = 3) |
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
docker compose run --rm integration-stress --grpc-endpoint http://matching-engine:50051 --kafka-brokers kafka:9092 --scenario amend-price-change --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 512 --connections 16 --settle-timeout-sec 15
```

## 결과 요약

| target/s | 판정 | 접수 성공률 | 채널 포화(503) | 503 비율 | target 달성 | 초당 접수 성공 | ack p50 | ack p99 | E2E p50 | E2E p99 | 무손실 |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 75,000 | 안전 | 100.00% | 0 | 0% | 예 (100.00%) | 74,991.3/s | 477µs | 1.400ms | 5.555ms | 9.656ms | 통과 |
| 100,000 | 경계 | 99.97% | 979 | 0.033% | 예 (100.00%) | 99,834.3/s | 2.142ms | 59.245ms | 8.605ms | 69.020ms | 통과 |
| 130,000 | 포화 | 99.78% | 6,932 | 0.22% | 아니오 (82.51%) | 106,902.4/s | 2.791ms | 72.869ms | 9.794ms | 83.997ms | 통과 |

## 판정

- **안전 TPS = 75,000** (2026-06-13: 5,000 → **15배 상승**). `채널 포화 0건`, `target 달성률 100%`이며 ack p99 `1.400ms`, E2E p99 `9.656ms`로 안정적이다.
- **경계 TPS = 100,000** (2026-06-13: 10,000 → **10배 상승**). `target 달성률 100%`는 유지하지만 `채널 포화 0.033%`가 발생하고 ack p99가 `1.400ms → 59.245ms`(약 42배), E2E p99가 `9.656ms → 69.020ms`(약 7배)로 급증한다.
- **포화 TPS = 130,000** (2026-06-13: 20,000 → **6.5배 상승**). `채널 포화 0.22%`, `target 달성률 82.51%`로 부하 생성기가 목표 유입률을 따라가지 못한다(pacing 지연 누적 2,794,234.321s, 최대 5.255s). ack p99 `72.869ms`, E2E p99 `83.997ms`로 경계 구간보다 추가 상승한다. `무손실 판정`은 통과(settle 완료, 누락·중복 없음).
- **오더북 초기화 효과**: 2026-06-13 baseline은 `place-resting-limit`이 가격 `10,000~10,049`에 남긴 잔존 주문 위에서 측정되어 안전/경계/포화 지점이 5,000/10,000/20,000으로 가장 큰 폭으로 낮게 나왔다. 매 실행 전 `docker compose restart matching-engine`으로 오더북을 비운 결과, 안전 TPS는 15배(5,000→75,000)까지 상승했다. `amend-price-change`는 가격 `10,000→10,001` cancel-replace로 두 가격 레벨을 모두 다루므로, 두 레벨 모두에 잔존 주문이 쌓여 있던 오염 상태에서 비용이 가장 크게 증폭되었던 것으로 보인다. 클린 오더북에서는 같은 commands/unit=3인 `amend-decrease-qty`(안전 50,000)보다도 오히려 높은 안전 TPS(75,000)를 보인다([오더북 초기화 절차](../../../../integration_stress.md) 참고).
