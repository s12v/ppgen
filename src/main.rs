use std::io::{self, Write};
use std::process::ExitCode;

/// EFF large wordlist: 7776 lowercase words, one per line (~12.9 bits each).
/// Derived from https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt (CC BY 4.0).
const WORDLIST: &str = include_str!("wordlist.txt");
const EXPECTED_WORDS: u64 = 7776;
const DEFAULT_WORDS: u32 = 5;
const MAX_WORDS: u32 = 64;
const MAX_DIGITS: u32 = 8;

/// Password alphabet: 62 alphanumerics (~5.95 bits each), optionally plus
/// 26 symbols (~6.46 bits each). Quotes, backslash, pipe and space are left
/// out on purpose: they break shells and web forms more often than they help.
const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
const SYMBOLS: &[u8] = b"!@#$%^&*()-_=+[]{};:,.<>?/";
const DEFAULT_PASSWORD_LEN: u32 = 16;
const MAX_PASSWORD_LEN: u32 = 256;

const USAGE: &str = "\
ppgen - diceware passphrase generator

Usage: ppgen [options]

Passphrase (default):
  -w, --words <N>    words in the passphrase (default: 5, max: 64)
  -d, --sep <S>      separator between words (default: '-')
  -n, --digits <N>   append a random number, N digits (default: 0, max: 8)
  -c, --capitalize   capitalize each word (adds no entropy; for password policies)

Password:
  -p, --password [N] random password of N characters from A-Za-z0-9 (default: 16, max: 256)
      --symbols      also use !@#$%^&*()-_=+[]{};:,.<>?/

Both:
  -b, --bits <N>     instead of -w or -p N: use the length needed for N bits of entropy
  -v, --verbose      print the entropy estimate to stderr
  -h, --help         print this help

Wordlist: EFF large wordlist, 7776 words, ~12.9 bits per word.
Characters: ~5.95 bits each from A-Za-z0-9, ~6.46 with --symbols.
";

enum Kind {
    Passphrase {
        words: u32,
        sep: String,
        digits: u32,
        capitalize: bool,
    },
    Password {
        len: u32,
        symbols: bool,
    },
}

struct Opts {
    kind: Kind,
    verbose: bool,
}

fn bits_per_word() -> f64 {
    (EXPECTED_WORDS as f64).log2()
}

fn bits_per_char(symbols: bool) -> f64 {
    (alphabet(symbols).len() as f64).log2()
}

fn alphabet(symbols: bool) -> Vec<u8> {
    if symbols {
        [ALNUM, SYMBOLS].concat()
    } else {
        ALNUM.to_vec()
    }
}

impl Opts {
    fn entropy_bits(&self) -> f64 {
        match &self.kind {
            Kind::Passphrase { words, digits, .. } => {
                f64::from(*words) * bits_per_word() + f64::from(*digits) * 10f64.log2()
            }
            Kind::Password { len, symbols } => f64::from(*len) * bits_per_char(*symbols),
        }
    }

    fn describe_entropy(&self) -> String {
        match &self.kind {
            Kind::Passphrase { words, digits, .. } => format!(
                "entropy: {words} words x {:.1} bits + {digits} digits x 3.3 bits = ~{:.1} bits",
                bits_per_word(),
                self.entropy_bits()
            ),
            Kind::Password { len, symbols } => format!(
                "entropy: {len} chars x {:.2} bits = ~{:.1} bits",
                bits_per_char(*symbols),
                self.entropy_bits()
            ),
        }
    }
}

enum Command {
    Generate(Opts),
    Help,
}

