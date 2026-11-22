[← Documentation index](../README.md)

# PSM 2

*A component: the PSM 2 music format and the drivers that play it, as their own files show them — `MUSADL.DRV`, the Ad Lib driver, in the two builds that are read, the sample path of the `DMA*.DRV` digital drivers, and the mixer of `DMABLAST.DRV` with a song's digital voices. What a program hosting the drivers does around them — which song it names, when it fades, how long it waits — is the hosting family's, and is documented in that family's tree.*

PSM 2 is a music system that games of the time shipped as files of their
own: a module format and a resident driver per sound device. Nothing in these files belongs to the engine of a game that ships
them: a host loads the driver file as it is, calls its entries and serves
its timer, and what the format means and what the driver does are read
out of those files, not out of the host.
That is why they are documented as a component, apart from any family's
tree, and why a family's pages say only what its engine does with them.

| Page | What it covers |
|---|---|
| [The format](format.md) | The `MTCVTS` module, the `PLX` song section with its events and instrument records, the `MDH` digital part with its voices and event stream, and the `SM8` sample header |
| [The drivers](drivers.md) | `MUSADL.DRV`: header, builds, tables, register shadow, tick, volume, fade, stop and its callback cell; the `DMA*.DRV` drivers' sample path; `DMABLAST.DRV`'s mixer, its blocks and buffer, and a song's digital voices in it |

The checks of the rebuild against recordings are recordings of particular
games, so they are written where those games are:
[MOTION 16-bit's PSM 2 page](../motion/motion16/formats/psm-music.md#how-the-rebuild-is-checked)
holds MOTION's, and [PARSEC's verification page](../parsec/verification.md)
Dunkle Schatten's.

No path of the rebuild stops by address, so the component carries no
ledger of open questions or departures of its own; what is unread is
named on the [drivers page](drivers.md#open-questions), and where the
rebuild decides what no file says — how the mixer's clock meets the
music's, and the DAC's level against the OPL — is said
[there](drivers.md#how-the-rebuild-meets-the-two-clocks) too.
