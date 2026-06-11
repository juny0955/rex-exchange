# cancel-resting-order Integration Stress

관련 문서:

- [시나리오 요약](../README.md)
- [Integration Stress 테스트](../../../integration_stress.md)
- [Runtime 측정](../runtime/README.md)

`cancel-resting-order`의 gRPC 인그레스부터 Kafka 수신까지의 E2E 부하 결과를 기록한다.
아직 기록된 integration 기준선은 없다.

## 최신 E2E 결과

| 항목 | 값 |
| --- | --- |
| 최신 측정 | 미측정 |
| 최신 안정 TPS | 미측정 |
| 경계 TPS | 미측정 |
| 대표 p99 지연 | 미측정 |
| 무손실 판정 | 미측정 |

## 고정 실행 조건

첫 측정 때 SUT, Kafka, 부하 생성기 배치와 실행 옵션을 기록한다.

대표 실행 명령:

```bash
./target/release/integration_stress --scenario cancel-resting-order --warmup-sec 5 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --sweep-depth 10 --concurrency 128 --settle-timeout-sec 15
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | 대표 p99 지연 | 무손실 | raw |
| --- | ---: | ---: | ---: | --- | --- |
| 미측정 | - | - | - | - | - |
