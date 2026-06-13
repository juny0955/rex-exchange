# amend-decrease-qty Runtime Stress

관련 문서:

- [Stress 테스트 홈](../../../README.md)
- [Runtime Stress 테스트](../../../runtime_stress.md)

`amend-decrease-qty`는 GTC 주문을 넣은 뒤 같은 가격에서 수량만 줄이고, 같은 주문 ID를 다시 취소하는 시나리오다.
체결은 발생하지 않고 orderbook 삽입, in-place 수량 정정, 삭제 비용과 result publish 경로를 확인한다.
Place/Amend/Cancel 3-command 묶음으로 구성되어 반복 종료 시 orderbook이 비워지므로 stationary 정정 경로 기준선으로 사용한다.

## 최신 TPS

| 항목 | 값 |
| --- | --- |
| 최신 안정 TPS | 미측정 |
| 경계 TPS | 미측정 |
| 초과 구간 | 미측정 |

로컬 Docker 환경에서 새 측정을 진행한 뒤 이 표를 갱신한다.

## 고정 실행 조건

| 항목 | 값 |
| --- | --- |
| 심볼 수 | `1` |
| 스윕 깊이 | `10` |
| 발행 지연 | `0ms` |
| warm-up 시간 | `10s` |
| 측정 시간 | `30s` |
| 제한 시간 | `30s` |
| 실행 모드 | `paced` |

`sweep-depth`는 출력에는 표시되지만 이 시나리오의 workload 생성에는 사용되지 않는다.

대표 실행 명령:

```bash
./target/release/runtime_stress --scenario amend-decrease-qty --warmup-sec 10 --duration-sec 30 --target-commands-per-sec <target> --symbols 1 --timeout-sec 30
```

## 측정 내역

| 측정 | 안정 TPS | 경계 TPS | raw |
| --- | ---: | ---: | --- |
| 미측정 | - | - | - |

새 측정 결과가 생기면 `measurements/`와 `raw/`에 같은 날짜/목적 이름으로 파일을 추가하고 이 표를 갱신한다.
