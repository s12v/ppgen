//! Black-box tests of the built binary: exit codes, stdout/stderr discipline,
//! output shape. These are the guarantees the README makes.

use std::process::{Command, Output};

const WORDLIST: &str = include_str!("../src/wordlist.txt");

fn ppgen(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ppgen"))
        .args(args)
        .output()
        .expect("failed to run ppgen")
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("stdout is not UTF-8")
}

fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).expect("stderr is not UTF-8")
}

/// The passphrase is the only line on stdout, without a trailing separator.
fn single_line(o: &Output) -> String {
    let out = stdout(o);
    let line = out
        .strip_suffix('\n')
        .expect("output must end with newline");
    assert!(!line.contains('\n'), "more than one line: {out:?}");
    line.to_string()
}

#[test]
fn default_prints_one_lowercase_line() {
    let o = ppgen(&[]);
    assert!(o.status.success());
    assert!(o.stderr.is_empty());
    let line = single_line(&o);
    // words themselves may contain '-' (yo-yo), so only check the alphabet here
    assert!(
        line.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
        "{line:?}"
    );
}

#[test]
fn default_is_five_words() {
    let o = ppgen(&["-d", "/"]);
    assert!(o.status.success());
    assert_eq!(single_line(&o).split('/').count(), 5);
}

#[test]
fn words_come_from_the_wordlist() {
    let o = ppgen(&["-w", "8", "-d", "/"]);
    assert!(o.status.success());
    let line = single_line(&o);
    let words: Vec<&str> = line.split('/').collect();
    assert_eq!(words.len(), 8);
    for w in words {
        assert!(
            WORDLIST.lines().any(|l| l == w),
            "{w:?} is not in the wordlist"
        );
    }
}

#[test]
fn custom_separator_can_be_multichar_or_empty() {
    let o = ppgen(&["-w", "3", "-d", " :: "]);
    assert!(o.status.success());
    assert_eq!(single_line(&o).matches(" :: ").count(), 2);

    let o = ppgen(&["-w", "3", "-d", ""]);
    assert!(o.status.success());
    let line = single_line(&o);
    assert!(
        line.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
        "{line:?}"
    );
}

#[test]
fn digits_are_appended_exactly_n_times() {
    let o = ppgen(&["-w", "2", "-d", "/", "-n", "4"]);
    assert!(o.status.success());
    let line = single_line(&o);
    let (words, digits) = line.split_at(line.len() - 4);
    assert!(digits.chars().all(|c| c.is_ascii_digit()), "{line:?}");
    assert!(
        words.ends_with(|c: char| c.is_ascii_lowercase()),
        "{line:?}"
    );
    assert_eq!(words.split('/').count(), 2);
}

#[test]
fn two_runs_differ() {
    // 65 bits of entropy: a collision here means the RNG is broken.
    assert_ne!(stdout(&ppgen(&[])), stdout(&ppgen(&[])));
}

#[test]
fn help_goes_to_stdout_with_exit_zero() {
    for flag in ["-h", "--help"] {
        let o = ppgen(&[flag]);
        assert_eq!(o.status.code(), Some(0), "{flag}");
        assert!(stdout(&o).starts_with("ppgen - "), "{flag}");
        assert!(o.stderr.is_empty(), "{flag}");
    }
}

#[test]
fn bad_arguments_go_to_stderr_with_exit_two() {
    let cases: &[&[&str]] = &[
        &["--nope"],
        &["-w"],
        &["-w", "0"],
        &["-w", "65"],
        &["-w", "abc"],
        &["-n", "9"],
        &["-d"],
    ];
    for args in cases {
        let o = ppgen(args);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(o.stdout.is_empty(), "{args:?} wrote to stdout");
        let err = stderr(&o);
        assert!(err.starts_with("ppgen: "), "{args:?}: {err:?}");
        assert!(err.contains("Try 'ppgen --help'"), "{args:?}: {err:?}");
    }
}

#[test]
fn boundaries_are_inclusive() {
    assert!(ppgen(&["-w", "1"]).status.success());
    assert!(ppgen(&["-w", "64"]).status.success());
    assert!(ppgen(&["-n", "0"]).status.success());
    assert!(ppgen(&["-n", "8"]).status.success());
}
