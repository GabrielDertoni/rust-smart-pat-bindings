# Smart Pattern Bindings

Currently Rust does not allow partial borrowing on match patterns. Because of this, the following
program won't compile

```rust
enum A {
    Opt1 {
        b: B,
        /* ... */
    },
    /* ... */
}

enum B {
    Opt1 {
        e: String,
    },
    /* ... */
}

impl A {
    fn some_fn(&mut self) {
        match self {
            a@A::Opt1 { b: b@B::Opt1 { e }, .. } => {
                if e == "hello" {
                    *a = A::Opt2(10);
                } else {
                    *e = String::from("world");
                }
            }
            _ => (),
        }
    }
}
```

However, with a bit of rewriting it is possible to get the desired behavior

```rust
fn some_fn(&mut self) {
    match self {
        a@A::Opt1 { b: B::Opt1 { e: _ }, .. } => {
            let A::Opt1 { b: b@B::Opt1 { e: _ }, .. } = a else { unreachable!() };
            let B::Opt1 { e } = b else { unreachable!() };

            if e == "hello" {
                *a = A::Opt2(10);
            } else {
                *e = String::from("world");
            }
        }
        _ => (),
    }
}
```

This crate exposes the attribute macro `#[smart_pat_bindings]` that does exactly this rewriting
(using `unreachable_unchecked()`). In order to use it, just put the attribute on any match
expression and write the patterns that you want, in our example this would be

```rust
fn some_fn(&mut self) {
    #[smart_pat_bindings]
    match self {
        a@A::Opt1 { b: b@B::Opt1 { e }, .. } => {
            if e == "hello" {
                *a = A::Opt2(10);
            } else {
                *e = String::from("world");
            }
        }
        _ => (),
    }
}
```