/// Parses argv without the program name. Error messages are bare;
/// main() adds the "ppgen: " prefix and the --help hint.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut words: Option<u32> = None;
    let mut bits: Option<u32> = None;
    let mut sep: Option<String> = None;
    let mut digits: Option<u32> = None;
    let mut capitalize = false;
    let mut password: Option<Option<u32>> = None; // Some(None) = -p without a length
    let mut symbols = false;
    let mut verbose = false;
    let mut passphrase_flag: Option<&str> = None; // first passphrase-only flag seen

    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let flag = flag.as_str();
        match flag {
            "-h" | "--help" => return Ok(Command::Help),
            "-w" | "--words" => {
                let value = take_value(&mut it, flag)?;
                let n = parse_num(flag, value)?;
                if n == 0 || n > MAX_WORDS {
                    return Err(format!("{flag} must be 1..={MAX_WORDS}, got {value}"));
                }
                words = Some(n);
                passphrase_flag.get_or_insert(flag);
            }
            "-d" | "--sep" => {
                sep = Some(take_value(&mut it, flag)?.to_string());
                passphrase_flag.get_or_insert(flag);
            }
            "-n" | "--digits" => {
                let value = take_value(&mut it, flag)?;
                let n = parse_num(flag, value)?;
                if n > MAX_DIGITS {
                    return Err(format!("{flag} must be 0..={MAX_DIGITS}, got {value}"));
                }
                digits = Some(n);
                passphrase_flag.get_or_insert(flag);
            }
            "-c" | "--capitalize" => {
                capitalize = true;
                passphrase_flag.get_or_insert(flag);
            }
            "-p" | "--password" => {
                // optional length: consume the next arg only if it is a number
                let len = match it.clone().next().and_then(|v| v.parse::<u32>().ok()) {
                    Some(n) => {
                        it.next();
                        if n == 0 || n > MAX_PASSWORD_LEN {
                            return Err(format!("{flag} must be 1..={MAX_PASSWORD_LEN}, got {n}"));
                        }
                        Some(n)
                    }
                    None => None,
                };
                password = Some(len);
            }
            "--symbols" => symbols = true,
            "-b" | "--bits" => {
                let value = take_value(&mut it, flag)?;
                let n = parse_num(flag, value)?;
                if n == 0 {
                    return Err(format!("{flag} must be positive"));
                }
                bits = Some(n);
            }
            "-v" | "--verbose" => verbose = true,
            other => return Err(format!("unknown option '{other}'")),
        }
    }

    let kind = match password {
        Some(explicit_len) => {
            if let Some(flag) = passphrase_flag {
                return Err(format!(
                    "{flag} applies to passphrases and cannot be combined with -p"
                ));
            }
            let len = match (explicit_len, bits) {
                (Some(_), Some(_)) => return Err("-p N and -b cannot be combined".to_string()),
                (Some(n), None) => n,
                (None, Some(b)) => {
                    length_for_bits(b, bits_per_char(symbols), MAX_PASSWORD_LEN, "chars")?
                }
                (None, None) => DEFAULT_PASSWORD_LEN,
            };
            Kind::Password { len, symbols }
        }
        None => {
            if symbols {
                return Err("--symbols requires -p".to_string());
            }
            let digits = digits.unwrap_or(0);
            let words = match (words, bits) {
                (Some(_), Some(_)) => return Err("-w and -b cannot be combined".to_string()),
                (Some(w), None) => w,
                (None, Some(b)) => {
                    let from_digits = f64::from(digits) * 10f64.log2();
                    let remaining = (f64::from(b) - from_digits).max(0.0);
                    length_for_bits(remaining, bits_per_word(), MAX_WORDS, "words")?
                }
                (None, None) => DEFAULT_WORDS,
            };
            Kind::Passphrase {
                words,
                sep: sep.unwrap_or_else(|| "-".to_string()),
                digits,
                capitalize,
            }
        }
    };
    Ok(Command::Generate(Opts { kind, verbose }))
}

/// Smallest count (at least 1) of `unit`s worth `bits_each` that reaches `bits`.
fn length_for_bits(
    bits: impl Into<f64>,
    bits_each: f64,
    max: u32,
    unit: &str,
) -> Result<u32, String> {
    let bits = bits.into();
    let n = (bits / bits_each).ceil().max(1.0) as u32;
    if n > max {
        return Err(format!("-b {bits:.0} needs {n} {unit}, max is {max}"));
    }
    Ok(n)
}

fn take_value<'a>(it: &mut std::slice::Iter<'a, String>, flag: &str) -> Result<&'a str, String> {
    it.next()
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_num(flag: &str, s: &str) -> Result<u32, String> {
    s.parse::<u32>()
        .map_err(|_| format!("{flag} expects a number, got '{s}'"))
}

