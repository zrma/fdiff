# fdiff

`fdiff`는 두 폴더의 현재 차이를 Commander 스타일의 좌우 터미널 화면으로 보여
주는 Rust 기반 폴더 diff 도구다. 동일한 상대 경로가 두 pane의 같은 행에 맞물려
표시되므로 한쪽에만 있거나 내용이 다른 항목을 빠르게 훑을 수 있다. 터미널에서
실행하면 live TUI가 열리고, 파이프나 CI에서는 안정적인 plain text snapshot을
출력한다.

## 현재 제공하는 기능

- 왼쪽에만 있는 경로, 오른쪽에만 있는 경로, 내용이 달라진 파일, 타입이 바뀐
  경로를 구분한다.
- 일반 파일은 크기와 BLAKE3 digest로 비교하며, 반복 scan에서는 변경되지 않은
  파일의 digest를 재사용한다.
- symlink는 링크 대상 경로를 비교하고 디렉터리 자체는 양쪽에 존재하면 동일하게
  본다.
- TUI는 synchronized dual-pane, folder tree 접기/펼치기, pane focus 전환을
  제공하고 주기적으로 다시 scan한다.
- 선택 경로와 접힌 folder는 가능한 경우 scan 뒤에도 유지되며, 좁은 터미널에서는
  두 pane을 위아래로 배치한다.
- stdout이 TTY가 아니면 자동으로 plain mode를 사용한다.

## 실행

```sh
cargo run -- <left-directory> <right-directory>
```

plain snapshot과 automation용 exit status는 다음처럼 사용할 수 있다.

```sh
cargo run -- --plain <left-directory> <right-directory>
cargo run -- --check <left-directory> <right-directory>
```

`--check`는 차이가 없으면 `0`, 차이가 있으면 `1`, 실행 오류가 발생하면 `2`를
반환한다.

## TUI 키

| 키 | 동작 |
| --- | --- |
| `j`, `↓` | 다음 항목 |
| `k`, `↑` | 이전 항목 |
| `g`, `Home` | 첫 항목 |
| `G`, `End` | 마지막 항목 |
| `Tab` | 활성 pane 전환 |
| `h`, `←` | folder 접기 또는 상위 folder 선택 |
| `l`, `→` | folder 펼치기 |
| `Enter` | 선택한 folder 접기/펼치기 |
| `Space` | live scan 일시 정지/재개 |
| `a` | 동일 항목 표시/숨김 |
| `r` | 즉시 다시 scan |
| `q`, `Esc` | 종료 |

## 개발

```sh
scripts/check.sh
```

현재 상태와 다음 제품 slice는 `docs/HANDOFF.md`에서 시작한다. AI agent의 공통
실행 계약은 `AGENTS.md`와 `docs/agent-harness.md`가 소유한다.
