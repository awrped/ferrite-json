# ferrite

humans make errors, so we should show it to them, so it can be fixed.

## usage

```bash
cargo build --release
./target/release/ferrite <file.json>
```

## example output

```
Error: ferrite::trailing_comma

  x trailing comma
   ,-[4:42]
 3 |   "age": 30,
 4 |   "hobbies": ["reading", "coding", "gaming",]
   :                                          |
   :                                          `-- remove this comma
 5 | }
   `----
  help: change `"hobbies": ["reading", "coding", "gaming",]` to `"hobbies": ["reading", "coding", "gaming"]`
```

## contributing

plsz make sure to pre-lint with clippy before making a pr:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```