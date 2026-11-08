//! 图嘟嘟服务端启动器。
//!
//! 干的活和常见发行包里的启动器一样：
//!
//! 1. 读环境文件，只补父进程没有的变量（同名时父进程优先）；
//! 2. 拉起主程序，输出沿用启动器的，交给服务管理器收日志；
//! 3. 主程序以 75 退出，说明新版本已经装好，重新拉起；
//! 4. 收到 TERM / INT / HUP 先转发给主程序，等它自己收尾。
//!
//! 用法：
//!
//! ```text
//! tududu-launcher [--env-file PATH] [--] [主程序参数...]
//! ```

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};

use anyhow::{Context, bail};

/// 主程序用这个退出码告诉启动器：新版本装好了，重启。
const RESTART_CODE: i32 = 75;

/// 子进程 pid，信号来的时候转发给它。0 表示当前没有子进程。
static CHILD: AtomicI32 = AtomicI32::new(0);

fn main() {
    if let Err(err) = run() {
        eprintln!("启动器出错：{err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut args = parse_args(std::env::args().skip(1).collect())?;

    if args.help {
        print_help();
        return Ok(());
    }

    // 主程序必须带子命令，没给就用 TUDUDU_ARGS，再没有就默认 serve
    if args.program_args.is_empty() {
        if let Ok(extra) = std::env::var("TUDUDU_ARGS") {
            args.program_args = extra
                .split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
        }
    }

    if args.program_args.is_empty() {
        args.program_args = vec!["serve".to_owned()];
    }

    if let Some(dir) = args.workdir {
        std::env::set_current_dir(&dir)
            .with_context(|| format!("进不去工作目录 {}", dir.display()))?;
    }

    let bin = resolve_bin(args.bin)?;
    if !bin.is_file() {
        bail!("找不到主程序 {}", bin.display());
    }

    // 环境文件默认取工作目录下的 .env；显式指定的必须能读
    let env_file = match args.env_file {
        Some(path) => Some(path),
        None => Path::new(".env").is_file().then(|| PathBuf::from(".env")),
    };

    if let Some(path) = &env_file {
        let applied =
            load_env_file(path).with_context(|| format!("读取环境文件 {} 失败", path.display()))?;
        println!(
            "已加载环境文件：{}（补了 {applied} 个变量，父进程已有的跳过）",
            path.display()
        );
    }

    println!(
        "启动器托管：{} {}",
        bin.display(),
        if args.program_args.is_empty() {
            "（无参数）".to_owned()
        } else {
            args.program_args.join(" ")
        }
    );

    supervise(&bin, &args.program_args)
}

/// 起主程序、等它退出；需要重启就再来一轮。
fn supervise(bin: &Path, program_args: &[String]) -> anyhow::Result<()> {
    install_signal_forwarding()?;

    loop {
        let mut child = Command::new(bin)
            .args(program_args)
            // 告诉主程序自己是被托管的，后台重启时才知道能拉起来
            .env("TUDUDU_LAUNCHER", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("拉起主程序 {} 失败", bin.display()))?;

        CHILD.store(child.id() as i32, Ordering::SeqCst);
        let status = child.wait();
        CHILD.store(0, Ordering::SeqCst);

        let code = status
            .as_ref()
            .ok()
            .and_then(|status| status.code())
            .unwrap_or(-1);

        match status {
            Ok(status) if status.code() == Some(RESTART_CODE) => {
                println!("主程序报告新版本已安装，重新启动");
            }
            Ok(status) => {
                if !status.success() {
                    println!("主程序退出，代码 {code}");
                }
                return Ok(());
            }
            Err(err) => bail!("等待主程序退出失败：{err}"),
        }
    }
}

/// 把停止信号转给主程序，主程序自己收尾后这边跟着退出。
fn install_signal_forwarding() -> anyhow::Result<()> {
    use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;

    let mut signals = Signals::new([SIGTERM, SIGINT, SIGHUP])?;

    std::thread::spawn(move || {
        for signal in signals.forever() {
            let pid = CHILD.load(Ordering::SeqCst);
            if pid > 0 {
                forward(pid, signal);
            } else {
                // 没有子进程就直接退出，别留个不退的启动器
                std::process::exit(0);
            }
        }
    });

    Ok(())
}

/// 转发信号。用 extern 直接声明，避免为了一次调用把 libc 引进依赖树。
fn forward(pid: i32, signal: i32) {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }

    unsafe {
        kill(pid, signal);
    }
}

fn resolve_bin(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(bin) = explicit {
        return Ok(bin);
    }

    // 默认用启动器同目录下的 tududu，发行包里两个文件放一起
    let mut dir = std::env::current_exe().context("拿不到启动器自身路径")?;
    dir.pop();
    Ok(dir.join("tududu"))
}

struct Args {
    bin: Option<PathBuf>,
    workdir: Option<PathBuf>,
    env_file: Option<PathBuf>,
    program_args: Vec<String>,
    help: bool,
}

fn parse_args(argv: Vec<String>) -> anyhow::Result<Args> {
    let mut args = Args {
        bin: std::env::var_os("TUDUDU_BIN").map(PathBuf::from),
        workdir: std::env::var_os("TUDUDU_WORKDIR").map(PathBuf::from),
        env_file: None,
        program_args: Vec::new(),
        help: false,
    };

    let mut index = 0;
    while index < argv.len() {
        let current = argv[index].as_str();

        if current == "--env-file" {
            index += 1;
            let value = argv.get(index).context("--env-file 后面要跟路径")?;
            args.env_file = Some(PathBuf::from(value));
        } else if let Some(value) = current.strip_prefix("--env-file=") {
            args.env_file = Some(PathBuf::from(value));
        } else if current == "-h" || current == "--help" {
            args.help = true;
        } else if current == "--" {
            args.program_args = argv[index + 1..].to_vec();
            return Ok(args);
        } else {
            bail!("未知参数 {current}，主程序参数请放在 -- 之后");
        }

        index += 1;
    }

    Ok(args)
}

/// 解析 dotenv 格式，只补充当前进程还没有的变量。
fn load_env_file(path: &Path) -> anyhow::Result<usize> {
    let text = std::fs::read_to_string(path)?;
    let mut applied = 0;

    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim_start_matches('\u{feff}').trim_end();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            // 只报行号不回显内容，里面可能写着密钥
            bail!("第 {} 行缺少等号", number + 1);
        };

        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            bail!("第 {} 行的键名不合法", number + 1);
        }

        if std::env::var_os(key).is_some() {
            continue;
        }

        // 去掉成对引号，其余原样保留
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|rest| rest.strip_suffix('\''))
            })
            .unwrap_or(value);

        // SAFETY: 此时还没有拉起任何线程，进程是单线程的；key 已校验只含字母数字下划线
        unsafe { std::env::set_var(key, value) };
        applied += 1;
    }

    Ok(applied)
}

fn print_help() {
    let lines = [
        "图嘟嘟服务端启动器",
        "",
        "用法：tududu-launcher [--env-file PATH] [--] [主程序参数...]",
        "不给参数时默认执行 serve，也可以用 TUDUDU_ARGS 覆盖",
        "",
        "环境变量：",
        "  TUDUDU_BIN       主程序路径，默认用启动器同目录下的 tududu",
        "  TUDUDU_WORKDIR   主程序工作目录，默认当前目录",
        "  TUDUDU_ARGS      主程序参数，默认 serve",
        "",
        "说明：",
        "  环境文件只补充缺失变量，父进程已有的同名变量优先",
        "  主程序以 75 退出表示新版本已装好，启动器会重新拉起",
        "  收到 TERM / INT / HUP 时先转发给主程序，等它自己收尾",
    ];

    for line in lines {
        println!("{line}");
    }
}
