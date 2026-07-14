# fdiff Sample Corpus

Commander view와 content diff를 함께 확인하려면 repository root에서 실행한다.

```sh
cargo run -- data/folder1/demo data/folder2/demo
```

기본 summary는 `3 left-only · 8 changed · 1 type · 2 right-only · 5 same`이다.

## Expected Folder Diff

| Path | Expected status | Content diff highlight |
| --- | --- | --- |
| `01-changed-lines.txt` | changed | replacement, removal, addition |
| `02-added-removed.txt` | changed | unequal-size changed block |
| `03-unicode.txt` | changed | Korean text and emoji |
| `04-long-line.txt` | changed | horizontal scrolling |
| `05-blank-vs-content.txt` | changed | blank line versus text |
| `06-same.txt` | same | visible after pressing `a` |
| `nested/guide.md` | changed | nested path navigation |
| `services/api/config.toml` | changed | repeated filename at one path |
| `services/worker/config.toml` | changed | repeated filename at another path |
| `left-only.txt` | left-only | content view unavailable |
| `right-only.txt` | right-only | content view unavailable |
| `type-change` | type-changed | directory versus file |
| `rename-old.txt`, `rename-new.txt` | one-sided | rename detection is not implemented |

## Keyboard Walkthrough

1. `01-changed-lines.txt`를 선택하고 `Enter`를 눌러 line diff를 연다.
2. `n`/`p`로 changed row를 이동하고 arrow key 또는 `h`/`l`로 긴 줄을 좌우로
   스크롤한다.
3. `Esc` 또는 `Backspace`로 folder view에 돌아온다.
4. `a`를 눌러 `06-same.txt`와 동일한 중간 directory를 표시하거나 숨긴다.
5. `type-change`와 one-sided file에서는 content diff가 열리지 않는지 확인한다.

Binary, non-UTF-8, inline size-limit 처리는 unit test fixture로 검증한다. Binary sample은
텍스트 기반 repository fixture에 임의 byte를 섞지 않기 위해 tracked corpus에 포함하지
않는다.
