# Example todo fixture

## Setup

- [x] Initialize repo
- [x] Add CI config
- [ ] Write contributing guide

## Features

- [ ] Implement parser
  - [/] Write lexer
  - [ ] Write AST builder
- [/] Add JSON output mode
- [x] Add `--help` flag

### Edge cases

- [ ] Handle empty files
- [x] Handle fenced code blocks

```js
// This [ ] should NOT be parsed as a todo
const x = '[ ] not a todo';
// [/] also not a todo
```

- [ ] Last item after fence
