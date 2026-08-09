# 02 — Rust for complete beginners

This is a from-zero tour of the Rust language, focused on **the features you actually see
in this repo**. It is not a replacement for [The Rust Book](https://doc.rust-lang.org/book/)
(the free official book), but it front-loads exactly what you need to read
[`src/main.rs`](../src/main.rs) and [`src/can_fd.rs`](../src/can_fd.rs) with understanding.

Read this with the source files open in a split view.

---

## 1. The mental model

Rust is a **compiled, statically typed, systems** language. That means:

- **Compiled** — a program (`rustc`, driven by `cargo`) turns your text into machine code
  *before* it runs. Errors are caught at compile time, not when the car is on the road.
- **Statically typed** — every value has a type known at compile time (`u8`, `bool`, a
  struct, etc.). The compiler checks they fit together.
- **Systems** — you control memory layout and have no garbage collector, so it's suitable
  for a 128 KB microcontroller.

Rust's superpower is the **ownership system**: a set of compile-time rules that guarantee
memory safety and data-race freedom *with no runtime cost*. Everything below builds to that.

---

## 2. Values, variables, and mutability

```rust
let x = 5;          // immutable by default
let mut y = 10;     // `mut` = you intend to change it
y += 1;             // ok
// x = 6;           // COMPILE ERROR: x is not `mut`
```

**Immutable by default** is a safety choice: the compiler assumes nothing changes unless
you *say so*, which eliminates a huge class of accidental-mutation bugs. You see this in
the heartbeat task: `let mut led` — the LED handle must be mutable because `toggle()`
changes it.

### Types you'll see here

| Type | Meaning | Example in repo |
| ---- | ------- | --------------- |
| `u8`, `u16`, `u32` | unsigned integers of 8/16/32 bits | `HEARTBEAT_ID: u16 = 0x100` |
| `i32`, `i8` … | signed integers | — |
| `bool` | `true` / `false` | — |
| `[u8; 2]` | fixed-size array of 2 bytes | `let payload = [0xAA, sequence];` |
| `&[u8]` | a *slice* — a borrowed view of a run of bytes | `frame.data()` returns one |
| `char`, `&str`, `String` | text (rare on bare metal) | log format strings |

Note the **exact-width integers**. On an MCU you care whether a value is 8 or 32 bits —
it affects memory and matches hardware registers. `u8` is one byte, `0x100` (256) needs a
`u16`. Being explicit is the norm here.

### Constants

```rust
const HEARTBEAT_PERIOD: Duration = Duration::from_millis(500);
```

`const` is a compile-time constant — it has no memory address of its own; the value is
baked in wherever it's used. Naming magic numbers as `const`s (bitrates, IDs, periods) is
why `can_fd.rs` is readable.

---

## 3. Ownership — the one big idea

Every value in Rust has exactly **one owner** (a variable). When the owner goes out of
scope, the value is **dropped** (its memory/resources freed) — automatically, at a
compile-time-known point. No garbage collector, no manual `free()`.

```rust
let a = String::from("hi"); // `a` owns the string
let b = a;                  // ownership MOVES to `b`; `a` is now invalid
// println!("{}", a);       // COMPILE ERROR: `a` was moved
```

This "move" rule means two variables can't both think they own (and free) the same
memory → **no double-free, no use-after-free**. In embedded terms, this is also how
Embassy guarantees a hardware peripheral has exactly one owner: when `main` hands
`peripherals.PA5` to the `heartbeat` task, the pin *moves* into the task. Nothing else can
touch it. That's "freedom from interference" enforced by the compiler.

### Borrowing and references

Moving everything would be painful, so you can **borrow** a value with a reference:

```rust
fn read_len(data: &[u8]) -> usize { data.len() } // borrows, doesn't take ownership
```

- `&T` — a **shared/immutable** borrow. You can have *many* at once, but can't mutate.
- `&mut T` — a **unique/mutable** borrow. You can have *exactly one*, and no shared borrows
  at the same time.

This "many readers XOR one writer" rule, checked by the **borrow checker**, is what makes
data races *impossible to compile*. You see `&frame`, `&payload`, `&mut led` throughout.

### Lifetimes and `'static`

A **lifetime** is the compiler's name for "how long a reference is valid." Usually it's
inferred. You'll see one explicit lifetime a lot here: **`'static`**, meaning "lives for
the entire program." Embassy tasks require their arguments to be `'static` because a task
may run forever, so anything it holds must never be freed:

```rust
async fn heartbeat(mut led: Output<'static>) { … }
```

`Output<'static>` = "an output pin that is valid for the whole program's life." This is
safe because MCU peripherals genuinely do live forever.

---

## 4. Structs, enums, and pattern matching

### Structs — bundle related data

```rust
struct Frame { id: u16, data: [u8; 8] } // (illustrative)
```

You mostly *use* structs from Embassy here (`Output`, `CanTx`, `Frame`) rather than define
your own, but the idea is the same: a named bundle of fields.

### Enums — one of several variants

Rust enums are far more powerful than C enums: each variant can carry data.

```rust
enum Level { Low, High } // used as Output::new(pin, Level::Low, …)
```

The two most important enums in all of Rust are **`Option`** and **`Result`**.

### `Option<T>` — "a value, or nothing"

Rust has **no null**. Absence is a real type:

```rust
enum Option<T> { Some(T), None }
```

You must *handle* the `None` case, so "forgot to check for null" bugs can't compile.

### `Result<T, E>` — "success or failure"

This is how Rust does error handling — **no exceptions**:

```rust
enum Result<T, E> { Ok(T), Err(E) }
```

`Frame::new_standard(id, &payload)` returns a `Result` because the id/payload might be
invalid. You *must* deal with both arms. That's why `can_fd.rs` does:

```rust
match Frame::new_standard(HEARTBEAT_ID, &payload) {
    Ok(frame)   => { /* send it */ }
    Err(_error) => warn!("CAN TX: refused to build frame (invalid payload/id)"),
}
```

### `match` — exhaustive pattern matching

`match` forces you to handle *every* possible variant (the compiler checks). This is the
"safety-oriented" style the repo teaches: you can't silently ignore an error case because
the code won't compile until you address it.

---

## 5. Handling errors without crashing

There are several ways to deal with a `Result`, and *which one you choose matters* in
embedded:

| Tool | What it does | Use in this repo |
| ---- | ------------ | ---------------- |
| `match` | handle every case explicitly | **preferred** in run loops |
| `if let Ok(x) = …` | handle just the success case | occasional |
| `?` operator | "return the error up to my caller" | in fallible helper fns |
| `.unwrap()` / `.expect()` | "on error, **panic** (crash)" | **avoided** in run loops |
| `unwrap!(…)` (from `defmt`) | like `unwrap()` but logs efficiently, used at *spawn time* | `spawner.spawn(unwrap!(heartbeat(led)))` |

> **The golden rule you're being taught:** an `.unwrap()` that fails **halts the whole
> microcontroller**. In a car that's a dead ECU. So the task loops in `can_fd.rs` never
> `unwrap()` — they `match` and log. The only `unwrap!` calls are at startup, where a
> failure to even spawn a task genuinely *is* unrecoverable and should stop the boot.

The `_error` / `_timestamp` names: a leading underscore tells the compiler "I'm
deliberately not using this binding," silencing the unused-variable warning while keeping
the code self-documenting.

---

## 6. Traits — shared behavior

A **trait** is like an interface: a set of methods a type promises to provide. This is how
Rust does polymorphism without inheritance.

You rarely define traits here, but they're everywhere under the surface: `Future` (the
trait that powers `async`), `defmt::Format` (lets a type be logged with `{:?}`), and the
`embedded-hal` traits that let generic drivers work across chips. When you see `{:?}` in a
log — `info!("CAN RX: {} bytes {:?}", data.len(), data)` — that works because `&[u8]`
implements `defmt::Format`.

