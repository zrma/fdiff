# Todo: Bounded Content Diff

Status: completed

## Goal

folder diff에서 양쪽에 존재하는 regular file을 선택해 같은 TUI 안에서 실제 content
차이를 읽을 수 있게 한다.

## Acceptance

- [x] folder view의 comparable file에서 `Enter`로 content diff를 연다.
- [x] text line을 좌우 같은 row에 정렬하고 replacement/addition/removal을 구분한다.
- [x] row/page/change navigation과 horizontal scroll을 제공한다.
- [x] `Esc`/`Backspace`로 기존 folder selection에 돌아온다.
- [x] binary/non-UTF-8 file은 첫 차이 byte와 bounded hex preview를 표시한다.
- [x] file당 4 MiB를 넘는 input은 읽지 않고 metadata summary로 제한한다.
- [x] terminal control character를 content display에 그대로 전달하지 않는다.
- [x] plain/check output과 folder diff classification은 바꾸지 않는다.
- [x] representative `data/` corpus와 keyboard walkthrough를 제공한다.
- [x] wide/narrow render test, actual PTY smoke, `scripts/check.sh`를 통과한다.

## Constraints

- line diff는 `similar`의 Myers algorithm과 250 ms timeout을 사용한다.
- inline word highlight, syntax highlight, external editor launch, rename detection은 다음
  slice로 남긴다.
