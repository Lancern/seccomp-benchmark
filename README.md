# seccomp-benchmark

Performance benchmark of seccomp and ptrace.

## Install the Benchmark

### Docker Image

The benchmark is dockerized. You can pull the docker image `lancern/seccomp-bench:latest` and run it immediately.

### Build from Source

The benchmark is implemented in Rust. You need `cargo` installed to build Rust program from source. A detailed install guide of the Rust toolchain can be found at [https://rustup.rs/](https://rustup.rs/).

To get the benchmark source code and build the benchmark, execute the following commands in a Linux environment:

```shell
git clone https://github.com/Lancern/seccomp-benchmark.git
cd seccomp-benchmark
cargo build --release
```

After successfully built, the compiled executable file is available under `target/release`. The executable file is named `seccompbench`.

## Execute the Benchmark

The benchmark program takes two command line arguments: `--mode` and `--iter`. `--mode` specifies which target to benchmark. The possible values are `ptrace`, `seccomp` and `payload`. Usually you should not use `payload` directly; you should use `ptrace` or `seccomp` according to your benchmark target. The `--iter` specifies how many times the payload program will carry out system calls. The `--iter` arg is optional and the default value for it is 1000.

Here are some examples of invoking the benchmark program.

* To benchmark ptrace and make 100 system calls, execute: `seccompbench --mode ptrace --iter 100`;
* To benchmark ptrace and make 100000 system calls, execute:`seccompbench --mode ptrace --iter 100000`;
* To benchmark seccomp and make 100 system calls, execute: `seccompbench --mode seccomp --iter 100`;
* To benchmark seccomp and make 100000 system calls, execute:`seccompbench --mode seccomp --iter 100000`.

# Benchmark Result

Environment information:

* OS: Debian Stretch
* Linux Kernel Version: 4.9
* CPU: Intel Core i5 Quad-Core 2.4GHz

The following table shows the amount of wall clock time (in milliseconds) the payload program consumes under different benchmark configurations:

| iter.    | p1    | p2    | p3    | pa       | s1   | s2   | s3   | sa      | ovh.   |
| -------- | ----- | ----- | ----- | -------- | ---- | ---- | ---- | ------- | ------ |
| 10       | 32    | 22    | 27    | 27       | 2    | 1    | 1    | 1.33    | 1925%  |
| 100      | 56    | 45    | 42    | 47.67    | 1    | 1    | 2    | 1.33    | 3475%  |
| 1000     | 214   | 196   | 209   | 206.33   | 2    | 2    | 2    | 2       | 10217% |
| 10000    | 2004  | 2577  | 1944  | 2175     | 7    | 7    | 6    | 6.67    | 32525% |
| 100000   | 21178 | 17774 | 21719 | 20223.67 | 63   | 77   | 63   | 67.67   | 29787% |
| 1000000  | -     | -     | -     | -        | 513  | 542  | 551  | 535.33  | -      |
| 10000000 | -     | -     | -     | -        | 4964 | 5115 | 5004 | 5027.67 | -      |

iter is the number of times of system calls carried out by the payload program. p1, p2 and p3 represents three different runs of ptrace configurations; s1, s2 and s3 represents three different runs of seccomp configurations. pa is the average time of p1, p2 and p3 and similarly sa. The ovh is the overhead of ptrace with respect to seccomp, computed by the following formula:

$$
ovh = \frac{p - s}{s} \times 100\%
$$

where p and s are the times spent by the payload program under ptrace and seccomp benchmark configuration, respectively.
