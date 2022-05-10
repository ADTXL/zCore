﻿use crate::{
    dir,
    download::{git_clone, wget},
    CommandExt, ALPINE_ROOTFS_VERSION, ALPINE_WEBSITE,
};
use dircpy::copy_dir;
use std::{
    ffi::{OsStr, OsString},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Args)]
pub(super) struct Arch {
    #[clap(subcommand)]
    command: ArchCommands,
}

#[derive(Subcommand)]
enum ArchCommands {
    #[clap(name = "riscv64")]
    Riscv64,
    #[clap(name = "x86_64")]
    X86_64,
}

impl Arch {
    /// 构造启动内存文件系统 rootfs。
    ///
    /// 将在文件系统中放置必要的库文件，并下载用于交叉编译的工具链。
    pub fn rootfs(&self, clear: bool) {
        self.wget_alpine();
        match self.command {
            ArchCommands::Riscv64 => {
                const DIR: &str = "riscv_rootfs";
                const ARCH: &str = "riscv64";

                let dir = Path::new(DIR);
                if dir.is_dir() && !clear {
                    return;
                }
                dir::clear(dir).unwrap();
                let tar = dir::detect(&format!("prebuilt/linux/{ARCH}"), "minirootfs").unwrap();
                Tar::xf(&tar, Some(DIR))
                    .args(&["--strip-components", "1"])
                    .join();
                Command::new("ln")
                    .args(&["-s", "busybox", "riscv_rootfs/bin/ls"])
                    .status()
                    .unwrap()
                    .exit_ok()
                    .expect("FAILED: ln -s busybox riscv_rootfs/bin/ls");
            }
            ArchCommands::X86_64 => {
                const DIR: &str = "rootfs";
                const ARCH: &str = "x86_64";

                let dir = Path::new(DIR);
                if dir.is_dir() && !clear {
                    return;
                }
                dir::clear(DIR).unwrap();
                let tar = dir::detect(&format!("prebuilt/linux/{ARCH}"), "minirootfs").unwrap();
                Tar::xf(&tar, Some(DIR)).join();
                // libc-libos.so (convert syscall to function call) is from https://github.com/rcore-os/musl/tree/rcore
                fs::copy(
                    "prebuilt/linux/libc-libos.so",
                    format!("{DIR}/lib/ld-musl-{ARCH}.so.1"),
                )
                .unwrap();
                {
                    const TEST_DIR: &str = "linux-syscall/test";
                    const DEST_DIR: &str = "rootfs/bin/";
                    // for linux syscall tests
                    fs::read_dir(TEST_DIR)
                        .unwrap()
                        .filter_map(|res| res.ok())
                        .map(|entry| entry.path())
                        .filter(|path| path.extension().map_or(false, |ext| ext == OsStr::new("c")))
                        .for_each(|c| {
                            let o = format!(
                                "{DEST_DIR}/{}",
                                c.file_prefix().and_then(|s| s.to_str()).unwrap()
                            );
                            Command::new("gcc")
                                .arg(&c)
                                .args(&["-o", &o])
                                .arg("-Wl,--dynamic-linker=/lib/ld-musl-x86_64.so.1")
                                .status()
                                .unwrap()
                                .exit_ok()
                                .expect("FAILED: gcc {c:?}");
                        });
                }
            }
        }
    }

    /// 将 libc-test 放入 rootfs。
    pub fn libc_test(&self) {
        self.rootfs(false);
        git_clone(
            "https://github.com/rcore-os/libc-test.git",
            "ignored/libc-test",
        );
        match self.command {
            ArchCommands::Riscv64 => {
                const DIR: &str = "riscv_rootfs/libc-test";
                const PRE: &str = "riscv_rootfs/libc-test-prebuild";

                fs::rename(DIR, PRE).unwrap();
                copy_dir("ignored/libc-test", DIR).unwrap();
                fs::copy(format!("{DIR}/config.mak.def"), format!("{DIR}/config.mak")).unwrap();
                Make::new(None)
                    .env("ARCH", "riscv64")
                    .env("CROSS_COMPILE", "riscv64-linux-musl-")
                    .env("PATH", riscv64_linux_musl_cross())
                    .current_dir(DIR)
                    .join();
                fs::copy(
                    format!("{PRE}/functional/tls_align-static.exe"),
                    format!("{DIR}/src/functional/tls_align-static.exe"),
                )
                .unwrap();
                dir::rm(PRE).unwrap();
            }
            ArchCommands::X86_64 => {
                const DIR: &str = "rootfs/libc-test";

                dir::rm(DIR).unwrap();
                copy_dir("ignored/libc-test", DIR).unwrap();
                fs::copy(format!("{DIR}/config.mak.def"), format!("{DIR}/config.mak")).unwrap();
                fs::OpenOptions::new()
                    .append(true)
                    .open(format!("{DIR}/config.mak"))
                    .unwrap()
                    .write_all(b"CC := musl-gcc\nAR := ar\nRANLIB := ranlib")
                    .unwrap();
                Make::new(None).current_dir(DIR).join();
            }
        }
    }

