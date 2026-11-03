#!/usr/bin/env python3
"""Compare amfrustlox against the reference Java interpreter (jlox).

Project goal: amfrustlox must be at least 10% faster than jlox, i.e.
(amfrustlox time) / (jlox time) <= 0.90 on the benchmark programs.

Each program in benchmarks/lox/ prints its own elapsed time (measured with the
interpreter's `clock()`) as its last line, so JVM start-up and process spawning
are NOT part of the numbers. The one exception is `zoo_batch`, which runs for a
fixed 10 seconds and prints how many batches it finished; there, more is better
and the ratio is reported as jlox_batches / amfrustlox_batches so that for every
row "lower ratio = amfrustlox is faster".

Typical use (from the amfrustlox directory):

    # first time: build jlox from a clone of munificent/craftinginterpreters and
    # save its numbers so later runs don't need Java at all
    python benchmarks/run_benchmarks.py --jlox-src ../craftinginterpreters --save benchmarks/baseline.json

    # while optimising: re-run only amfrustlox against the saved jlox numbers
    python benchmarks/run_benchmarks.py --jlox-from benchmarks/baseline.json

Only the Python standard library is needed.
"""

import argparse
import json
import math
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parent
LOX_DIR = HERE / "lox"
BUILD_DIR = HERE / ".build"

GOAL_RATIO = 0.90  # amfrustlox must take at most 90% of jlox's time
THROUGHPUT = {"zoo_batch"}  # prints a batch count (higher is better)


