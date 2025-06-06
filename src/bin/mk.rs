use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(version, about)]
struct Options {
    /// Force the created entry to be a file.
    #[clap(short, long)]
    file: bool,

    /// Force the created entry to be a directory.
    #[clap(short, long)]
    directory: bool,

    /// Overwrite existing entries.
    #[clap(short, long)]
    overwrite: bool,

    /// Force the created file to be executable.
    #[clap(short = 'x', long)]
    executable: bool,

    /// The path to make.
    ///
    /// Entry type is inferred from if the path has an extension or not. Paths with final item starting with '.' are inferred as directories.
    path: PathBuf,
}

const EXECUTABLE_EXTENSIONS: &[&str] = &[
    "exe", "bat", "cmd", "com", "ps1", "vbs", "msi", "scr", // Windows
    "sh", "bash", "zsh", "ksh", "run", "bin", "cgi", "py", "pl", "rb", "php", // Unix-like
    "jar", "appimage", "apk", "wasm", "pyz", // Cross-platform
];

fn main() -> anyhow::Result<()> {
    let dir = std::env::current_dir()?;
    let options = Options::parse();

    if atty::is(atty::Stream::Stdin) {
        run(dir, options, &[][..])
    } else {
        run(dir, options, std::io::stdin().lock())
    }
}

fn run<R: std::io::Read>(
    root: impl AsRef<Path>,
    options: Options,
    mut stdin: R,
) -> anyhow::Result<()> {
    let path = root.as_ref().join(&options.path);

    let is_file = match (options.file, options.directory) {
        (false, false) => path.extension().is_some(),
        (true, false) => true,
        (false, true) => false,
        (true, true) => anyhow::bail!("Cannot force both file and directory"),
    };

    anyhow::ensure!(
        options.overwrite || !std::fs::exists(&path)?,
        "Entry {} already exists",
        options.path.display()
    );

    if !is_file {
        anyhow::ensure!(!options.executable, "Cannot make directory executable");

        let is_stdin_empty = stdin.read(&mut [0; 1][..])? == 0;
        anyhow::ensure!(is_stdin_empty, "Cannot create directory with stdin data");

        std::fs::create_dir_all(path)?;
        return Ok(());
    }

    std::fs::create_dir_all(path.parent().expect("joined with root"))?;
    let mut file = std::fs::File::create(&path)?;
    std::io::copy(&mut stdin, &mut file)?;

    let mut is_executable = options.executable;
    if let Some(ext) = path.extension() {
        if let Some(as_str) = ext.to_str() {
            is_executable |= EXECUTABLE_EXTENSIONS.contains(&as_str);
        }
    }

    if is_executable {
        make_executable(&path)?;
    }

    Ok(())
}

#[cfg(unix)]
fn make_executable(file: impl AsRef<Path>) -> anyhow::Result<()> {
    let output = std::process::Command::new("chmod")
        .arg("+x")
        .arg(file.as_ref())
        .output()?;
    anyhow::ensure!(
        output.status.success(),
        "Unsuccessful in setting file executable"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::TempDir;

    fn run_command(cmd: &str) -> anyhow::Result<TempDir> {
        let dir = tempfile::tempdir()?;
        run_command_in(dir.path(), cmd)?;
        Ok(dir)
    }

    fn run_command_in(dir: &Path, cmd: &str) -> anyhow::Result<()> {
        let options = Options::try_parse_from(cmd.split(" "))?;
        super::run(dir, options, &[][..])?;
        Ok(())
    }

    fn run_command_stdin(cmd: &str, stdin: &str) -> anyhow::Result<TempDir> {
        let dir = tempfile::tempdir()?;
        let options = Options::try_parse_from(cmd.split(" "))?;
        super::run(dir.path(), options, stdin.as_bytes())?;
        Ok(dir)
    }

    #[test]
    fn creates_root_file() -> anyhow::Result<()> {
        let dir = run_command("mk foo.txt")?;

        assert!(std::fs::metadata(dir.path().join("foo.txt"))?.is_file());
        Ok(())
    }

    #[test]
    fn creates_root_dir() -> anyhow::Result<()> {
        let dir = run_command("mk foo")?;

        assert!(std::fs::metadata(dir.path().join("foo"))?.is_dir());
        Ok(())
    }

    #[test]
    fn creates_nested_file() -> anyhow::Result<()> {
        let dir = run_command("mk foo/bar/baz.txt")?;

        assert!(std::fs::metadata(dir.path().join("foo"))?.is_dir());
        assert!(std::fs::metadata(dir.path().join("foo/bar"))?.is_dir());
        assert!(std::fs::metadata(dir.path().join("foo/bar/baz.txt"))?.is_file());
        Ok(())
    }

    #[test]
    fn creates_nested_dir() -> anyhow::Result<()> {
        let dir = run_command("mk foo/bar/baz")?;

        assert!(std::fs::metadata(dir.path().join("foo"))?.is_dir());
        assert!(std::fs::metadata(dir.path().join("foo/bar"))?.is_dir());
        assert!(std::fs::metadata(dir.path().join("foo/bar/baz"))?.is_dir());
        Ok(())
    }

    #[test]
    fn creates_dot_dir() -> anyhow::Result<()> {
        let dir = run_command("mk .dir")?;

        assert!(std::fs::metadata(dir.path().join(".dir"))?.is_dir());
        Ok(())
    }

    #[test]
    fn creates_file_with_dash_f() -> anyhow::Result<()> {
        let dir = run_command("mk -f dir_like")?;

        assert!(std::fs::metadata(dir.path().join("dir_like"))?.is_file());
        Ok(())
    }

    #[test]
    fn creates_dir_with_dash_d() -> anyhow::Result<()> {
        let dir = run_command("mk -d file_like.txt")?;

        assert!(std::fs::metadata(dir.path().join("file_like.txt"))?.is_dir());
        Ok(())
    }

    #[test]
    fn errors_if_already_exists() -> anyhow::Result<()> {
        let dir = run_command("mk foo.txt")?;

        assert!(run_command_in(dir.path(), "mk foo.txt").is_err());
        Ok(())
    }

    #[test]
    fn overwrites_existing_file() -> anyhow::Result<()> {
        let dir = run_command("mk foo.txt")?;

        run_command_in(dir.path(), "mk -o foo.txt")?;
        Ok(())
    }

    #[test]
    fn writes_stdin_to_created_file() -> anyhow::Result<()> {
        let dir = run_command_stdin("mk foo.txt", "some contents")?;

        assert_eq!(
            std::fs::read_to_string(dir.path().join("foo.txt"))?,
            "some contents"
        );
        Ok(())
    }

    #[test]
    fn errors_with_stdin_for_dir() -> anyhow::Result<()> {
        assert!(run_command_stdin("mk foo", "some contents").is_err());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn marks_file_executable() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = run_command("mk foo.sh")?;

        let file = std::fs::File::open(dir.path().join("foo.sh"))?;
        assert_eq!(file.metadata()?.permissions().mode() & 0o111, 0o111);

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn does_not_make_normal_file_executable() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = run_command("mk foo.txt")?;

        let file = std::fs::File::open(dir.path().join("foo.txt"))?;
        assert_eq!(file.metadata()?.permissions().mode() & 0o111, 0o000);

        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn forces_executable() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let dir = run_command("mk -x foo.txt")?;

        let file = std::fs::File::open(dir.path().join("foo.txt"))?;
        assert_eq!(file.metadata()?.permissions().mode() & 0o111, 0o111);

        Ok(())
    }
}
