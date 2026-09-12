use std::io::{self, Write};
use std::process::ExitCode;

/// EFF large wordlist: 7776 lowercase words, one per line (~12.9 bits each).
/// Derived from https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt (CC BY 4.0).
const WORDLIST: &str = include_str!("wordlist.txt");
const EXPECTED_WORDS: u64 = 7776;
const DEFAULT_WORDS: u32 = 5;
const MAX_WORDS: u32 = 64;
const MAX_DIGITS: u32 = 8;

const USAGE: &str = "\
ppgen - diceware passphrase generator

Usage: ppgen [options]

  -w, --words <N>    words in the passphrase (default: 5, max: 64)
  -b, --bits <N>     instead of -w: use as many words as needed for N bits of entropy
  -d, --sep <S>      separator between words (default: '-')
  -n, --digits <N>   append a random number, N digits (default: 0, max: 8)
  -c, --capitalize   capitalize each word (adds no entropy; for password policies)
  -v, --verbose      print the entropy estimate to stderr
  -h, --help         print this help

Wordlist: EFF large wordlist, 7776 words, ~12.9 bits per word.
";

struct Opts {
    words: u32,
    sep: String,
    digits: u32,
    capitalize: bool,
    verbose: bool,
}

impl Opts {
    fn entropy_bits(&self) -> f64 {
        f64::from(self.words) * bits_per_word() + f64::from(self.digits) * 10f64.log2()
    }
}

fn bits_per_word() -> f64 {
    (EXPECTED_WORDS as f64).log2()
}

enum Command {
    Generate(Opts),
    Help,
}

/// Parses argv without the program name. Error messages are bare;
/// main() adds the "ppgen: " prefix and the --help hint.
fn parse_args(args: &[String]) -> Result<Command, String> {
    let mut opts = Opts {
        words: DEFAULT_WORDS,
        sep: "-".to_string(),
        digits: 0,
        capitalize: false,
        verbose: false,
    };
    let mut words: Option<u32> = None;
    let mut bits: Option<u32> = None;

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
            }
            "-b" | "--bits" => {
                let value = take_value(&mut it, flag)?;
                let n = parse_num(flag, value)?;
                if n == 0 {
                    return Err(format!("{flag} must be positive"));
                }
                bits = Some(n);
            }
            "-d" | "--sep" => opts.sep = take_value(&mut it, flag)?.to_string(),
            "-n" | "--digits" => {
                let value = take_value(&mut it, flag)?;
                opts.digits = parse_num(flag, value)?;
                if opts.digits > MAX_DIGITS {
                    return Err(format!("{flag} must be 0..={MAX_DIGITS}, got {value}"));
                }
            }
            "-c" | "--capitalize" => opts.capitalize = true,
            "-v" | "--verbose" => opts.verbose = true,
            other => return Err(format!("unknown option '{other}'")),
        }
    }

    opts.words = match (words, bits) {
        (Some(_), Some(_)) => return Err("-w and -b cannot be combined".to_string()),
        (Some(w), None) => w,
        (None, Some(b)) => words_for_bits(opts.digits, b)?,
        (None, None) => DEFAULT_WORDS,
    };
    Ok(Command::Generate(opts))
}

/// Smallest word count (at least 1) reaching `bits` of entropy together
/// with `digits` random digits.
fn words_for_bits(digits: u32, bits: u32) -> Result<u32, String> {
    let from_digits = f64::from(digits) * 10f64.log2();
    let needed = ((f64::from(bits) - from_digits) / bits_per_word()).ceil();
    let n = needed.max(1.0) as u32;
    if n > MAX_WORDS {
        return Err(format!("-b {bits} needs {n} words, max is {MAX_WORDS}"));
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
    let mut pw = String::new();
    for k in 0..opts.words as usize {
        if k > 0 {
            pw.push_str(&opts.sep);
        }
        let idx = uniform(words.len() as u64)?;
        let word = words[idx as usize];
        if opts.capitalize {
            pw.push_str(&capitalize(word));
        } else {
            pw.push_str(word);
        }
    }
    if opts.digits > 0 {
        let v = uniform(10u64.pow(opts.digits))?;
        pw.push_str(&format!("{v:0width$}", width = opts.digits as usize));
    }
    Ok(pw)
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
        eprintln!(
            "entropy: {} words x {:.1} bits + {} digits x 3.3 bits = ~{:.1} bits",
            opts.words,
            bits_per_word(),
            opts.digits,
            opts.entropy_bits()
        );
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
    fn uniform_stays_in_range() {
        for &n in &[2u64, 3, 6, 10, 1000, 4096, 7776] {
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
    fn entropy_estimate() {
        let close = |a: f64, b: f64| (a - b).abs() < 0.05;
        assert!(close(opts(&[]).entropy_bits(), 64.6));
        assert!(close(opts(&["-w", "4", "-n", "2"]).entropy_bits(), 58.3));
        assert!(close(opts(&["-c"]).entropy_bits(), 64.6)); // capitalize adds nothing
    }

    #[test]
    fn bits_picks_the_word_count() {
        assert_eq!(opts(&["-b", "1"]).words, 1);
        assert_eq!(opts(&["-b", "64"]).words, 5);
        assert_eq!(opts(&["-b", "65"]).words, 6);
        assert_eq!(opts(&["-b", "80"]).words, 7);
        assert_eq!(opts(&["-b", "80", "-n", "4"]).words, 6);
        assert_eq!(opts(&["-n", "8", "-b", "20"]).words, 1); // digits alone would do
        assert!(opts(&["-b", "80"]).entropy_bits() >= 80.0);
        assert!(parse_args(&args(&["-b", "0"])).is_err());
        assert!(parse_args(&args(&["-b", "900"])).is_err()); // > 64 words
        assert!(parse_args(&args(&["-b", "80", "-w", "5"])).is_err());
    }

    #[test]
    fn parses_options() {
        let o = opts(&[]);
        assert_eq!((o.words, o.sep.as_str(), o.digits), (DEFAULT_WORDS, "-", 0));
        assert!(!o.capitalize && !o.verbose);
        let o = opts(&["-w", "3", "-d", "+", "-n", "2", "-c", "-v"]);
        assert_eq!((o.words, o.sep.as_str(), o.digits), (3, "+", 2));
        assert!(o.capitalize && o.verbose);
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
