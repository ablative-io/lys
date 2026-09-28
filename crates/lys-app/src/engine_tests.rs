#![cfg(test)]

use std::path::{Path, PathBuf};

use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn the_sockets_asked_are_each_engines_and_the_system_link() {
    let home = Path::new("/Users/someone");
    let paths = socket_paths(Some(home));
    assert_eq!(paths[0], home.join(".docker/run/docker.sock"));
    assert!(paths.contains(&home.join(".colima/default/docker.sock")));
    assert!(paths.contains(&home.join(".orbstack/run/docker.sock")));
    assert!(paths.contains(&home.join(".rd/docker.sock")));
    assert_eq!(paths.last(), Some(&PathBuf::from("/var/run/docker.sock")));
    assert_eq!(socket_paths(None), [PathBuf::from("/var/run/docker.sock")]);
}

#[test]
fn a_missing_engine_is_guided_to_the_download_for_this_chip() {
    let apple = Guidance::missing(Chip::Apple);
    assert_eq!(apple.state, "missing");
    assert_eq!(apple.download, Some(Chip::Apple.download()));
    assert!(apple.download.is_some_and(|url| url.contains("/arm64/")));
    assert!(apple.what.contains("an Apple chip"));
    let intel = Guidance::missing(Chip::Intel);
    assert!(intel.download.is_some_and(|url| url.contains("/amd64/")));
    assert!(intel.what.contains("an Intel chip"));
    for guidance in [&apple, &intel] {
        let official = |url: &str| url.starts_with("https://desktop.docker.com/");
        assert!(guidance.download.is_some_and(official));
        assert!(guidance.page.starts_with("https://docs.docker.com/"));
    }
}

#[test]
fn a_stopped_engine_is_told_to_open_with_no_download() {
    let stopped = Guidance::stopped();
    assert_eq!(stopped.state, "stopped");
    assert_eq!(stopped.download, None);
    assert!(stopped.what.contains("Open Docker Desktop"));
}

#[test]
fn the_chip_is_read_from_the_architecture() {
    assert_eq!(Chip::of("aarch64"), Chip::Apple);
    assert_eq!(Chip::of("x86_64"), Chip::Intel);
}

/// The guidance says what the engine is for and that the install continues
/// by itself, and none of it names the issuer inside Lys, a port or a
/// terminal.
#[test]
fn the_guidance_says_why_and_that_lys_continues_and_names_no_issuer() -> TestResult {
    let mut read = 0;
    let every = [
        Guidance::missing(Chip::Apple),
        Guidance::missing(Chip::Intel),
        Guidance::stopped(),
    ];
    for guidance in every {
        assert!(guidance.why.contains("container engine"));
        assert!(guidance.then.contains("continues by itself"));
        let text = serde_json::to_string(&guidance)?.to_lowercase();
        for refused in [
            "rauthy",
            "terminal",
            "command",
            "localhost",
            "127.0.0.1",
            "password",
        ] {
            assert!(
                !text.contains(refused),
                "the guidance names {refused}: {text}"
            );
        }
        read += 1;
    }
    assert_eq!(read, 3);
    Ok(())
}

#[test]
fn a_socket_folder_not_made_yet_is_watched_at_its_nearest_existing_folder() -> TestResult {
    let home = tempfile::tempdir()?;
    let socket = home.path().join(".docker/run/docker.sock");
    assert_eq!(watched_folder(&socket), Some(home.path().to_path_buf()));
    std::fs::create_dir_all(home.path().join(".docker/run"))?;
    assert_eq!(
        watched_folder(&socket),
        Some(home.path().join(".docker/run"))
    );
    let twice = [socket, home.path().join(".docker/run/other.sock")];
    assert_eq!(watched_folders(&twice), [home.path().join(".docker/run")]);
    Ok(())
}

#[test]
fn a_socket_nobody_listens_on_does_not_answer() -> TestResult {
    let home = tempfile::tempdir()?;
    let absent = home.path().join("docker.sock");
    assert!(!answers(&absent));
    assert_eq!(answering(&[absent]), None);
    Ok(())
}

/// Serves `/_ping` on a socket at `path`, answering `200` to every caller.
#[cfg(unix)]
fn serve_ping(path: &Path) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let listener = std::os::unix::net::UnixListener::bind(path)?;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut request = [0_u8; 512];
            if stream.read(&mut request).is_err() {
                return;
            }
            let answer = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
            if stream.write_all(answer).is_err() {
                return;
            }
        }
    });
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_socket_answering_ping_answers() -> TestResult {
    let home = tempfile::tempdir()?;
    let socket = home.path().join("docker.sock");
    serve_ping(&socket)?;
    assert!(answers(&socket));
    let asked = [home.path().join("absent.sock"), socket.clone()];
    assert_eq!(answering(&asked), Some(socket));
    Ok(())
}

/// No wait in lys-app reads a clock or loops on a sleep: its source, apart
/// from its tests, holds none of the words a clock or a sleep is spelt with.
#[test]
fn no_wait_in_the_app_reads_a_clock_or_sleeps() -> TestResult {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let refused = [
        "Instant",
        "SystemTime",
        "sleep(",
        "Duration",
        "timeout(",
        "set_read_timeout",
    ];
    let mut read = 0;
    for entry in std::fs::read_dir(&src)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        let rust = path.extension().is_some_and(|extension| extension == "rs");
        if !rust || name.ends_with("_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        for word in refused {
            assert!(!text.contains(word), "{} holds {word}", path.display());
        }
        read += 1;
    }
    assert!(read >= 9, "read {read} source files");
    Ok(())
}

/// Docker Desktop's data folder and its engine's folder are the state
/// folders, and with no home there are none.
#[test]
fn the_state_folders_are_docker_desktops_data_folder() {
    let home = Path::new("/Users/someone");
    let data = home.join("Library/Containers/com.docker.docker/Data");
    assert_eq!(
        state_folders(Some(home)),
        [data.clone(), data.join("vms/0")]
    );
    assert!(state_folders(None).is_empty());
}
