# Runtime Stress 위키

이 디렉터리는 `runtime_stress` 부하 테스트의 실행 방법, 시나리오별 기준선, 개선 비교 결과를 관리하는 공간이다.
Criterion 벤치마크 문서는 `docs/bench`에 두고, 런타임 채널 포화와 result publish 경로를 보는 부하 테스트는 이 위키에서 관리한다.
gRPC 인그레스부터 Kafka 발행까지 종단(E2E) 부하는 `integration_stress`로 측정한다.

## 고정 측정 환경

runtime stress 기준선과 개선 비교는 아래 환경에서 고정 측정한다.
다른 환경에서 실행한 결과는 이 기준선과 직접 비교하지 않는다.

| 항목 | 값 |
| --- | --- |
| Cloud Provider | Vultr |
| OS | Ubuntu 24.04 |
| Kernel | Linux 6.8.0-124-generic x86_64 |
| Architecture | x86_64 |
| CPU | AMD EPYC-Rome Processor |
| vCPU | 2 |
| Physical Core / Thread | 1 Core / 2 Threads |
| CPU Clock | 2.0GHz |
| Virtualization | Full virtualization, QEMU |
| NUMA Node | 1 |
| L1 Cache | 32 KiB I-cache / 32 KiB D-cache |
| L2 Cache | 512 KiB |
| L3 Cache | 16 MiB |
| Memory | 3.8 GiB |
| Available Memory | 3.3 GiB |
| Swap | 8.0 GiB, unused |

## 문서 목록

| 문서 | 용도 |
| --- | --- |
| [Runtime Stress 테스트](./runtime_stress.md) | `runtime_stress` 도구의 목적, 옵션, 시나리오, 실행 방법 |
| [Integration Stress 테스트](./integration_stress.md) | `integration_stress` 도구의 목적, 옵션, 시나리오, gRPC+Kafka 종단 실행 방법 |
| [full-fill-same-level](./scenarios/full-fill-same-level/README.md) | 동일 호가 전량 체결 시나리오의 측정 요약과 측정 내역 |
| [market-quote-sweep](./scenarios/market-quote-sweep/README.md) | 시장가 금액 스윕 시나리오의 측정 요약과 측정 내역 |
| [partial-fill-rest](./scenarios/partial-fill-rest/README.md) | 부분 체결 후 잔존 시나리오의 측정 요약과 측정 내역 |
| [place-resting-limit](./scenarios/place-resting-limit/README.md) | 미체결 지정가 잔존 시나리오의 측정 요약과 측정 내역 |
| [cancel-resting-order](./scenarios/cancel-resting-order/README.md) | 잔존 주문 취소 시나리오의 측정 요약과 측정 내역 |
| [amend-decrease-qty](./scenarios/amend-decrease-qty/README.md) | 수량 감소 정정 시나리오의 측정 요약과 측정 내역 |
| [amend-price-change](./scenarios/amend-price-change/README.md) | 가격 변경 정정 시나리오의 측정 요약과 측정 내역 |

## 최신 측정 TPS 요약

`최신 안정 TPS`는 같은 target/s의 기록된 run이 모두 `채널 포화 = 0`, `완료 여부 = 예`, `발행 결과 ~= 접수 성공`을 만족할 때 인정한다.
일부 run에서만 channel full이 발생한 target/s는 `경계 TPS`로 분리한다.

| 시나리오 | 최신 측정 | 최신 안정 TPS | 경계 TPS | 초과 구간 | 요약 | raw |
| --- | --- | ---: | ---: | --- | --- | --- |
| `full-fill-same-level` | 2026-06-09 기준선 | 200,000 commands/s | 300,000 commands/s | 380,000+ commands/s | [summary](./scenarios/full-fill-same-level/measurements/2026-06-09_baseline.md) | [raw](./scenarios/full-fill-same-level/raw/2026-06-09_baseline.log) |
| `market-quote-sweep` | 2026-06-09 기준선 | 300,000 commands/s | 330,000 commands/s | 370,000 commands/s | [summary](./scenarios/market-quote-sweep/measurements/2026-06-09_baseline.md) | [raw](./scenarios/market-quote-sweep/raw/2026-06-09_baseline.log) |
| `partial-fill-rest` | 2026-06-09 기준선 | 50,000 commands/s | 55,000 commands/s | 55,000+ commands/s | [summary](./scenarios/partial-fill-rest/measurements/2026-06-09_baseline.md) | [raw](./scenarios/partial-fill-rest/raw/2026-06-09_baseline.log) |
| `place-resting-limit` | 2026-06-09 기준선 | 10,000 commands/s | 15,000 commands/s | 15,000+ commands/s | [summary](./scenarios/place-resting-limit/measurements/2026-06-09_baseline.md) | [raw](./scenarios/place-resting-limit/raw/2026-06-09_baseline.log) |
| `cancel-resting-order` | 2026-06-09 기준선 | 200,000 commands/s | 300,000-900,000 commands/s | 950,000+ commands/s | [summary](./scenarios/cancel-resting-order/measurements/2026-06-09_baseline.md) | [raw](./scenarios/cancel-resting-order/raw/2026-06-09_baseline.log) |
| `amend-decrease-qty` | 2026-06-09 기준선 | 500,000 commands/s | 700,000-850,000 commands/s | 900,000+ commands/s | [summary](./scenarios/amend-decrease-qty/measurements/2026-06-09_baseline.md) | [raw](./scenarios/amend-decrease-qty/raw/2026-06-09_baseline.log) |
| `amend-price-change` | 2026-06-09 기준선 | 300,000 commands/s | 500,000-700,000 commands/s | 800,000+ commands/s | [summary](./scenarios/amend-price-change/measurements/2026-06-09_baseline.md) | [raw](./scenarios/amend-price-change/raw/2026-06-09_baseline.log) |

새 시나리오를 측정하면 `docs/stress/scenarios/<scenario>/` 아래에 summary와 raw를 추가하고 이 표를 갱신한다.
`partial-fill-rest`, `place-resting-limit`는 잔존 주문이 누적되는 non-stationary 시나리오이므로 book이 비워지는 시나리오와 직접 비교하지 않는다.

## 운영 규칙

- 고정 측정 환경이 바뀌면 기존 기준선과 분리된 새 기준선 문서를 만든다.
- 시나리오별 측정 결과는 `scenarios/<scenario>/measurements/`에 요약 문서로 저장한다.
- 시나리오별 원본 또는 원본에 준하는 측정 기록은 `scenarios/<scenario>/raw/`에 저장한다.
- 기준선 문서는 오탈자나 계산 오류를 제외하고 수정하지 않는다.
- 개선 후 측정은 새 문서로 추가하고, 기준선 대비 변화율을 기록한다.
- `완료 여부 = 예`는 접수된 command만 완료됐다는 뜻이며, 거부된 command까지 처리됐다는 뜻이 아니다.
