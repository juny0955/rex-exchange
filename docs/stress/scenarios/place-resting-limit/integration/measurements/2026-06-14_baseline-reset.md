# place-resting-limit Integration Baseline (2026-06-14, 오더북 초기화)

관련 문서:

- [place-resting-limit Integration Stress](../README.md)
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
| 시나리오 | `place-resting-limit` (commands/unit = 1) |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| 동시 실행(units) | `512` |
| gRPC 연결 수 | `16` |
| 전송 방식 | `SubmitBatch` (unit 전체를 batch RPC 1콜로 전송, commands/unit=1이므로 unary와 동일 콜 수) |
| 오더북 초기화 | 매 target rate 실행 전 `docker compose restart matching-engine` 후 healthy 대기 (오더북 완전 초기화) |
| warm-up 시간 | `5s` |
| 측정 시간 | `30s` |
| settle 제한 시간 | `15s` |

대표 실행 명령:

```bash
docker compose restart matching-engine
# matching-engine이 healthy 상태가 될 때까지 대기 후 실행
docker compose run --rm integration-stress --grpc-endpoint http://matching-engine:50051 --kafka-brokers kafka:9092 --scenario place-resting-limit --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 512 --connections 16 --settle-timeout-sec 15
```

## 결과 요약

| target/s | 판정 | 접수 성공률 | 채널 포화(503) | 503 비율 | target 달성 | 초당 접수 성공 | ack p50 | ack p99 | E2E p50 | E2E p99 | 무손실 |
| ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 25,000 | 안전 | 99.95% | 340 | 0.045% | 예 (100.00%) | 24,986.3/s | 461µs | 2.095ms | 4.681ms | 8.620ms | 통과 |
| 40,000 | 경계 | 99.39% | 7,272 | 0.61% | 예 (100.00%) | 39,755.5/s | 1.099ms | 37.267ms | 5.809ms | 43.969ms | 통과 |
| 50,000 | 포화 | 99.57% | 6,010 | 0.43% | 아니오 (93.92%) | 46,743.2/s | 3.226ms | 65.147ms | 6.975ms | 70.258ms | 통과 |

## 판정

- **안전 TPS = 25,000** (2026-06-13과 동일). `target 달성률 100%`, ack p99 `2.095ms`, E2E p99 `8.620ms`로 안정적이다. 503은 340건(0.045%) 발생했지만 비율이 매우 낮아 안전 구간으로 본다.
- **경계 TPS = 40,000** (2026-06-13과 동일). `target 달성률 100%`는 유지하지만 `채널 포화 0.61%`가 발생하고 ack p99가 `2.095ms → 37.267ms`(약 18배), E2E p99가 `8.620ms → 43.969ms`(약 5배)로 급증한다.
- **포화 TPS = 50,000+** (2026-06-13과 동일 양상). `채널 포화 0.43%`로 503 자체는 크지 않지만 `target 달성률 93.92%`로 부하 생성기가 목표 유입률을 따라가지 못한다(pacing 지연 누적 1,120,357.497s, 최대 1.833s). 이 시나리오의 포화는 503 기반이 아니라 **gRPC ack 처리 자체의 CPU 한계**(commands/unit=1 → unit마다 RPC 1콜, 콜당 고정비가 그대로 청구)로 나타난다. `무손실 판정`은 여전히 통과.
- **오더북 초기화 효과 없음(컨트롤 확인)**: 25,000/40,000/50,000 세 지점 모두 2026-06-13(오더북 오염 상태) 측정값과 거의 동일하다. `place-resting-limit`은 가격 `10,000~10,049`에 신규 주문을 계속 삽입만 하는 insert-only 워크로드이고, 이전 실행이 남긴 잔존 주문의 누적량이 신규 주문 삽입 비용에 영향을 주지 않기 때문이다. 즉 이 시나리오는 오더북 오염에 영향받지 않는 **컨트롤 그룹**이며, 동일 조건에서 `cancel-resting-order`/`amend-decrease-qty`/`amend-price-change`만 큰 변화를 보인 것은 오더북 초기화가 그 세 시나리오에만 유효하게 작용했음을 뒷받침한다([오더북 초기화 절차](../../../../integration_stress.md) 참고).