def die(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(2)


# --------------------------------------------------------------------------
# Locating / building the two interpreters
# --------------------------------------------------------------------------

def build_rust():
    print("Building amfrustlox (cargo build --release)...", file=sys.stderr)
    if shutil.which("cargo") is None:
        die("cargo not found on PATH")
    subprocess.run(["cargo", "build", "--release"], cwd=PROJECT, check=True)


def rust_binary(explicit):
    if explicit:
        path = Path(explicit)
    else:
        name = "amfrustlox.exe" if sys.platform == "win32" else "amfrustlox"
        path = PROJECT / "target" / "release" / name
    if not path.is_file():
        die(f"amfrustlox binary not found at {path} (build it, or pass --rust-bin)")
    return path


def compile_jlox(src):
    """Compile jlox from a craftinginterpreters clone (or its `java` folder)."""
    src = Path(src)
    for base in (src / "java", src):
        java_dir = base / "com" / "craftinginterpreters" / "lox"
        if java_dir.is_dir():
            break
    else:
        die(f"no com/craftinginterpreters/lox folder found under {src}")
    javac = shutil.which("javac")
    if javac is None:
        die("javac not found on PATH (a JDK is needed to compile jlox)")
    out = BUILD_DIR / "jlox"
    out.mkdir(parents=True, exist_ok=True)
    print(f"Compiling jlox from {java_dir} ...", file=sys.stderr)
    sources = [str(p) for p in sorted(java_dir.glob("*.java"))]
    subprocess.run([javac, "-d", str(out), *sources], check=True)
    return out


def jlox_command(classes_dir):
    java = shutil.which("java")
    if java is None:
        die("java not found on PATH")
    return [java, "-cp", str(classes_dir), "com.craftinginterpreters.lox.Lox"]


# --------------------------------------------------------------------------
# Running
# --------------------------------------------------------------------------

def run_once(command, name, timeout):
    """Returns (metric, error). metric is seconds, or batches for throughput."""
    program = LOX_DIR / f"{name}.lox"
    try:
        started = time.perf_counter()
        proc = subprocess.run(
            [*command, str(program)], capture_output=True, text=True, timeout=timeout
        )
        wall = time.perf_counter() - started
    except subprocess.TimeoutExpired:
        return None, f"timed out after {timeout}s"
    if proc.returncode != 0:
        detail = proc.stderr.strip().splitlines()[-1:] or [""]
        return None, f"exit code {proc.returncode}: {detail[0][:120]}"

    lines = [line.strip() for line in proc.stdout.splitlines() if line.strip()]
    try:
        if name in THROUGHPUT:
            return float(lines[-2]), None  # batch count precedes the elapsed time
        return float(lines[-1]), None  # elapsed seconds printed by the program
    except (IndexError, ValueError):
        if name in THROUGHPUT:
            return None, "could not read the batch count from the output"
        return wall, None  # fall back to wall-clock time


def measure(label, command, names, runs, timeout):
    results = {}
    for name in names:
        values = []
        for i in range(runs):
            print(f"  {label:8} {name:16} run {i + 1}/{runs}", end="\r", file=sys.stderr)
            metric, error = run_once(command, name, timeout)
            if error:
                print(f"\n  {label} {name}: {error}", file=sys.stderr)
                values = []
                break
            values.append(metric)
        results[name] = values
        print(" " * 60, end="\r", file=sys.stderr)
    return results


# --------------------------------------------------------------------------
# Reporting
# --------------------------------------------------------------------------

def slowdown(name, jlox_values, rust_values):
    """amfrustlox relative cost: < 1.0 means amfrustlox is faster."""
    jlox, rust = statistics.median(jlox_values), statistics.median(rust_values)
    if name in THROUGHPUT:
        return jlox / rust
    return rust / jlox


def report(names, jlox, rust):
    header = f"{'benchmark':<16}{'jlox':>10}{'amfrustlox':>12}{'ratio':>9}  verdict"
    print(header)
    print("-" * len(header))
    ratios = []
    for name in names:
        j, r = jlox.get(name), rust.get(name)
        if not j or not r:
            print(f"{name:<16}{'-':>10}{'-':>12}{'-':>9}  FAILED / missing")
            continue
        ratio = slowdown(name, j, r)
        ratios.append(ratio)
        medians = f"{statistics.median(j):.2f}", f"{statistics.median(r):.2f}"
        verdict = "ok" if ratio <= GOAL_RATIO else (
            "slower" if ratio > 1.0 else "faster, but < 10%"
        )
        label = f"{name} *" if name in THROUGHPUT else name
        print(f"{label:<16}{medians[0]:>10}{medians[1]:>12}{ratio:>8.2f}x  {verdict}")
    if ratios:
        geomean = math.exp(sum(math.log(x) for x in ratios) / len(ratios))
        passing = sum(1 for x in ratios if x <= GOAL_RATIO)
        print("-" * len(header))
        print(
            f"geometric mean ratio: {geomean:.2f}x   "
            f"meeting the 10% goal (<= {GOAL_RATIO:.2f}x): {passing}/{len(ratios)}"
        )
        print("times are seconds; ratio = amfrustlox / jlox, lower is better")
        if any(n in THROUGHPUT for n in names):
            print("* batches completed in 10s (higher is better), ratio = jlox / amfrustlox")
    return ratios


# --------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--runs", type=int, default=3, help="runs per benchmark (default 3, median is used)")
    parser.add_argument("--only", help="comma-separated benchmark names, e.g. fib,zoo")
    parser.add_argument("--timeout", type=int, default=300, help="seconds per run (default 300)")
    parser.add_argument("--no-build", action="store_true", help="do not run cargo build --release first")
    parser.add_argument("--rust-bin", help="path to the amfrustlox binary (default target/release/amfrustlox)")
    parser.add_argument("--jlox-src", help="path to a munificent/craftinginterpreters clone; jlox is compiled from it")
    parser.add_argument("--jlox-classes", help="directory with already compiled jlox classes")
    parser.add_argument("--jlox-from", help="reuse jlox numbers from a file written by --save (no Java needed)")
    parser.add_argument("--save", help="write all measurements to this JSON file")
    parser.add_argument("--strict", action="store_true", help="exit with status 1 if any benchmark misses the 10%% goal")
    args = parser.parse_args()

    available = sorted(p.stem for p in LOX_DIR.glob("*.lox"))
    names = args.only.split(",") if args.only else available
    unknown = [n for n in names if n not in available]
    if unknown:
        die(f"unknown benchmark(s): {', '.join(unknown)}. Available: {', '.join(available)}")

    if not args.no_build and not args.rust_bin:
        build_rust()
    rust_cmd = [str(rust_binary(args.rust_bin))]

    if args.jlox_from:
        saved = json.loads(Path(args.jlox_from).read_text())
        jlox = saved["jlox"]
    else:
        if args.jlox_classes:
            classes = Path(args.jlox_classes)
        elif args.jlox_src:
            classes = compile_jlox(args.jlox_src)
        else:
            die("need a jlox source: --jlox-src, --jlox-classes or --jlox-from (see --help)")
        print("Running jlox...", file=sys.stderr)
        jlox = measure("jlox", jlox_command(classes), names, args.runs, args.timeout)

    print("Running amfrustlox...", file=sys.stderr)
    rust = measure("rust", rust_cmd, names, args.runs, args.timeout)

    print()
    ratios = report(names, jlox, rust)

    if args.save:
        Path(args.save).write_text(json.dumps({"jlox": jlox, "amfrustlox": rust}, indent=2))
        print(f"\nsaved measurements to {args.save}")

    if args.strict and (len(ratios) < len(names) or any(x > GOAL_RATIO for x in ratios)):
        sys.exit(1)


if __name__ == "__main__":
    main()
