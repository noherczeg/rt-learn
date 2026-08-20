# 02 — Rust for complete beginners

This is a from-zero tour of the Rust language, focused on **the features you actually see
in this repo**. It is not a replacement for [The Rust Book](https://doc.rust-lang.org/book/)
(the free official book), but it front-loads exactly what you need to read
[`src/main.rs`](../src/main.rs) with understanding.

Read this with the source file open in a split view.

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
| `u8`, `u16`, `u32` | unsigned integers of 8/16/32 bits | a byte buffer `[u8; 4]` |
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
baked in wherever it's used. Naming magic numbers as `const`s (periods, sizes, limits) is
why the code stays readable — e.g. `HEARTBEAT_PERIOD` instead of a bare `500`.

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

You mostly *use* structs from Embassy here (e.g. `Output` for a GPIO pin) rather than define
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

Many fallible operations return a `Result` — the caller *must* deal with both arms. For
example, a peripheral read that can fail is handled by `match`ing on the result rather than
assuming success:

```rust
match some_fallible_read() {
    Ok(value)   => { /* use it */ }
    Err(_error) => warn!("read failed; keeping the loop alive"),
}
```

(The current firmware's `heartbeat` loop has no fallible call, but this is the pattern you
reach for the moment you add a peripheral that can error.)

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
> microcontroller**. In a car that's a dead ECU. So run-loop tasks should never
> `unwrap()` — they `match` and log, staying alive through transient errors. The only
> `unwrap!` calls belong at startup, where a failure to even spawn a task genuinely *is*
> unrecoverable and should stop the boot (`spawner.spawn(unwrap!(heartbeat(led)))`).

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
log — `info!("buffer = {:?}", data)` for a `&[u8]` — that works because `&[u8]`
implements `defmt::Format`.

---

## 7. Functions, modules, and visibility

```rust
fn init(config: Config) -> Peripherals { … }
```

- `fn` declares a function; `->` gives the return type.
- `pub` makes an item **public** — visible outside its module. Without `pub`, items are
  private to their module.
- **Modules** (`mod`) group code. This crate is small enough to live in a single module
  (`main.rs`); as it grows you would split code into files and pull them in with
  `mod <name>;`, then call across with `<name>::<item>`.

`use` brings names into scope so you don't have to write the full path every time:
`use embassy_time::{Duration, Timer};`.

### Attributes

Lines starting with `#[…]` or `#![…]` are **attributes** — metadata for the compiler:

- `#![no_std]` (crate-level, note the `!`) — "don't link the standard library." (Doc 03.)
- `#[embassy_executor::task]` — a **macro** that transforms an `async fn` into a spawnable
  task (Doc 04).
- `#[allow(unsafe_code)]` / `#![deny(clippy::all)]` — turn lint rules on/off (Doc 09).

---

## 8. `async`/`await` in one paragraph (full detail in Doc 04)

An `async fn` doesn't run immediately — calling it produces a **future**, a value that
represents "work that can make progress, pause, and resume." Writing `.await` on something
means "pause here until this is ready, and let other tasks run meanwhile." On this MCU
there are no OS threads; instead the **Embassy executor** juggles these futures on the
single CPU core. When the heartbeat task hits `Timer::after(…).await`, it steps aside so
any other ready task can run, and the CPU sleeps if nobody needs it. That's how multiple
"concurrent" tasks share one core with no locks and no races.

---

## 9. Macros

Anything ending in `!` is a **macro**, not a function: `info!`, `warn!`, `unwrap!`.
Macros generate code at compile time. `defmt`'s logging macros are
special: they intern the format string at compile time and only send tiny IDs at runtime
(see [08-toolchain-build-flash.md](08-toolchain-build-flash.md)).

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
