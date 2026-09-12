# ppgen

[![CI](https://github.com/s12v/ppgen/actions/workflows/ci.yml/badge.svg)](https://github.com/s12v/ppgen/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/s12v/ppgen)](https://github.com/s12v/ppgen/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A small command-line tool that generates random, easy-to-remember passphrases
from the EFF wordlist.

```
$ ppgen
dangle-grandkid-bakeshop-rocky-edgy
$ ppgen -w 4 -n 2
latitude-maker-garden-skillful73
$ ppgen -w 6 -d "."
showdown.plenty.pantomime.blurt.bristle.bonus
$ ppgen -b 80 -v
entropy: 7 words x 12.9 bits + 0 digits x 3.3 bits = ~90.5 bits
geometry-striving-vitally-desolate-liftoff-unpiloted-chomp
```

## Installing

```
brew install s12v/tap/ppgen
```

Works on macOS and Linux. Without Homebrew, grab a prebuilt binary for macOS
(Apple Silicon, Intel) or Linux (x86-64, ARM64; statically linked) from the
[releases page](https://github.com/s12v/ppgen/releases) and put `ppgen`
somewhere on your `PATH`.

## Building

```
cargo build --release
```

A single ~370 KB binary with the wordlist embedded. The only dependency is the
[`getrandom`](https://crates.io/crates/getrandom) crate (a thin wrapper over
`getrandom(2)` / `arc4random_buf` / `BCryptGenRandom`).

## Usage

```
ppgen [options]
  -w, --words <N>    words in the passphrase (default: 5, max: 64)
  -b, --bits <N>     instead of -w: use as many words as needed for N bits of entropy
  -d, --sep <S>      separator between words (default: '-')
  -n, --digits <N>   append a random number, N digits (default: 0, max: 8)
  -c, --capitalize   capitalize each word (adds no entropy; for password policies)
  -v, --verbose      print the entropy estimate to stderr
  -h, --help         print this help
```

Exit codes: `0` success, `1` RNG or output failure, `2` invalid arguments.
`--help` prints to stdout and exits 0; all diagnostics (including `-v`) go to
stderr, so `ppgen -v | pbcopy` copies just the passphrase.

`-c` and `-n` exist for sites that insist on an uppercase letter or a digit.
Capitalizing every word is a fixed rule, so it adds nothing an attacker has to
guess; digits do add ~3.3 bits each, but a sixth word adds ~12.9.

## Entropy

Each word from the EFF list (7776 words) contributes log₂(7776) ≈ 12.9 bits,
each digit log₂(10) ≈ 3.3 bits — so four digits are worth about one word.

For comparison, one character of a uniformly random `A-Za-z0-9` password
contributes log₂(62) ≈ 5.95 bits. The last column shows how long such a
password would have to be to match the passphrase:

| Options     | Example                                                       | Entropy, bits | Random `A-Za-z0-9`, chars |
|-------------|---------------------------------------------------------------|--------------:|--------------------------:|
| `-w 3`      | `detergent-alfalfa-rockband`                                  |            39 |                         7 |
| `-w 3 -n 2` | `lugged-daytime-exploit85`                                    |            45 |                         8 |
| `-w 4`      | `mouse-headsman-twisting-whimsical`                           |            52 |                         9 |
| `-w 3 -n 4` | `relay-stubbly-tumbling0141`                                  |            52 |                         9 |
| `-w 4 -n 2` | `quarry-lapel-headlamp-stagnant95`                            |            58 |                        10 |
| `-w 5`      | `jury-many-smock-dose-sterility` (default)                    |            65 |                        11 |
| `-w 4 -n 4` | `taunt-demeanor-throbbing-angrily4431`                        |            65 |                        11 |
| `-w 5 -n 3` | `ripping-greedy-rind-repose-factoid340`                       |            75 |                        13 |
| `-w 6`      | `slacked-polymer-haven-radiance-spent-washable`               |            78 |                        13 |
| `-w 6 -n 2` | `morphing-quit-backpack-context-aroma-retouch28`              |            84 |                        14 |
| `-w 7`      | `dreamily-stuffy-defraud-budding-plank-numerate-refutable`    |            90 |                        15 |
| `-w 8`      | `avoid-bartender-hut-conclude-bulldozer-lake-qualifier-rehab` |           103 |                        17 |

The passphrase is longer to type but far easier to remember; the entropy is
the same as long as the words are chosen by a proper random source, which is
the point of this tool.

## How many words?

It depends on what an attacker can do with a guess:

| Threat                                                          | Target      | Command     |
|-----------------------------------------------------------------|-------------|-------------|
| Online guessing against a service with rate limiting            | ≥ 40 bits   | `-w 4`      |
| Offline cracking of a leaked hash protected by a slow KDF (password manager vault, LUKS/FileVault, SSH key) | ≥ 75 bits | `-w 6` or `-b 75` |
| Offline cracking where the KDF is fast or unknown               | ≥ 90 bits   | `-w 7` or `-b 90` |

`-b` picks the word count for you and `-v` shows what you got. Passphrases are
for secrets you type from memory; everything else belongs in a password
manager as a long random string.

Two things matter more than the exact number: take the phrase as generated
(hand-picking or "improving" words removes entropy, because people are
predictable), and never reuse it. [NIST SP 800-63B][nist] says the same about
passwords in general: length beats complexity rules, forced rotation does more
harm than good, and checking against breach lists is what actually helps.

[nist]: https://pages.nist.gov/800-63-4/sp800-63b.html

## Why you can trust it

- **Cryptographic RNG**: the OS entropy source via `getrandom`; no
  `rand()`/`srand()`.
- **No modulo bias**: word indices are drawn by rejection sampling (values that
  would skew `r % 7776` are discarded).
- **Wordlist integrity**: on every run the embedded list is checked to contain
  exactly 7776 words.
- **Output discipline**: the passphrase is the only line on stdout; diagnostics
  go to stderr; exit codes 0/1/2 as above.
- Tests: unit tests for wordlist integrity, sampling range and uniformity,
  digit padding, entropy math and argument parsing; integration tests run the
  built binary and check exit codes, stdout/stderr discipline and output shape.

## Wordlist

[EFF large wordlist](https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt),
7776 words, CC BY 4.0. The embedded copy is `src/wordlist.txt` (one word per
line, produced from the original with `cut -f2`).

## License

Code: [MIT](LICENSE). EFF wordlist: CC BY 4.0 (attribution above).
