# address-tool

Every address form I've filled out either accepts garbage silently or
rejects perfectly valid input with some baroque regex. This is a small
command-line tool that does one thing: take a US postal address as plain
text, check that it's actually well-formed (real state code, plausible
zip), and print it back out in a canonical shape. Either as text you'd
put on an envelope, or as JSON if you're piping it into something else.

Right now it only understands US addresses in the standard block form:

```
123 Main St
Apt 4B
Springfield, IL 62704
```

The last non-blank line has to be `City, ST ZIP` (or `ZIP-XXXX`). Every
line above that is treated as a street line, and you can have as many
of those as you need (apartment number, suite, attention line, etc).

## Usage

Build it with plain cargo, no extra setup:

```
cargo build --release
```

Feed it an address on stdin:

```
$ printf '123 Main St\nSpringfield, IL 62704\n' | ./target/release/address-tool
123 Main St
Springfield, IL 62704
```

Or give it a file:

```
$ ./target/release/address-tool address.txt
```

Add `--json` for machine-readable output instead:

```
$ printf '1 Infinite Loop\nCupertino, CA 95014\n' | ./target/release/address-tool --json
{"street_lines":["1 Infinite Loop"],"city":"Cupertino","state":"CA","zip":"95014"}
```

Invalid input still produces useful output. In text mode the error goes
to stderr and the process exits non-zero; in JSON mode it's a JSON
object on stdout instead, so a calling script doesn't have to fork its
error handling between two output paths:

```
$ printf '123 Main St\nNowhere, ZZ 00000\n' | ./target/release/address-tool --json
{"error":"unrecognized state code: \"ZZ\""}
```

## What counts as valid right now

- At least one street line and a city/state/zip line.
- State must be a real two-letter USPS code (50 states, DC, and the
  territories that get their own code: PR, VI, GU, AS, MP).
- Zip must be 5 digits, or 5 digits + a hyphen + 4 digits.

That's deliberately narrow. See the roadmap below for what's missing.

## Suffix normalization

Street lines get their suffix words rewritten to the canonical USPS
Publication 28 abbreviation, so "Street", "Str", and "St" all come out
as "St" (same for Avenue/Ave, Boulevard/Blvd, and so on for the common
ones). This runs word by word, so it catches a suffix in the middle of
a line too:

```
$ printf '100 Main Street Suite 4\nSpringfield, IL 62704\n' | ./target/release/address-tool
100 Main St Suite 4
Springfield, IL 62704
```

It only knows a few dozen suffix variants, not the full USPS table, and
it can't tell "St" the abbreviation from "St" the word (as in "St
Charles Ave") — since both cases already read "St" either way, that
particular ambiguity happens to be harmless.

## Status

Early skeleton. The parser only handles the single-format US case
described above — no international addresses, no fuzzy matching. Zero
dependencies by design; everything here is standard library only.
