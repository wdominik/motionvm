[← Documentation index](../README.md)

# The Animator

*PARSEC — Dunkle Schatten (1994): `XP01.EXE`, as its unpacked load image of 150 096 bytes shows it. Addresses `0x…` are linear offsets into that image; `seg:off` names a routine by its far address.*

A room's doors swing, its sleeper turns over, its people breathe — none of
it in the location's own code. The location asks for an animation by
number, and segment `0883h` plays it out of an `AN` block: the room's own,
resource 1 of its slot, or the global one the figure's animations come
from ([animations](../formats/animations.md)). This page is what the
byte-code in those blocks says and how the animator runs it.

## Where it runs

Every frame pass hands the ticks counted since the last one round, in
order, to the tasks (`0CFDh:11ECh`, `0x101F4`), the walkers
(`0A9Fh:0892h`, `0x101FB`) and the animator, `0883h:0305h` (`0x10202`)
([game loop](game-loop.md)). The animator walks the room's layers, and in
each the objects and then the chain the walkers hang in. A live object
(`+2` bit 0, `0x08B4D`) with a state (`+12h`, `0x08B57`) has the ticks
counted off its countdown `+16h` (`0x08B5E`), and steps when none are
left (`0x08B62`); a kind-8 record in a chain steps every tick
(`0x08B9D`).

A wait is counted in ticks, not in passes. On a machine fast enough to
run several frame passes between two ticks of the clock, the passes with
no tick behind them hand the animator nothing and nothing moves; how
many passes fall into one tick is the machine's, how many ticks a frame
stands is the stream's.

## One step

`0883h:012Ah` is one step of a record whose countdown ran out, by its
state: 7 runs the entry's stream, then its frame (`0x08A71`); 8 the same,
then the chain (`0x08A82`); 9, a part of a chain, never steps — its
countdown is set to `7FFFh` (`0x08A8A`); `0Ah`, the figure under an
animation of its own, takes the mover's position (`0A9Fh:0B4Ch`,
`0x08A98`) and then runs the stream (`0x08AAF`). The tail puts the
interval back on the countdown (`0x08AF7`) and goes round again while
none is left (`0x08B02`). The walk's own states are the walker's
([walking](walking.md)).

`0883h:0CBBh` is the stream and then the entry's frame into the record:
hidden for `FFh` (`0x0951E`), else `+9` and the cel `+0Ah`
(`0x09564`). The stream itself is the interpreter `0883h:0642h`, which
runs from a queued animation's takeover through the opcodes until one of
them sets a frame and its wait.

## The stream

| Opcode | Bytes | What |
|---|---|---|
| `00` | 1 | end: the animation asked for next, or idle |
| `01` | 1 | return: pop a frame and go where its opcode says |
| `02` | 1 | next: the same without popping a loop's frame |
| `03 tt tt` | 3 | push a frame that returns to `tt` |
| `04 tt tt` | 3 | call `tt`, skipping a `03` there |
| `05` … `09` | 1, 1, 2, n+2, 4n+2 | when the next animation may take over |
| `0A t` | 2 | the longest wait once one is asked for |
| `0B n tt tt` | 4 | loop `n` times, out at `tt` |
| `0C tt tt` | 3 | loop forever, out at `tt` |
| `0D tt tt` | 3 | jump |
| `0E n (tt tt)×n` | 2n+2 | random jump |
| `0F n (tt tt)×n` | 2n+2 | random call |
| `10 d`, `11 lo hi` | 2, 3 | the default and the random wait |
| `12`–`15` | 3 | move the object by, or to |
| `16 dx dy w h` | 9 | the hotspot follows the object |
| `17 cc cc` | 3 | signal code `cc` |
| `18 m` | 2 | mirror off, on, toggled |
| `19`–`1C` | 3 | the scale set or moved |
| `80`–`85` | 2–4 | one frame, with its wait |
| `88`–`8D` | | a sequence forward |
| `90`–`95` | | a sequence forward and back |
| `98`–`9D` | | a random walk through a sequence |
| `A0`–`A5` | | one frame drawn from a list |

The twenty-nine opcodes from `00` to `1C` are dispatched through a table
in the data segment, `DS:0474h` (`0x0902F`); the classes from `80h` up are
a five-way match on `op & F8h` (`0x08F8A`) and a jump on `op & 7` inside
each.

