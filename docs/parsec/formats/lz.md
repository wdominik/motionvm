[← Documentation index](../README.md)

# The RIB LZ Codec

*PARSEC — Dunkle Schatten (1994): the compression inside every packed RIB and every packed entry of the music library, as `XP01.EXE`'s decoder runs it.*

The decoder is `1E05h:0000h` in the unpacked `XP01.EXE` (`0x1E050`). It is
in place and runs **backwards**: the packed payload is copied to the start
of a `bufSize` buffer, the direction flag is set, and two pointers walk
toward each other — tokens are taken from the *end* of the payload, output
is written from the *end* of the buffer, and the decoder stops where the
two meet. There is no end marker and no checksum.

The decoder first compares the payload's length with `bufSize`
(`0x1E059`–`0x1E064`); when the two are equal the payload is stored, not
packed, and it returns at once (`0x1E066`, to the exit at `0x1E2E7`).

This is not the LZEXE bit stream the [executables](executables.md) are
packed with. The music library's packed songs are decoded by the same
routine ([audio](audio.md)).

## Tokens

A token is one byte, dispatched on its high nibble; `n` is the low nibble
and "next" the byte taken after the token.

| Token | Meaning |
|---|---|
| `0n` | Run of `n+4` copies of the next byte |
| `1n` | Run of `((n<<8)\|next)+20` copies of the byte after that |
| `2n` | `n+1` literal bytes |
| `3n` | `((n<<8)\|next)+17` literal bytes |
| `40` | Literal bytes, a 16-bit count |
| `41` | Match: 16-bit distance, 16-bit length |
| `42` | Match: 16-bit distance, length next byte `+17` |
| `43`–`4F` | Match: 16-bit distance, length `n+1` (4 to 16) |
| `5n` | Match: distance `((n<<8)\|next)+2`, length the following byte `+14` (14 to 269) |
| `6n`–`Fn` | Match: distance `((n<<8)\|next)+2`, length the high nibble `−2` (4 to 13) |

A match copies from `dist` bytes above the write position as that position
descends, so a match may overlap what it is writing.

**A 16-bit operand comes high byte first.** The stream is read backwards,
so the first byte taken of a word is its high byte.

The largest RIB the game ships decodes to 120 088 bytes, slot 12 of
`DSROOM.DAT`.

## See also

- [Resource containers](containers.md) — the RIB header the codec sits behind
- [Audio](audio.md) — the music library's packed entries
