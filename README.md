# RefTake

**RefTake** is a non-owning alternative to [`std::io::Take`] that allows limiting how many bytes can be read from a referenced reader without taking ownership of it.

It is useful in scenarios where:
- You need to apply a byte limit to an existing borrowed reader.
- You’re implementing stream parsers or protocols where ownership cannot be moved.
- You want to reuse a single reader across multiple limited reads.

---

## ✨ Features

- ✅ Works with any type implementing `Read` or `BufRead`
- ✅ Does not take ownership — wraps `&mut R` instead of consuming `R`
- ✅ `Read` and `BufRead` implementations respect the byte limit
- ✅ Supports dynamic limit adjustment via `.set_limit()`
- ✅ Extension trait to simplify usage: `.take_ref(limit)`

---

## 📦 Usage

### Add to your project

Add it to your `Cargo.toml`:

```toml
[dependencies]
reftake = "0.1"
```

### Example

```rust
use std::io::{Cursor, Read};
use ref_take::RefTakeExt;

fn main() {
    let mut reader = Cursor::new(b"hello world");
    let mut limited = (&mut reader).by_ref_take(5);

    let mut buffer = String::new();
    limited.read_to_string(&mut buffer).unwrap();
    assert_eq!(buffer, "hello");
}
```

---

## 🔒 License

MIT OR Apache-2.0

---

## 🔧 Contributing

Feel free to open issues, suggest improvements, or submit pull requests.

---

## 📎 Related

- [`std::io::Take`](https://doc.rust-lang.org/std/io/struct.Take.html) — the owning version
- [`BufReader`](https://doc.rust-lang.org/std/io/struct.BufReader.html) — for buffered readers