**Calls and loops** share one stack of the offsets of the opcodes that
pushed, thirty-two deep (`0x0A238`). The return (`0x09610`) and the next
(`0x0973A`) dispatch on the opcode that pushed the top frame: `03` and
`0C` return to the address in the word after the pushing opcode, `04`
three bytes past it, `0B` out of the loop to the address at its `+2`,
`0F` past its table of words; a next on a counted loop whose count is not
spent goes back into the body and keeps the frame (`0x0977B`), and on a
loop without a count it always does (`0x0977F`).

**Frames and sequences.** `80`–`85` set one frame and its wait and leave
the entry after a frame (`0x08FB6`–`0x0904F`); `A0`–`A5` draw the frame
from a list with the game's generator (`0x090B7`–`0x0927B`). A sequence
runs its frames forward (`0x092B4`), or forward and back down to its
second frame (`0x09355`), or wanders: one step up or down at a time, the
step drawn as `19D2h(−15, 31) >> 4 | 1` (`0x093EB`), the frame shown the
one stood on before the step, for as many steps as the repeat count
(`0x0939F`). A wait between two bounds is drawn the same way, the lower
one when they are not in order (`0x0A20E`).

## Starting one, and taking over

`0883h:1F82h(obj, k)` asks for animation `k` on an object. Only states 7
to `0Bh` take it, and `0Ah` does nothing (`0x0A7BE`); a `k` past the
block's count puts the object in state 0 (`0x0A7FE`). An object that is
not live — a door — comes up hidden (`0x0A84C`). An idle entry starts at
once with its countdown at 0 (`0x0A862`); a busy one has the animation
asked for in `+6` (`0x0A889`) and its wait capped. `0883h:2099h(id, k)`
does the same by the object's id, `0883h:20B3h(id)` shows the object and
plays its animation 0 (`0x0A8EC`), and `0883h:20D9h(1693h, k)` plays
animation `k` on the figure, binding it to its entry first when it has
none (`14DDh:0098h`).

Whether the animation asked for may take over is `0883h:0486h`. An idle
entry may at once (`0x08CBD`); one that is still fetching may not
(`0x08CC7`). Otherwise the condition the stream set with `05`–`09`
decides (`0x08CE0`): on a given frame (`0x08D01`), on a frame in a list
(`0x08D3D`), or through a transition table that names an animation to
play first (`0x08D7F`). `0883h:0582h` starts animation `k` from its first
opcode with the stack and the loops cleared (`0x08DF8`–`0x08E35`).

When an animation is over, `0883h:0D86h` starts the one asked for next;
with none, the entry idles and its record is looked at again in two
seconds, a wait of `0F0h` ticks (`0x095F2`).

## What an entry keeps

An entry's working state lives in its own bytes, `+5` to `+73h`: the
animation playing and the one asked for (`+5`, `+6`), where the
interpreter is (`+7`: idle, fetching, after a frame, forward, back,
wandering), the head's frame (`+9`, `FFh` for hidden), the takeover
condition (`+10h`), the stack (`+18h`), the stream (`+68h`) and the body
it drives (`+70h`). The global block's entries count their offsets from
`[1ED4h]` (`0x08DBC`). An entry is bound only to the 32-byte objects
(`14DDh:0007h`, `0x14DD7`); a room's 30h-byte people are bodies of the
chains alone.

## Chains

A chain is an object made of parts: a head and the records placed
against it. `0883h:1A82h` gives the head its frame for this frame of the
chain and places every part. The head's node decides its cel — kind 0
takes the node's cel for the frame (`0x0A3DC`), kind `100h` one whose
kind-6 frame is a position (`0x0A426`) — and each node's form, its high
byte (`0x0A5C9`), picks the placement from a jump table (`0x0A6E0`), form
0 skipping the node. A part with a cel is shown there (`0x0A760`), one
with `FFh` hidden (`0x0A783`). Moving the head moves the chain with it
(`0x0A283`–`0x0A2AD`).

## What the body does

`12`–`15` move the object by an offset or to a point; `18` mirrors it;
`19`–`1C` set or move its scale. `16` puts the hotspot that names the
object on the object's box, moved by the offsets and sized by the extents
where they are given and by the cel where they are 0
(`0x09C68`–`0x09DE0`): a door's hotspot follows the door.

`17 cc cc` is how an animation tells the location something. The code
goes through `0EB9h:1D97h` and on to the location's own hook at
`[1E60h]`, with the object's id — `FFh` for the figure — and the code;
the Bogenstraße, for one, waits on the code an animation signals before
it lets its scene go on.

## See also

- [Animations](../formats/animations.md) — the `AN` block in the file
- [Objects](objects.md) — the records an entry binds, and the entry pass that binds them
- [Walking](walking.md) — the walk's own steps, which the animator times
- [Game loop](game-loop.md) — the frame pass and the tick