fn random_u64() -> Result<u64, getrandom::Error> {
    let mut buf = [0u8; 8];
    getrandom::fill(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

/// Uniform value in 0..n, rejection sampling against modulo bias.
fn uniform(n: u64) -> Result<u64, getrandom::Error> {
    let limit = u64::MAX / n * n; // largest multiple of n below 2^64
    loop {
        let r = random_u64()?;
        if r < limit {
            return Ok(r % n);
        }
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn generate(opts: &Opts, words: &[&str]) -> Result<String, getrandom::Error> {
    match &opts.kind {
        Kind::Passphrase {
            words: count,
            sep,
            digits,
            capitalize: cap,
        } => {
            let mut pw = String::new();
            for k in 0..*count as usize {
                if k > 0 {
                    pw.push_str(sep);
                }
                let idx = uniform(words.len() as u64)?;
                let word = words[idx as usize];
                if *cap {
                    pw.push_str(&capitalize(word));
                } else {
                    pw.push_str(word);
                }
            }
            if *digits > 0 {
                let v = uniform(10u64.pow(*digits))?;
                pw.push_str(&format!("{v:0width$}", width = *digits as usize));
            }
            Ok(pw)
        }
        Kind::Password { len, symbols } => {
            let alphabet = alphabet(*symbols);
            let mut pw = String::with_capacity(*len as usize);
            for _ in 0..*len {
                let idx = uniform(alphabet.len() as u64)?;
                pw.push(alphabet[idx as usize] as char);
            }
            Ok(pw)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(Command::Generate(opts)) => opts,
        Ok(Command::Help) => {
            print!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(msg) => {
            eprintln!("ppgen: {msg}\nTry 'ppgen --help' for more information.");
            return ExitCode::from(2);
        }
    };

    let words: Vec<&str> = WORDLIST.lines().collect();
    if words.len() as u64 != EXPECTED_WORDS {
        eprintln!(
            "ppgen: embedded wordlist is corrupt ({} words, expected {EXPECTED_WORDS})",
            words.len()
        );
        return ExitCode::FAILURE;
    }

    let pw = match generate(&opts, &words) {
        Ok(pw) => pw,
        Err(e) => {
            eprintln!("ppgen: random source failure: {e}");
            return ExitCode::FAILURE;
        }
    };

    if opts.verbose {
        eprintln!("{}", opts.describe_entropy());
    }

    let mut out = io::stdout().lock();
    match writeln!(out, "{pw}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE, // e.g. broken pipe, stay quiet
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn opts(v: &[&str]) -> Opts {
        match parse_args(&args(v)) {
            Ok(Command::Generate(o)) => o,
            _ => panic!("expected Generate for {v:?}"),
        }
    }

    fn phrase(v: &[&str]) -> (u32, String, u32, bool) {
        match opts(v).kind {
            Kind::Passphrase {
                words,
                sep,
                digits,
                capitalize,
            } => (words, sep, digits, capitalize),
            Kind::Password { .. } => panic!("expected Passphrase for {v:?}"),
        }
    }

    fn password(v: &[&str]) -> (u32, bool) {
        match opts(v).kind {
            Kind::Password { len, symbols } => (len, symbols),
            Kind::Passphrase { .. } => panic!("expected Password for {v:?}"),
        }
    }

    #[test]
    fn wordlist_integrity() {
        assert_eq!(WORDLIST.lines().count() as u64, EXPECTED_WORDS);
        assert!(
            WORDLIST
                .lines()
                .all(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_lowercase() || c == '-'))
        );
    }

    #[test]
    fn alphabets_have_no_duplicates() {
        for symbols in [false, true] {
            let mut a = alphabet(symbols);
            let n = a.len();
            a.sort_unstable();
            a.dedup();
            assert_eq!(a.len(), n);
            assert!(a.iter().all(|c| c.is_ascii_graphic()));
        }
        assert_eq!(alphabet(false).len(), 62);
        assert_eq!(alphabet(true).len(), 88);
    }

    #[test]
    fn uniform_stays_in_range() {
        for &n in &[2u64, 3, 6, 10, 62, 88, 1000, 4096, 7776] {
            for _ in 0..1000 {
                assert!(uniform(n).unwrap() < n);
            }
        }
    }

    #[test]
    fn uniform_is_roughly_even() {
        let n = 6u64;
        let mut bins = [0u32; 6];
        for _ in 0..60_000 {
            bins[uniform(n).unwrap() as usize] += 1;
        }
        for &count in &bins {
            assert!((9000..11000).contains(&count), "biased bins: {bins:?}");
        }
    }

    #[test]
    fn digits_are_zero_padded() {
        let words: Vec<&str> = WORDLIST.lines().take(2).collect();
        let o = opts(&["-w", "2", "-n", "2"]);
        let pw = generate(&o, &words).unwrap();
        // the number is always the last `digits` chars (list contains hyphenated words)
        let suffix = &pw[pw.len() - 2..];
        assert!(suffix.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn capitalize_touches_only_the_first_letter() {
        assert_eq!(capitalize("gusto"), "Gusto");
        assert_eq!(capitalize("yo-yo"), "Yo-yo");
        assert_eq!(capitalize(""), "");
        let words = ["alpha", "beta"];
        let o = opts(&["-w", "3", "-c", "-d", " "]);
        let pw = generate(&o, &words).unwrap();
        for w in pw.split(' ') {
            assert!(w == "Alpha" || w == "Beta", "{pw:?}");
        }
    }

    #[test]
    fn password_uses_only_its_alphabet() {
        let o = opts(&["-p", "200"]);
        let pw = generate(&o, &[]).unwrap();
        assert_eq!(pw.len(), 200);
        assert!(pw.bytes().all(|b| ALNUM.contains(&b)), "{pw:?}");

        let o = opts(&["-p", "200", "--symbols"]);
        let pw = generate(&o, &[]).unwrap();
        assert_eq!(pw.len(), 200);
        assert!(
            pw.bytes()
                .all(|b| ALNUM.contains(&b) || SYMBOLS.contains(&b)),
            "{pw:?}"
        );
        // 200 draws from 88 symbols: P(no symbol at all) = (62/88)^200 ~ 1e-31
        assert!(pw.bytes().any(|b| SYMBOLS.contains(&b)), "{pw:?}");
    }

    #[test]
    fn entropy_estimate() {
        let close = |a: f64, b: f64| (a - b).abs() < 0.1;
        assert!(close(opts(&[]).entropy_bits(), 64.6));
        assert!(close(opts(&["-w", "4", "-n", "2"]).entropy_bits(), 58.3));
        assert!(close(opts(&["-c"]).entropy_bits(), 64.6)); // capitalize adds nothing
        assert!(close(opts(&["-p"]).entropy_bits(), 95.3));
        assert!(close(opts(&["-p", "--symbols"]).entropy_bits(), 103.3));
        assert!(close(opts(&["-p", "10"]).entropy_bits(), 59.5));
    }

    #[test]
    fn bits_picks_the_length() {
        assert_eq!(phrase(&["-b", "1"]).0, 1);
        assert_eq!(phrase(&["-b", "64"]).0, 5);
        assert_eq!(phrase(&["-b", "65"]).0, 6);
        assert_eq!(phrase(&["-b", "80"]).0, 7);
        assert_eq!(phrase(&["-b", "80", "-n", "4"]).0, 6);
        assert_eq!(phrase(&["-n", "8", "-b", "20"]).0, 1); // digits alone would do
        assert!(opts(&["-b", "80"]).entropy_bits() >= 80.0);

        assert_eq!(password(&["-p", "-b", "80"]).0, 14);
        assert_eq!(password(&["-p", "--symbols", "-b", "80"]).0, 13);
        assert_eq!(password(&["-b", "1", "-p"]).0, 1);
        assert!(opts(&["-p", "-b", "128"]).entropy_bits() >= 128.0);

        for bad in [
            &["-b", "0"][..],
            &["-b", "900"],
            &["-b", "80", "-w", "5"],
            &["-p", "20", "-b", "80"],
            &["-p", "-b", "2000"],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn parses_passphrase_options() {
        assert_eq!(phrase(&[]), (DEFAULT_WORDS, "-".to_string(), 0, false));
        assert_eq!(
            phrase(&["-w", "3", "-d", "+", "-n", "2", "-c"]),
            (3, "+".to_string(), 2, true)
        );
        assert!(!opts(&[]).verbose);
        assert!(opts(&["-v"]).verbose);
    }

    #[test]
    fn parses_password_options() {
        assert_eq!(password(&["-p"]), (DEFAULT_PASSWORD_LEN, false));
        assert_eq!(password(&["-p", "20"]), (20, false));
        assert_eq!(
            password(&["--password", "--symbols"]),
            (DEFAULT_PASSWORD_LEN, true)
        );
        assert_eq!(password(&["-p", "-v"]), (DEFAULT_PASSWORD_LEN, false)); // -v is not a length
        assert!(opts(&["-p", "-v"]).verbose);

        for bad in [
            &["-p", "0"][..],
            &["-p", "257"],
            &["--symbols"],
            &["-p", "-w", "5"],
            &["-p", "-d", "/"],
            &["-p", "-n", "2"],
            &["-p", "-c"],
            &["-c", "-p"],
        ] {
            assert!(parse_args(&args(bad)).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn help_is_not_an_error() {
        assert!(matches!(parse_args(&args(&["-h"])), Ok(Command::Help)));
        assert!(matches!(
            parse_args(&args(&["-w", "3", "--help"])),
            Ok(Command::Help)
        ));
    }

    #[test]
    fn rejects_bad_options() {
        assert!(parse_args(&args(&["-w", "0"])).is_err());
        assert!(parse_args(&args(&["-w", "100"])).is_err());
        assert!(parse_args(&args(&["-w", "x"])).is_err());
        assert!(parse_args(&args(&["-n", "9"])).is_err());
        assert!(parse_args(&args(&["-w"])).is_err());
        assert!(parse_args(&args(&["--nope"])).is_err());
    }
}
