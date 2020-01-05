extern crate error_chain;
extern crate libc;
extern crate nix;
extern crate seccomp_sys;
extern crate clap;

use std::ffi::{CString, CStr};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::str::FromStr;
use std::time::Instant;

use error_chain::ChainedError;

use clap::{App, Arg};

mod utils;

error_chain::error_chain! {
    types {
        Error, ErrorKind, ResultExt, Result;
    }

    links {
    }

    foreign_links {
        IoError(std::io::Error);
        NixError(nix::Error);
    }

    errors {
        NoSuchSyscall(name: String) {
            description("no such system call")
            display("no such system call: {}", name)
        }

        SeccompError {
            description("seccomp failed")
        }
    }
}

fn main() {
    match do_main() {
        Ok(..) => (),
        Err(e) => {
            eprintln!("{}", e.display_chain().to_string());
            std::process::exit(1);
        }
    }
}

fn do_main() -> Result<()> {
    fn parse_validator<T>(s: String) -> std::result::Result<(), String>
        where T: FromStr {
        T::from_str(&s).map(|_| ()).map_err(|_| String::from("invalid argument."))
    }

    let matches = App::new("seccompbench")
        .version("0.1")
        .author("Sirui Mu")
        .about("Benchmark seccomp against a similar implementation with ptrace")
        .arg(Arg::with_name("mode")
            .short("m")
            .long("mode")
            .value_name("SCHEME")
            .takes_value(true)
            .required(true)
            .possible_values(&["seccomp", "ptrace", "payload"])
            .help("which scheme to use?"))
        .arg(Arg::with_name("iterations")
            .short("i")
            .long("iter")
            .value_name("ITERATIONS")
            .takes_value(true)
            .required(false)
            .default_value("10000")
            .validator(parse_validator::<i32>)
            .help("how many system calls to make in the worker"))
        .get_matches();

    let mut options = BenchOptions::new(
        i32::from_str(matches.value_of("iterations").unwrap()).unwrap());
    options.add_disallowed_syscall("socket")
        .chain_err(|| "failed to add system call \"socket\"")?;
    options.add_disallowed_syscall("connect")
        .chain_err(|| "failed to add system call \"connect\"")?;
    options.add_disallowed_syscall("accept")
        .chain_err(|| "failed to add system call \"accept\"")?;
    options.add_disallowed_syscall("sendto")
        .chain_err(|| "failed to add system call \"sendto\"")?;
    options.add_disallowed_syscall("recvfrom")
        .chain_err(|| "failed to add system call \"recvfrom\"")?;
    options.add_disallowed_syscall("bind")
        .chain_err(|| "failed to add system call \"bind\"")?;
    options.add_disallowed_syscall("listen")
        .chain_err(|| "failed to add system call \"listen\"")?;

    match matches.value_of("mode").unwrap() {
        "seccomp" => bench_seccomp(&options),
        "ptrace" => bench_ptrace(&options),
        "payload" => payload_main(&options),
        _ => unreachable!()
    }
}

struct BenchOptions {
    iterations: i32,
    disallowed_syscalls: Vec<i64>,
}

impl BenchOptions {
    fn new(iterations: i32) -> Self {
        BenchOptions {
            iterations,
            disallowed_syscalls: Vec::new(),
        }
    }

    fn add_disallowed_syscall(&mut self, name: &str) -> Result<()> {
        let c_name = CString::new(name).unwrap();
        let res = unsafe { seccomp_sys::seccomp_syscall_resolve_name(c_name.as_ptr()) };
        if res == seccomp_sys::__NR_SCMP_ERROR {
            Err(Error::from(ErrorKind::SeccompError))
        } else {
            self.disallowed_syscalls.push(res as i64);
            Ok(())
        }
    }
}

