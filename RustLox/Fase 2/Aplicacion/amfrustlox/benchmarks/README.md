# Benchmarks

Project goal: **amfrustlox must be at least 10% faster than the reference Java
interpreter (jlox)**, i.e. `amfrustlox time / jlox time <= 0.90`.

`run_benchmarks.py` runs the same programs on both interpreters and prints the
ratio per program, plus the geometric mean and how many programs meet the goal.
It needs Python 3 (standard library only). jlox is only needed the first time.

## Programs

The `.lox` files in `lox/` are the benchmark programs from the book's
repository ([munificent/craftinginterpreters](https://github.com/munificent/craftinginterpreters),
`test/benchmark/`, MIT license, see `lox/LICENSE-MIT.txt`).

| Program | What it stresses |
|---|---|
| `fib` | function calls and arithmetic (recursive `fib(35)`) |
| `invocation` | plain function calls |
| `method_call` | method calls |
| `instantiation` | creating instances |
| `properties` | field reads and writes |
| `equality`, `string_equality` | `==` on numbers / strings in a loop |
| `binary_trees`, `trees` | allocation-heavy object graphs |
| `zoo` | many small method calls on one instance |
| `zoo_batch` | like `zoo`, but runs for a fixed 10 s and reports **batches completed** (higher is better) |

Every program prints its own elapsed time as its last line, measured with
`clock()`. The runner uses that number, so JVM start-up and process spawning
are not counted. The exceptions are `equality` and `string_equality`, whose last
line is `elapsed - loopTime` (a difference that can be near zero or negative);
for those the runner uses the total time printed after the `elapsed` label.
A run that produces a non-positive time is reported as failed, never as a pass.

## Usage

Run everything from the `amfrustlox` directory.

```sh
# 1. First time: compile jlox from a clone of the book's repo and save its numbers.
#    Clone it OUTSIDE this git repository (here: next to the project folder), then
#    point --jlox-src at wherever it ended up. The path is relative to the folder
#    you run the command from.
git clone --depth 1 https://github.com/munificent/craftinginterpreters ../craftinginterpreters
python benchmarks/run_benchmarks.py --jlox-src ../craftinginterpreters --save benchmarks/baseline.json
# (if you cloned inside amfrustlox instead, use --jlox-src craftinginterpreters
#  and add that folder to .gitignore, since it is a repository of its own)

# 2. While optimising: only amfrustlox is re-run; jlox numbers come from the file
python benchmarks/run_benchmarks.py --jlox-from benchmarks/baseline.json

# Useful options
python benchmarks/run_benchmarks.py --jlox-from benchmarks/baseline.json --only fib,invocation --runs 5
python benchmarks/run_benchmarks.py --jlox-from benchmarks/baseline.json --strict   # exit 1 if the goal is missed
```

Other options: `--no-build` (skip `cargo build --release`), `--rust-bin PATH`,
`--jlox-classes DIR` (already compiled jlox), `--timeout SECONDS`. See `--help`.

### Which Java runs jlox

By default the `java` found on PATH is used, and jlox is compiled for exactly
that version, so a `javac` newer than `java` still works. But **the Java version
changes jlox's speed**, so use a recent JDK for the comparison. On Windows
`java` and `javac` often come from different installs (for example an old
Java 8 `java` next to a new JDK's `javac`). Pick the JDK's own `java` explicitly:

```
python benchmarks/run_benchmarks.py --jlox-src craftinginterpreters --java "C:\Program Files\Java\jdk-24\bin\java.exe" --save benchmarks/baseline.json
```

The runner prints which `java` and version it used. Mention that version when
you report results, e.g. "jlox on OpenJDK 24".
Each benchmark runs 3 times by default and the **median** is used.

## Reading the output

```
benchmark             jlox  amfrustlox    ratio  verdict
invocation            1.51        3.59    2.37x  slower
zoo_batch *         652.00      365.00    1.79x  slower
```

`ratio` is always "amfrustlox cost relative to jlox", so lower is better and
`<= 0.90x` meets the goal. For `zoo_batch *` the columns are batch counts, and
the ratio is `jlox / amfrustlox` so it reads the same way.

## Getting trustworthy numbers

- jlox and amfrustlox numbers are only comparable if measured **on the same
  machine under the same conditions**. A saved `baseline.json` is valid for the
  machine that produced it; regenerate it if you switch computers.
- Close other heavy programs and plug in the laptop. Use `--runs 5` for the
  numbers you plan to report.
- Always benchmark the `--release` build (the runner does this for you).
- Correctness is not checked here. Run `cargo test` as well, so an
  optimisation that makes things faster by making them wrong gets caught.
