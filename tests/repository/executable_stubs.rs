//! The stand-in executables the tests write and run: each written outside the
//! test process, so running one never meets "Text file busy", however many
//! tests start processes at the same moment.

use crate::harness::{root, temp_dir, test_sources, write_executable};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

/// `ETXTBSY`: the kernel refuses to run a file some process holds open for
/// writing.
const TEXT_FILE_BUSY: i32 = 26;

/// Write and run fifty stand-ins, counting the runs refused as busy.
fn write_and_run(dir: &Path, writer: usize) -> usize {
    (0..50)
        .filter(|run| {
            let path = dir.join(format!("stand-in-{writer}-{run}"));
            write_executable(&path, "#!/bin/bash\nexit 0\n");
            match Command::new(&path).status() {
                Ok(status) => {
                    assert!(status.success());
                    false
                }
                Err(error) if error.raw_os_error() == Some(TEXT_FILE_BUSY) => true,
                Err(error) => panic!("{}: {error}", path.display()),
            }
        })
        .count()
}

#[test]
fn a_stand_in_runs_while_other_threads_start_processes() {
    // What the test process does all the time: some threads start processes
    // while others write a stand-in and run it straight away. A child started
    // while this process holds the new file open for writing inherits that
    // descriptor until it runs its own program, and the stand-in is refused.
    let dir = temp_dir("executable-stubs");
    let stop = AtomicBool::new(false);
    let refused: usize = thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                while !stop.load(Ordering::Relaxed) {
                    Command::new("true").output().unwrap();
                }
            });
        }
        let writers: Vec<_> = (0..4)
            .map(|writer| {
                let dir = &dir;
                scope.spawn(move || write_and_run(dir, writer))
            })
            .collect();
        let counts: Vec<_> = writers
            .into_iter()
            .map(thread::ScopedJoinHandle::join)
            .collect();
        // Stop the starters before a writer's panic can leave them running.
        stop.store(true, Ordering::Relaxed);
        counts.into_iter().map(|count| count.unwrap()).sum()
    });
    assert_eq!(
        refused, 0,
        "{refused} of 200 stand-ins were refused as busy"
    );
}

#[test]
fn no_test_marks_a_file_executable_in_its_own_process() {
    // write_executable is the one way a test makes a file it can run; the
    // needles are split so this file does not name them.
    let needles = [
        ["set_", "permissions("].concat(),
        ["from", "_mode("].concat(),
    ];
    let offenders: Vec<String> = test_sources()
        .into_iter()
        .filter(|path| {
            let text = fs::read_to_string(path).unwrap();
            needles.iter().any(|needle| text.contains(needle.as_str()))
        })
        .map(|path| path.strip_prefix(root()).unwrap().display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "write stand-ins with write_executable, not in the test process: {offenders:?}"
    );
}