---

## 7. Functions, modules, and visibility

```rust
pub fn init(…) -> (CanTx<'static>, CanRx<'static>) { … }
```

- `fn` declares a function; `-> (…, …)` is the return type (here a **tuple** of two values).
- `pub` makes it **public** — visible outside its module. Without `pub`, items are private
  to their module.
- **Modules** (`mod`) group code. `mod can_fd;` in `main.rs` pulls in `can_fd.rs` as a
  module; `can_fd::init(…)` calls into it. `mod irqs { … }` is an *inline* module used to
  fence off the one `unsafe` block (see below).

`use` brings names into scope so you don't have to write the full path every time:
`use embassy_time::{Duration, Timer};`.

### Attributes

Lines starting with `#[…]` or `#![…]` are **attributes** — metadata for the compiler:

- `#![no_std]` (crate-level, note the `!`) — "don't link the standard library." (Doc 03.)
- `#[embassy_executor::task]` — a **macro** that transforms an `async fn` into a spawnable
  task (Doc 04).
- `#[allow(unsafe_code)]` / `#![deny(clippy::all)]` — turn lint rules on/off (Doc 10).

---

## 8. `async`/`await` in one paragraph (full detail in Doc 04)

An `async fn` doesn't run immediately — calling it produces a **future**, a value that
represents "work that can make progress, pause, and resume." Writing `.await` on something
means "pause here until this is ready, and let other tasks run meanwhile." On this MCU
there are no OS threads; instead the **Embassy executor** juggles these futures on the
single CPU core. When the heartbeat task hits `Timer::after(…).await`, it steps aside so
the CAN tasks can run, and the CPU sleeps if nobody needs it. That's how three "concurrent"
tasks share one core with no locks and no races.

---

## 9. Macros

Anything ending in `!` is a **macro**, not a function: `info!`, `warn!`, `unwrap!`,
`bind_interrupts!`. Macros generate code at compile time. `defmt`'s logging macros are
special: they intern the format string at compile time and only send tiny IDs at runtime
(see [09-toolchain-build-flash.md](09-toolchain-build-flash.md)). `bind_interrupts!`
generates the glue that wires hardware interrupt vectors to Embassy's handlers.

---

## 10. Putting it together: read the heartbeat task

```rust
#[embassy_executor::task]                 // macro: make this a spawnable task
async fn heartbeat(mut led: Output<'static>) {  // owns a 'static, mutable output pin
    loop {                                 // run forever
        led.toggle();                      // flip the pin (mutation → needs `mut`)
        Timer::after(HEARTBEAT_PERIOD).await; // pause 500ms, yield the CPU
    }
}
```

Every keyword there now has meaning: the attribute macro, `async`, ownership of `led`,
`'static`, `mut`, the infinite `loop`, and `.await` yielding control. If you can explain
each to yourself, you've got the Rust fundamentals this project needs.

**Next:** [03-embedded-rust.md](03-embedded-rust.md) — what changes when there's no OS.

---

### Where to go deeper (official)

- [The Rust Programming Language ("The Book")](https://doc.rust-lang.org/book/) — the canonical intro.
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) — runnable snippets.
- [The `std`/`core` API docs](https://doc.rust-lang.org/core/) — reference (`core` is the `no_std` subset).