fn bench_seccomp(options: &BenchOptions) -> Result<()> {
    match nix::unistd::fork()? {
        nix::unistd::ForkResult::Child => {
            let prog = CString::new(std::env::current_exe().unwrap().to_str().unwrap()).unwrap();
            let args = vec![
                prog.clone(),
                CString::new("--mode").unwrap(),
                CString::new("payload").unwrap(),
                CString::new("--iter").unwrap(),
                CString::new(options.iterations.to_string()).unwrap(),
            ];
            let arg_refs = args.iter().map(|s| s.as_c_str()).collect::<Vec<&CStr>>();

            // Setup seccomp environment.
            let seccomp = unsafe { seccomp_sys::seccomp_init(seccomp_sys::SCMP_ACT_ALLOW) };
            if seccomp.is_null() {
                return Err(Error::from("seccomp_init failed."));
            }

            for scid in &options.disallowed_syscalls {
                let ret = unsafe {
                    seccomp_sys::seccomp_rule_add_array(
                        seccomp, seccomp_sys::SCMP_ACT_KILL, *scid as i32,
                        0, std::ptr::null())
                };
                if ret < 0 {
                    return Err(Error::from(
                        format!("failed to install seccomp filter for syscall {}", scid)));
                }
            }

            let ret = unsafe { seccomp_sys::seccomp_load(seccomp) };
            if ret < 0 {
                return Err(Error::from("seccomp_load failed."));
            }

            unsafe { seccomp_sys::seccomp_release(seccomp) };

            nix::unistd::execv(&prog, &arg_refs)
                .chain_err(|| "execv failed")?;
            unreachable!()
        },
        nix::unistd::ForkResult::Parent { child } => {
            let start_time = Instant::now();

            let status = nix::sys::wait::waitpid(child, None)
                .chain_err(|| "wait for exit failed")?;
            match status {
                nix::sys::wait::WaitStatus::Exited(_, code) => {
                    println!("child exited with exit code {}", code);
                },
                nix::sys::wait::WaitStatus::Signaled(_, sig, _) => {
                    println!("child aborted by signal {}", sig);
                },
                _ => unreachable!()
            };

            let end_time = Instant::now();
            let elapsed = end_time - start_time;
            println!("benchmark finished within {} ms.", elapsed.as_millis());

            Ok(())
        }
    }
}

fn bench_ptrace(options: &BenchOptions) -> Result<()> {
    match nix::unistd::fork()? {
        nix::unistd::ForkResult::Child => {
            let prog = CString::new(std::env::current_exe().unwrap().to_str().unwrap()).unwrap();
            let args = vec![
                prog.clone(),
                CString::new("--mode").unwrap(),
                CString::new("payload").unwrap(),
                CString::new("--iter").unwrap(),
                CString::new(options.iterations.to_string()).unwrap(),
            ];
            let arg_refs = args.iter().map(|s| s.as_c_str()).collect::<Vec<&CStr>>();

            nix::sys::ptrace::traceme()
                .chain_err(|| "ptrace traceme failed")?;
            nix::unistd::execv(&prog, &arg_refs)
                .chain_err(|| "execv failed")?;

            unreachable!()
        },
        nix::unistd::ForkResult::Parent { child } => {
            let start_time = Instant::now();

            // Sync with `execve`.
            nix::sys::wait::waitpid(child, None)
                .chain_err(|| "failed to sync traceme")?;
            nix::sys::ptrace::setoptions(child, nix::sys::ptrace::Options::PTRACE_O_EXITKILL)
                .chain_err(|| "failed to set PTRACE_O_EXITKILL flag")?;

            let mut trapped = false;
            loop {
                nix::sys::ptrace::syscall(child, None)
                    .chain_err(|| "ptrace syscall failed")?;
                let status = nix::sys::wait::waitpid(child, None)
                    .chain_err(|| "waitpid for syscall failed")?;
                match status {
                    nix::sys::wait::WaitStatus::Exited(_, code) => {
                        println!("child exited with exit code {}", code);
                        break;
                    },
                    nix::sys::wait::WaitStatus::Signaled(_, sig, _) => {
                        println!("child aborted by signal {}", sig);
                        break;
                    },
                    _ => {
                        if trapped {
                            trapped = false;
                            continue;
                        }

                        trapped = true;
                        let regs = nix::sys::ptrace::getregs(child)
                            .chain_err(|| "ptrace getregs failed")?;
                        let syscall_id = utils::bitcast::<u64, i64>(regs.orig_rax);
                        if options.disallowed_syscalls.contains(&syscall_id) {
                            println!("child called disallowed syscall: {}", syscall_id);
                            nix::sys::signal::kill(child, nix::sys::signal::Signal::SIGKILL)
                                .chain_err(|| "failed to kill child process")?;
                            nix::sys::wait::waitpid(child, None)
                                .chain_err(|| "waitpid for exit failed")?;
                            break;
                        }
                    }
                }
            }

            let end_time = Instant::now();
            let elapsed = end_time - start_time;
            println!("benchmark finished within {} ms.", elapsed.as_millis());
            Ok(())
        }
    }
}

fn payload_main(options: &BenchOptions) -> Result<()> {
    println!("payload started");

    let file = File::create("seccompbench.tmp")
        .chain_err(|| "failed to create temporary benchmark file")?;
    let file_fp = file.as_raw_fd();
    let _delete_file_guard = utils::defer(|| {
        std::fs::remove_file("seccompbench.tmp").ok();
    });

    for _ in 0..options.iterations {
        nix::fcntl::fcntl(file_fp, nix::fcntl::FcntlArg::F_GETFD)
            .chain_err(|| "fcntl failed")?;
    }

    println!("payload finished");
    Ok(())
}
