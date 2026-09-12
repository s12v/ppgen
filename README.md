# ppgen

A small command-line tool that generates random, easy-to-remember passphrases
from the EFF wordlist.

```
$ ppgen
dangle-grandkid-bakeshop-rocky-edgy
$ ppgen -w 4 -n 2
latitude-maker-garden-skillful73
$ ppgen -w 6 -d "."
showdown.plenty.pantomime.blurt.bristle.bonus
```

## Installing

Prebuilt binaries for macOS (Apple Silicon) and Linux (x86-64 and ARM64,
statically linked) are attached to each
[GitHub release](https://github.com/s12v/ppgen/releases). Unpack and put
`ppgen` somewhere on your `PATH`.

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
  -d, --sep <S>      separator between words (default: '-')
  -n, --digits <N>   append a random number, N digits (default: 0, max: 8)
  -h, --help         print this help
```

Exit codes: `0` success, `1` RNG or output failure, `2` invalid arguments.
`--help` prints to stdout and exits 0; all diagnostics go to stderr.

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

## Why you can trust it

- **Cryptographic RNG**: the OS entropy source via `getrandom`; no
  `rand()`/`srand()`.
- **No modulo bias**: word indices are drawn by rejection sampling (values that
  would skew `r % 7776` are discarded).
- **Wordlist integrity**: on every run the embedded list is checked to contain
  exactly 7776 words.
- **Output discipline**: the passphrase is the only line on stdout; diagnostics
  go to stderr; exit codes 0/1/2 as above.
- Unit tests cover wordlist integrity, sampling range and uniformity, digit
  padding, and argument parsing.

## Wordlist

[EFF large wordlist](https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt),
7776 words, CC BY 4.0. The embedded copy is `src/wordlist.txt` (one word per
line, produced from the original with `cut -f2`).

## License

Code: [MIT](LICENSE). EFF wordlist: CC BY 4.0 (attribution above).
