# place-resting-limit Integration Stress

관련 문서:

- [시나리오 요약](../README.md)
- [Integration Stress 테스트](../../../integration_stress.md)
- [Runtime 측정](../runtime/README.md)

`place-resting-limit`의 gRPC 인그레스부터 Kafka 수신까지의 E2E 부하 결과를 기록한다.

## 최신 E2E 결과

| 항목 | 값 |
| --- | --- |
| 최신 측정 | [2026-06-13 baseline](./measurements/2026-06-13_baseline.md) |
| 최신 안전 TPS | 25,000 |
| 경계 TPS | 40,000 |
| 포화 TPS | 50,000+ |
| 대표 p99 지연 (E2E, 25k) | 8.281ms |
| 무손실 판정 | 통과 |

## 고정 실행 조건

| 항목 | 값 |
| --- | --- |
| 동시 실행(units) | `512` |
| gRPC 연결 수 | `16` |
| 전송 방식 | `SubmitBatch` (unit 전체를 batch RPC 1콜로 전송, commands/unit=1이므로 unary와 동일 콜 수) |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| warm-up 시간 | `5s` |
| settle 제한 시간 | `15s` |

대표 실행 명령:

```bash
docker compose run --rm integration-stress --grpc-endpoint http://matching-engine:50051 --kafka-brokers kafka:9092 --scenario place-resting-limit --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 512 --connections 16 --settle-timeout-sec 15
```

## 측정 내역

| 측정 | 안전 TPS | 경계 TPS | 포화 TPS | 대표 p99 지연(E2E) | 무손실 | raw |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| [2026-06-13 baseline](./measurements/2026-06-13_baseline.md) | 25,000 | 40,000 | 50,000+ | 8.281ms | 통과 | [raw](./raw/2026-06-13_baseline.log) |