    /// 生成镜像。
    pub fn image(&self) {
        self.rootfs(false);
        let image = match self.command {
            ArchCommands::Riscv64 => {
                const ARCH: &str = "riscv64";

                let image = format!("zCore/{ARCH}.img");
                fuse("riscv_rootfs", &image);
                image
            }
            ArchCommands::X86_64 => {
                const ARCH: &str = "x86_64";
                const TMP_ROOTFS: &str = "/tmp/rootfs";
                const ROOTFS_LIB: &str = "rootfs/lib";

                // ld-musl-x86_64.so.1 替换为适用 bare-matel 的版本
                dir::clear(TMP_ROOTFS).unwrap();
                let tar = dir::detect(&format!("prebuilt/linux/{ARCH}"), "minirootfs").unwrap();
                Tar::xf(&tar, Some(TMP_ROOTFS)).join();
                dir::clear(ROOTFS_LIB).unwrap();
                fs::copy(
                    format!("{TMP_ROOTFS}/lib/ld-musl-x86_64.so.1"),
                    format!("{ROOTFS_LIB}/ld-musl-x86_64.so.1"),
                )
                .unwrap();

                let image = format!("zCore/{ARCH}.img");
                fuse("rootfs", &image);
                fs::copy(
                    "prebuilt/linux/libc-libos.so",
                    format!("{ROOTFS_LIB}/ld-musl-x86_64.so.1"),
                )
                .unwrap();
                image
            }
        };
        Command::new("qemu-img")
            .args(&["resize", &image, "+5M"])
            .status()
            .unwrap()
            .exit_ok()
            .expect("FAILED: qemu-img resize");
    }

    /// 获取 alpine 镜像。
    fn wget_alpine(&self) {
        let (local_path, web_url) = match self.command {
            ArchCommands::Riscv64 => {
                const ARCH: &str = "riscv64";
                const FILE_NAME: &str = "minirootfs.tar.xz";
                const WEB_URL: &str = "https://github.com/rcore-os/libc-test-prebuilt/releases/download/0.1/prebuild.tar.xz";

                let local_path = PathBuf::from(format!("prebuilt/linux/{ARCH}/{FILE_NAME}"));
                if local_path.exists() {
                    return;
                }
                (local_path, WEB_URL.into())
            }
            ArchCommands::X86_64 => {
                const ARCH: &str = "x86_64";
                const FILE_NAME: &str = "minirootfs.tar.gz";

                let local_path = PathBuf::from(format!("prebuilt/linux/{ARCH}/{FILE_NAME}"));
                if local_path.exists() {
                    return;
                }
                (
                    local_path,
                    format!(
                        "{ALPINE_WEBSITE}/{ARCH}/alpine-minirootfs-{ALPINE_ROOTFS_VERSION}-{ARCH}.tar.gz"
                    ),
                )
            }
        };

        fs::create_dir_all(local_path.parent().unwrap()).unwrap();
        wget(&web_url, &local_path);
    }
}

struct Make(Command);

impl AsRef<Command> for Make {
    fn as_ref(&self) -> &Command {
        &self.0
    }
}

impl AsMut<Command> for Make {
    fn as_mut(&mut self) -> &mut Command {
        &mut self.0
    }
}

impl CommandExt for Make {}

impl Make {
    fn new(j: Option<usize>) -> Self {
        let mut make = Self(Command::new("make"));
        match j {
            Some(0) => {}
            Some(j) => {
                make.arg(format!("-j{j}"));
            }
            None => {
                make.arg("-j");
            }
        }
        make
    }
}

struct Tar(Command);

impl AsRef<Command> for Tar {
    fn as_ref(&self) -> &Command {
        &self.0
    }
}

impl AsMut<Command> for Tar {
    fn as_mut(&mut self) -> &mut Command {
        &mut self.0
    }
}

impl CommandExt for Tar {}

impl Tar {
    fn xf(src: &impl AsRef<OsStr>, dst: Option<&str>) -> Self {
        let mut cmd = Command::new("tar");
        cmd.arg("xf").arg(src);
        if let Some(dst) = dst {
            cmd.arg("-C").arg(dst);
        }
        Self(cmd)
    }
}

/// 下载 riscv64-musl 工具链。
fn riscv64_linux_musl_cross() -> OsString {
    const DIR: &str = "ignored";
    const NAME: &str = "riscv64-linux-musl-cross";
    let dir = format!("{DIR}/{NAME}");
    let tgz = format!("{dir}.tgz");

    wget(&format!("https://musl.cc/{NAME}.tgz"), &tgz);
    dir::rm(&dir).unwrap();
    Tar::xf(&tgz, Some(DIR)).join();

    // 将交叉工具链加入 PATH 环境变量
    let mut path = OsString::new();
    if let Ok(current) = std::env::var("PATH") {
        path.push(current);
        path.push(":");
    }
    path.push(std::env::current_dir().unwrap());
    path.push("/ignored/riscv64-linux-musl-cross/bin");
    path
}

/// 制作镜像。
fn fuse(dir: impl AsRef<Path>, image: impl AsRef<Path>) {
    use rcore_fs::vfs::FileSystem;
    use rcore_fs_fuse::zip::zip_dir;
    use rcore_fs_sfs::SimpleFileSystem;
    use std::sync::{Arc, Mutex};

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(image)
        .expect("failed to open image");
    const MAX_SPACE: usize = 0x1000 * 0x1000 * 1024; // 1G
    let fs = SimpleFileSystem::create(Arc::new(Mutex::new(file)), MAX_SPACE)
        .expect("failed to create sfs");
    zip_dir(dir.as_ref(), fs.root_inode()).expect("failed to zip fs");
}
