# 벤치마크 위키

이 디렉터리는 매칭 엔진 성능 측정 방법과 기준선을 모아두는 공간이다.
새 벤치마크를 추가하거나 최적화 전후를 비교할 때는 이 문서들을 먼저 확인한다.

## 문서 목록

| 문서 | 용도 |
| --- | --- |
| [OrderBook 벤치마크](./orderbook/orderbook.md) | `src/engine/orderbook.rs` 벤치마크의 목적, 측정 대상, 기준선, 개선 측정 내역 |
| [MatchingEngine 벤치마크](./matching-engine/matching-engine.md) | `src/engine/matching_engine.rs` 벤치마크의 목적, 측정 대상, 기준선, 개선 측정 내역 |

## 벤치마크 실행 방법

각 벤치마크 문서에 기록된 feature와 bench target을 사용해 실행한다.

```bash
cargo bench --features <feature-name> --bench <bench-target>
```

벤치마크 목록만 확인하려면 `-- --list`를 붙인다.

```bash
cargo bench --features <feature-name> --bench <bench-target> -- --list
```

Criterion HTML 리포트는 기본적으로 아래 경로에 생성된다.

```text
target/criterion/report/index.html
```

개별 벤치마크의 JSON 추정치는 아래 경로에 생성된다.

```text
target/criterion/<benchmark-name>/.../new/estimates.json
```

## 측정 환경

현재 로컬 기준 측정 환경은 아래와 같다.

| 항목 | 값 |
| --- | --- |
| OS | macOS 26.3.1, Build 25D771280a |
| Chip | Apple M4 Pro |
| Architecture | arm64 |
| Rust | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Cargo | `cargo 1.95.0 (f2d3ce0bd 2026-03-21)` |

환경을 다시 확인할 때는 아래 명령을 사용한다.

```bash
sw_vers
sysctl -n machdep.cpu.brand_string
uname -m
rustc --version
cargo --version
```

## 최적화 후 비교 방법

1. 변경하기 전에 clean working tree에서 기준 벤치를 한 번 실행한다.

```bash
cargo bench --features <feature-name> --bench <bench-target>
```

2. 최적화를 적용한다.

3. 정확성 검증을 실행한다.

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

4. 동일한 벤치마크 명령을 다시 실행한다.

```bash
cargo bench --features <feature-name> --bench <bench-target>
```

5. Criterion 리포트를 열어 `base`와 `new`의 변화율을 확인한다.

```text
target/criterion/report/index.html
```
