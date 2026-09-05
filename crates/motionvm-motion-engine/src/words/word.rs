//! Every kernel word the engine implements, as one value each.
//!
//! The engine is asked for a word by *ordinal*, and answers by looking the
//! ordinal up in a table it built once when the game opened. This enum is what
//! that table holds, and this file is the whole of the name → meaning map:
//! two matches on string literals, one per engine generation, and nothing
//! anywhere else compares a word to a name.
//!
//! ## Why a value and not a name
//!
//! A word that arrived as a `&str` and was offered to fifteen or seventeen
//! groups in turn, each a `match name`, the first to recognize it winning,
//! would make the order of those calls load-bearing with nothing checking it:
//! two groups could claim one name and the earlier one would win silently.
//! That is not hypothetical — **eleven names mean different things on the two
//! machines**, and a call order would be the only thing telling them apart:
//!
//! | name | 32-bit | 16-bit |
//! |---|---|---|
//! | `FADEIN`, `FADEOUT` | a band curtain | a box wipe (`05f1:2827`, `05f1:29e4`) |
//! | `SETBUF`, `RESETBUF`, `SDBUF` | inert, or a descriptor field | real buffers |
//! | `SCRX`, `SCRPOS`, `GSCRX`, `GSCRY` | the shared screen words | this engine's own |
//! | `STARTTUNE`, `ENDTUNE` | start and stop | with `ENDTUNE`'s half-second wait |
//! | `=>ERASE` | frees the slot | frees the slot and unloads the module |
//!
//! Each of those is two values — the 16-bit one carrying a `_16` suffix —
//! decided by which resolver ran, so the difference is in the type rather than
//! in the order of a call chain. A group that does not know a value answers
//! `None`, and nothing depends on which group is asked first.
//!
//! A duplicate name is a compile error: [`Word::of_m32`] and
//! [`Word::of_m16`] are single matches on literals, and a literal written
//! twice is an unreachable pattern.
//!
//! ## Naming
//!
//! The variants are the kernel's own spelling, which is how every page under
//! `docs/`, every comment here and the disassembler's `kernel-usage.txt` write
//! them — `Word::SDACTIVE`, not `Word::SdActive`. A name that is not an
//! identifier is transliterated by a fixed rule and carries its real spelling
//! in a doc comment: `?` becomes `Q_`, `=>` becomes `RES_`, `->` becomes
//! `TO_`, a leading `+` or `-` becomes `PLUS_` or `MINUS_`, a trailing one
//! `_PLUS` or `_MINUS`, `%` becomes `_PCT_`, `.` becomes `DOT`, and a name
//! starting with a digit takes a `MODE_` prefix. [`Word::name`] answers the
//! real spelling, and a test holds the two matches against each other.

use crate::Field;

/// One kernel word the engine implements.
///
/// Grouped by the file that handles it, in the order the dispatchers ask —
/// which is a matter of taste rather than of correctness.
#[expect(
    clippy::upper_case_acronyms,
    reason = "the same rule as the one below: a word's spelling is the kernel's"
)]
#[expect(
    non_camel_case_types,
    reason = "the kernel's own spelling, which is what the disassembly, the \
              documentation and every comment in this crate use; a camel-cased \
              variant would have to be mentally transliterated at every site"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Word {
    // Answered where the machine itself is needed.
    /// `=>ERASE`
    RES_ERASE_16,
    /// `=>GET`
    RES_GET,
    DOORDER,
    REQUEST,
    SCRCTRL,

    // Video mode, the subsystem flags, and the two block-file words.
    /// `320x200x256`
    MODE_320X200X256,
    /// `640x480x256`
    MODE_640X480X256,
    /// `640x480x32K`
    MODE_640X480X32K,
    /// `?SOUND`
    Q_SOUND,
    CTRL,
    DREQUEST,
    ERRORLEVEL,
    GET,
    GFXTO,
    HICOLOR,
    PUT,
    RESETANIM,
    RESETFONT,
    RESETTI,
    SETRES,
    SPEEDMODE,
    TOGFX,

    // Screens: making them, sizing them, moving their views.
    ACTSCR,
    FREEZESCR,
    GSCRACT,
    GSCRVSIZE,
    GSCRX,
    GSCRY,
    NEWSCREEN,
    SCRFVSIZE,
    SCRPOS,
    SCRSIZE,
    SCRVPOS,
    SCRVSIZE,
    SCRX,
    UNFREEZESCR,

    // Descriptors: making them, selecting them, setting and getting their fields.
    /// `?ACTDESC`
    Q_ACTDESC,
    ACTDESC,
    GDACTIVE,
    GDBL,
    GDCOL,
    GDCX,
    GDCY,
    GDHEIGHT,
    GDLEV,
    GDLV,
    GDNR,
    GDOX,
    GDOY,
    GDSPR,
    GDTB,
    GDTEXTLEN,
    GDTXT,
    GDTXTLEN,
    GDWIDTH,
    GDX,
    GDXLEN,
    GDY,
    GDYLEN,
    GDZ,
    KILLDESC,
    KILLNDESC,
    NEWDESC,
    NEWSETDESC,
    /// `SD%SHR`
    SD_PCT_SHR,
    SDACTIVE,
    SDALINES,
    SDAUTOBUF,
    SDBL,
    SDBLK,
    SDBUF,
    SDCEN,
    SDCOL,
    SDCX,
    SDCY,
    SDFNT,
    /// `SDH%SHR`
    SDH_PCT_SHR,
    SDINACTIVE,
    SDINSERT,
    SDLEV,
    SDLV,
    SDNORM,
    SDOX,
    SDOY,
    SDPOS,
    SDSHADE,
    SDSPR,
    SDSTARTLINE,
    SDTB,
    SDTDT,
    SDTRANS,
    SDTXT,
    /// `SDV%SHR`
    SDV_PCT_SHR,
    SDVCEN,
    SDWORD,
    SDWAIT,
    SDX,
    SDY,
    SDZ,

    // Fonts and the text defaults.
    /// `+FONT`
    PLUS_FONT,
    DEFTDT,

    // The savegame words.
    /// `=>ERASE`
    RES_ERASE,
    /// `=>GETAS`
    RES_GETAS,
    /// `=>PUTAS`
    RES_PUTAS,
    GETANIM,
    PUTANIM,

    // The resource loader and the status hints.
    BLKSTAT,
    /// `BLKSTAT+`
    BLKSTAT_PLUS,
    /// `BLKSTAT-`
    BLKSTAT_MINUS,
    EXIST,
    FNTSTAT,
    /// `FNTSTAT+`
    FNTSTAT_PLUS,
    /// `FNTSTAT-`
    FNTSTAT_MINUS,
    GFXCRUNCH,
    GFXSTAT,
    /// `GFXSTAT+`
    GFXSTAT_PLUS,
    /// `GFXSTAT-`
    GFXSTAT_MINUS,
    GFXVFLIP,
    PALSTAT,
    /// `PALSTAT+`
    PALSTAT_PLUS,
    /// `PALSTAT-`
    PALSTAT_MINUS,
    SCRSTAT,
    /// `SCRSTAT+`
    SCRSTAT_PLUS,
    /// `SCRSTAT-`
    SCRSTAT_MINUS,
    TXTSTAT,
    /// `TXTSTAT+`
    TXTSTAT_PLUS,
    /// `TXTSTAT-`
    TXTSTAT_MINUS,
    XBLKSTAT,
    /// `XBLKSTAT+`
    XBLKSTAT_PLUS,
    /// `XBLKSTAT-`
    XBLKSTAT_MINUS,
    XFNTSTAT,
    /// `XFNTSTAT+`
    XFNTSTAT_PLUS,
    /// `XFNTSTAT-`
    XFNTSTAT_MINUS,
    XGFXCRUNCH,
    XGFXSTAT,
    /// `XGFXSTAT+`
    XGFXSTAT_PLUS,
    /// `XGFXSTAT-`
    XGFXSTAT_MINUS,
    XGFXVFLIP,
    XPALSTAT,
    /// `XPALSTAT+`
    XPALSTAT_PLUS,
    /// `XPALSTAT-`
    XPALSTAT_MINUS,
    XSCRSTAT,
    /// `XSCRSTAT+`
    XSCRSTAT_PLUS,
    /// `XSCRSTAT-`
    XSCRSTAT_MINUS,
    XTXTSTAT,
    /// `XTXTSTAT+`
    XTXTSTAT_PLUS,
    /// `XTXTSTAT-`
    XTXTSTAT_MINUS,

    // The curtains.
    FADEIN,
    FADEOUT,

    // The walk.
    DOWALK,

    // The frame, the clock, and what the player pressed.
    /// `?KEY`
    Q_KEY,
    ANIMPLAY,
    DELAY,
    QUITANIM,
    STEPMULTI,

    // The inventory list.
    /// `?INVINCL`
    Q_INVINCL,
    ADDTOINV,
    CCALCINV,
    SUBFROMINV,

    // The dialogue queue.
    ADDMESSPIPE,

    // Music.
    ENDTUNE,
    STARTTUNE,

    // The pointer.
    /// `?INSIDE`
    Q_INSIDE,
    /// `?XINSIDE`
    Q_XINSIDE,
    ATMOUSE,
    HIDEMOUSE,
    MOUSEINFO,
    MOUSELK,
    MOUSERK,
    MOUSEX,
    MOUSEXY,
    MOUSEY,
    NORMMOUSE,
    SETMOUSELB,
    SETMOUSERB,
    SETMOUSEX,
    SETMOUSEY,
    SHOWMOUSE,
    XATMOUSE,

    // The palette.
    CUTPAL,
    /// `RGB->COL`
    RGB_TO_COL,
    SETPAL,

    // Redrawing, and the buffer words the 32-bit engine walks past.
    DRAWSCR,
    ERASESCR,
    FRESHSCREEN,
    REMSCR,
    RESETBUF,
    SCRACT,
    SCRINACT,
    SETBUF,

    // The 16-bit engine's own words, and its own readings of shared ones.
    /// `->SCRX`
    TO_SCRX,
    /// `->SCRY`
    TO_SCRY,
    /// `-FONT`
    MINUS_FONT,
    /// `.`
    DOT,
    /// `=>EXIST`
    RES_EXIST,
    CROUTE,
    EMIT,
    /// `ENDTUNE`
    ENDTUNE_16,
    /// `FADEIN`
    FADEIN_16,
    /// `FADEOUT`
    FADEOUT_16,
    GSCRPOS,
    /// `GSCRX`
    GSCRX_16,
    /// `GSCRY`
    GSCRY_16,
    KEY,
    NEWANIM,
    /// `SCRPOS`
    SCRPOS_16,
    /// `SCRX`
    SCRX_16,
    SCRY,
    SETCYCLE,
    SETSHADE,
    SFT,
    /// `STARTTUNE`
    STARTTUNE_16,
    SYSBC,
    SYSFC,
    /// `_POOR`
    POOR,

    // The 16-bit engine's real buffer words.
    BUFON,
    KILLNBUF,
    /// `RESETBUF`
    RESETBUF_16,
    /// `SDBUF`
    SDBUF_16,
    /// `SETBUF`
    SETBUF_16,
}

impl Word {
    /// Every variant, so a test can walk them.
    #[cfg(test)]
    pub(crate) const ALL: &'static [Word] = &[
        Word::PLUS_FONT,
        Word::TO_SCRX,
        Word::TO_SCRY,
        Word::MINUS_FONT,
        Word::DOT,
        Word::MODE_320X200X256,
        Word::MODE_640X480X256,
        Word::MODE_640X480X32K,
        Word::RES_ERASE,
        Word::RES_ERASE_16,
        Word::RES_EXIST,
        Word::RES_GET,
        Word::RES_GETAS,
        Word::RES_PUTAS,
        Word::Q_ACTDESC,
        Word::Q_INSIDE,
        Word::Q_INVINCL,
        Word::Q_KEY,
        Word::Q_SOUND,
        Word::Q_XINSIDE,
        Word::ACTDESC,
        Word::ACTSCR,
        Word::ADDMESSPIPE,
        Word::ADDTOINV,
        Word::ANIMPLAY,
        Word::ATMOUSE,
        Word::BLKSTAT,
        Word::BLKSTAT_PLUS,
        Word::BLKSTAT_MINUS,
        Word::BUFON,
        Word::CCALCINV,
        Word::CROUTE,
        Word::CTRL,
        Word::CUTPAL,
        Word::DEFTDT,
        Word::DELAY,
        Word::DOORDER,
        Word::DOWALK,
        Word::DRAWSCR,
        Word::DREQUEST,
        Word::EMIT,
        Word::ENDTUNE,
        Word::ENDTUNE_16,
        Word::ERASESCR,
        Word::ERRORLEVEL,
        Word::EXIST,
        Word::FADEIN,
        Word::FADEIN_16,
        Word::FADEOUT,
        Word::FADEOUT_16,
        Word::FNTSTAT,
        Word::FNTSTAT_PLUS,
        Word::FNTSTAT_MINUS,
        Word::FREEZESCR,
        Word::FRESHSCREEN,
        Word::GDACTIVE,
        Word::GDBL,
        Word::GDCOL,
        Word::GDCX,
        Word::GDCY,
        Word::GDHEIGHT,
        Word::GDLEV,
        Word::GDLV,
        Word::GDNR,
        Word::GDOX,
        Word::GDOY,
        Word::GDSPR,
        Word::GDTB,
        Word::GDTEXTLEN,
        Word::GDTXT,
        Word::GDTXTLEN,
        Word::GDWIDTH,
        Word::GDX,
        Word::GDXLEN,
        Word::GDY,
        Word::GDYLEN,
        Word::GDZ,
        Word::GET,
        Word::GETANIM,
        Word::GFXCRUNCH,
        Word::GFXSTAT,
        Word::GFXSTAT_PLUS,
        Word::GFXSTAT_MINUS,
        Word::GFXTO,
        Word::GFXVFLIP,
        Word::GSCRACT,
        Word::GSCRPOS,
        Word::GSCRVSIZE,
        Word::GSCRX,
        Word::GSCRX_16,
        Word::GSCRY,
        Word::GSCRY_16,
        Word::HICOLOR,
        Word::HIDEMOUSE,
        Word::KEY,
        Word::KILLDESC,
        Word::KILLNBUF,
        Word::KILLNDESC,
        Word::MOUSEINFO,
        Word::MOUSELK,
        Word::MOUSERK,
        Word::MOUSEX,
        Word::MOUSEXY,
        Word::MOUSEY,
        Word::NEWANIM,
        Word::NEWDESC,
        Word::NEWSCREEN,
        Word::NEWSETDESC,
        Word::NORMMOUSE,
        Word::PALSTAT,
        Word::PALSTAT_PLUS,
        Word::PALSTAT_MINUS,
        Word::PUT,
        Word::PUTANIM,
        Word::QUITANIM,
        Word::REMSCR,
        Word::REQUEST,
        Word::RESETANIM,
        Word::RESETBUF,
        Word::RESETBUF_16,
        Word::RESETFONT,
        Word::RESETTI,
        Word::RGB_TO_COL,
        Word::SCRACT,
        Word::SCRCTRL,
        Word::SCRFVSIZE,
        Word::SCRINACT,
        Word::SCRPOS,
        Word::SCRPOS_16,
        Word::SCRSIZE,
        Word::SCRSTAT,
        Word::SCRSTAT_PLUS,
        Word::SCRSTAT_MINUS,
        Word::SCRVPOS,
        Word::SCRVSIZE,
        Word::SCRX,
        Word::SCRX_16,
        Word::SCRY,
        Word::SD_PCT_SHR,
        Word::SDACTIVE,
        Word::SDALINES,
        Word::SDAUTOBUF,
        Word::SDBL,
        Word::SDBLK,
        Word::SDBUF,
        Word::SDBUF_16,
        Word::SDCEN,
        Word::SDCOL,
        Word::SDCX,
        Word::SDCY,
        Word::SDFNT,
        Word::SDH_PCT_SHR,
        Word::SDINACTIVE,
        Word::SDINSERT,
        Word::SDLEV,
        Word::SDLV,
        Word::SDNORM,
        Word::SDOX,
        Word::SDOY,
        Word::SDPOS,
        Word::SDSHADE,
        Word::SDSPR,
        Word::SDSTARTLINE,
        Word::SDTB,
        Word::SDTDT,
        Word::SDTRANS,
        Word::SDTXT,
        Word::SDV_PCT_SHR,
        Word::SDVCEN,
        Word::SDWORD,
        Word::SDWAIT,
        Word::SDX,
        Word::SDY,
        Word::SDZ,
        Word::SETBUF,
        Word::SETBUF_16,
        Word::SETCYCLE,
        Word::SETMOUSELB,
        Word::SETMOUSERB,
        Word::SETMOUSEX,
        Word::SETMOUSEY,
        Word::SETPAL,
        Word::SETRES,
        Word::SETSHADE,
        Word::SFT,
        Word::SHOWMOUSE,
        Word::SPEEDMODE,
        Word::STARTTUNE,
        Word::STARTTUNE_16,
        Word::STEPMULTI,
        Word::SUBFROMINV,
        Word::SYSBC,
        Word::SYSFC,
        Word::TOGFX,
        Word::TXTSTAT,
        Word::TXTSTAT_PLUS,
        Word::TXTSTAT_MINUS,
        Word::UNFREEZESCR,
        Word::XATMOUSE,
        Word::XBLKSTAT,
        Word::XBLKSTAT_PLUS,
        Word::XBLKSTAT_MINUS,
        Word::XFNTSTAT,
        Word::XFNTSTAT_PLUS,
        Word::XFNTSTAT_MINUS,
        Word::XGFXCRUNCH,
        Word::XGFXSTAT,
        Word::XGFXSTAT_PLUS,
        Word::XGFXSTAT_MINUS,
        Word::XGFXVFLIP,
        Word::XPALSTAT,
        Word::XPALSTAT_PLUS,
        Word::XPALSTAT_MINUS,
        Word::XSCRSTAT,
        Word::XSCRSTAT_PLUS,
        Word::XSCRSTAT_MINUS,
        Word::XTXTSTAT,
        Word::XTXTSTAT_PLUS,
        Word::XTXTSTAT_MINUS,
        Word::POOR,
    ];

    /// What a name means to the **32-bit** kernel, or `None` for a word this
    /// engine does not implement.
    ///
    /// Called once per kernel word when a game opens, never on a frame.
    pub(crate) fn of_m32(name: &str) -> Option<Word> {
        Some(match name {
            "+FONT" => Word::PLUS_FONT,
            "320x200x256" => Word::MODE_320X200X256,
            "640x480x256" => Word::MODE_640X480X256,
            "640x480x32K" => Word::MODE_640X480X32K,
            "=>ERASE" => Word::RES_ERASE,
            "=>GET" => Word::RES_GET,
            "=>GETAS" => Word::RES_GETAS,
            "=>PUTAS" => Word::RES_PUTAS,
            "?ACTDESC" => Word::Q_ACTDESC,
            "?INSIDE" => Word::Q_INSIDE,
            "?INVINCL" => Word::Q_INVINCL,
            "?KEY" => Word::Q_KEY,
            "?SOUND" => Word::Q_SOUND,
            "?XINSIDE" => Word::Q_XINSIDE,
            "ACTDESC" => Word::ACTDESC,
            "ACTSCR" => Word::ACTSCR,
            "ADDMESSPIPE" => Word::ADDMESSPIPE,
            "ADDTOINV" => Word::ADDTOINV,
            "ANIMPLAY" => Word::ANIMPLAY,
            "ATMOUSE" => Word::ATMOUSE,
            "BLKSTAT" => Word::BLKSTAT,
            "BLKSTAT+" => Word::BLKSTAT_PLUS,
            "BLKSTAT-" => Word::BLKSTAT_MINUS,
            "CCALCINV" => Word::CCALCINV,
            "CTRL" => Word::CTRL,
            "CUTPAL" => Word::CUTPAL,
            "DEFTDT" => Word::DEFTDT,
            "DELAY" => Word::DELAY,
            "DOORDER" => Word::DOORDER,
            "DOWALK" => Word::DOWALK,
            "DRAWSCR" => Word::DRAWSCR,
            "DREQUEST" => Word::DREQUEST,
            "ENDTUNE" => Word::ENDTUNE,
            "ERASESCR" => Word::ERASESCR,
            "ERRORLEVEL" => Word::ERRORLEVEL,
            "EXIST" => Word::EXIST,
            "FADEIN" => Word::FADEIN,
            "FADEOUT" => Word::FADEOUT,
            "FNTSTAT" => Word::FNTSTAT,
            "FNTSTAT+" => Word::FNTSTAT_PLUS,
            "FNTSTAT-" => Word::FNTSTAT_MINUS,
            "FREEZESCR" => Word::FREEZESCR,
            "FRESHSCREEN" => Word::FRESHSCREEN,
            "GDACTIVE" => Word::GDACTIVE,
            "GDBL" => Word::GDBL,
            "GDCOL" => Word::GDCOL,
            "GDCX" => Word::GDCX,
            "GDCY" => Word::GDCY,
            "GDHEIGHT" => Word::GDHEIGHT,
            "GDLEV" => Word::GDLEV,
            "GDLV" => Word::GDLV,
            "GDNR" => Word::GDNR,
            "GDOX" => Word::GDOX,
            "GDOY" => Word::GDOY,
            "GDSPR" => Word::GDSPR,
            "GDTB" => Word::GDTB,
            "GDTEXTLEN" => Word::GDTEXTLEN,
            "GDTXT" => Word::GDTXT,
            "GDTXTLEN" => Word::GDTXTLEN,
            "GDWIDTH" => Word::GDWIDTH,
            "GDX" => Word::GDX,
            "GDXLEN" => Word::GDXLEN,
            "GDY" => Word::GDY,
            "GDYLEN" => Word::GDYLEN,
            "GDZ" => Word::GDZ,
            "GET" => Word::GET,
            "GETANIM" => Word::GETANIM,
            "GFXCRUNCH" => Word::GFXCRUNCH,
            "GFXSTAT" => Word::GFXSTAT,
            "GFXSTAT+" => Word::GFXSTAT_PLUS,
            "GFXSTAT-" => Word::GFXSTAT_MINUS,
            "GFXTO" => Word::GFXTO,
            "GFXVFLIP" => Word::GFXVFLIP,
            "GSCRACT" => Word::GSCRACT,
            "GSCRVSIZE" => Word::GSCRVSIZE,
            "GSCRX" => Word::GSCRX,
            "GSCRY" => Word::GSCRY,
            "HICOLOR" => Word::HICOLOR,
            "HIDEMOUSE" => Word::HIDEMOUSE,
            "KILLDESC" => Word::KILLDESC,
            "KILLNDESC" => Word::KILLNDESC,
            "MOUSEINFO" => Word::MOUSEINFO,
            "MOUSELK" => Word::MOUSELK,
            "MOUSERK" => Word::MOUSERK,
            "MOUSEX" => Word::MOUSEX,
            "MOUSEXY" => Word::MOUSEXY,
            "MOUSEY" => Word::MOUSEY,
            "NEWDESC" => Word::NEWDESC,
            "NEWSCREEN" => Word::NEWSCREEN,
            "NEWSETDESC" => Word::NEWSETDESC,
            "NORMMOUSE" => Word::NORMMOUSE,
            "PALSTAT" => Word::PALSTAT,
            "PALSTAT+" => Word::PALSTAT_PLUS,
            "PALSTAT-" => Word::PALSTAT_MINUS,
            "PUT" => Word::PUT,
            "PUTANIM" => Word::PUTANIM,
            "QUITANIM" => Word::QUITANIM,
            "REMSCR" => Word::REMSCR,
            "REQUEST" => Word::REQUEST,
            "RESETANIM" => Word::RESETANIM,
            "RESETBUF" => Word::RESETBUF,
            "RESETFONT" => Word::RESETFONT,
            "RESETTI" => Word::RESETTI,
            "RGB->COL" => Word::RGB_TO_COL,
            "SCRACT" => Word::SCRACT,
            "SCRCTRL" => Word::SCRCTRL,
            "SCRFVSIZE" => Word::SCRFVSIZE,
            "SCRINACT" => Word::SCRINACT,
            "SCRPOS" => Word::SCRPOS,
            "SCRSIZE" => Word::SCRSIZE,
            "SCRSTAT" => Word::SCRSTAT,
            "SCRSTAT+" => Word::SCRSTAT_PLUS,
            "SCRSTAT-" => Word::SCRSTAT_MINUS,
            "SCRVPOS" => Word::SCRVPOS,
            "SCRVSIZE" => Word::SCRVSIZE,
            "SCRX" => Word::SCRX,
            "SD%SHR" => Word::SD_PCT_SHR,
            "SDACTIVE" => Word::SDACTIVE,
            "SDALINES" => Word::SDALINES,
            "SDAUTOBUF" => Word::SDAUTOBUF,
            "SDBL" => Word::SDBL,
            "SDBLK" => Word::SDBLK,
            "SDBUF" => Word::SDBUF,
            "SDCEN" => Word::SDCEN,
            "SDCOL" => Word::SDCOL,
            "SDCX" => Word::SDCX,
            "SDCY" => Word::SDCY,
            "SDFNT" => Word::SDFNT,
            "SDH%SHR" => Word::SDH_PCT_SHR,
            "SDINACTIVE" => Word::SDINACTIVE,
            "SDINSERT" => Word::SDINSERT,
            "SDLEV" => Word::SDLEV,
            "SDLV" => Word::SDLV,
            "SDNORM" => Word::SDNORM,
            "SDOX" => Word::SDOX,
            "SDOY" => Word::SDOY,
            "SDPOS" => Word::SDPOS,
            "SDSHADE" => Word::SDSHADE,
            "SDSPR" => Word::SDSPR,
            "SDSTARTLINE" => Word::SDSTARTLINE,
            "SDTB" => Word::SDTB,
            "SDTDT" => Word::SDTDT,
            "SDTRANS" => Word::SDTRANS,
            "SDTXT" => Word::SDTXT,
            "SDV%SHR" => Word::SDV_PCT_SHR,
            "SDVCEN" => Word::SDVCEN,
            "SDWORD" => Word::SDWORD,
            "SDWAIT" => Word::SDWAIT,
            "SDX" => Word::SDX,
            "SDY" => Word::SDY,
            "SDZ" => Word::SDZ,
            "SETBUF" => Word::SETBUF,
            "SETMOUSELB" => Word::SETMOUSELB,
            "SETMOUSERB" => Word::SETMOUSERB,
            "SETMOUSEX" => Word::SETMOUSEX,
            "SETMOUSEY" => Word::SETMOUSEY,
            "SETPAL" => Word::SETPAL,
            "SETRES" => Word::SETRES,
            "SHOWMOUSE" => Word::SHOWMOUSE,
            "SPEEDMODE" => Word::SPEEDMODE,
            "STARTTUNE" => Word::STARTTUNE,
            "STEPMULTI" => Word::STEPMULTI,
            "SUBFROMINV" => Word::SUBFROMINV,
            "TOGFX" => Word::TOGFX,
            "TXTSTAT" => Word::TXTSTAT,
            "TXTSTAT+" => Word::TXTSTAT_PLUS,
            "TXTSTAT-" => Word::TXTSTAT_MINUS,
            "UNFREEZESCR" => Word::UNFREEZESCR,
            "XATMOUSE" => Word::XATMOUSE,
            "XBLKSTAT" => Word::XBLKSTAT,
            "XBLKSTAT+" => Word::XBLKSTAT_PLUS,
            "XBLKSTAT-" => Word::XBLKSTAT_MINUS,
            "XFNTSTAT" => Word::XFNTSTAT,
            "XFNTSTAT+" => Word::XFNTSTAT_PLUS,
            "XFNTSTAT-" => Word::XFNTSTAT_MINUS,
            "XGFXCRUNCH" => Word::XGFXCRUNCH,
            "XGFXSTAT" => Word::XGFXSTAT,
            "XGFXSTAT+" => Word::XGFXSTAT_PLUS,
            "XGFXSTAT-" => Word::XGFXSTAT_MINUS,
            "XGFXVFLIP" => Word::XGFXVFLIP,
            "XPALSTAT" => Word::XPALSTAT,
            "XPALSTAT+" => Word::XPALSTAT_PLUS,
            "XPALSTAT-" => Word::XPALSTAT_MINUS,
            "XSCRSTAT" => Word::XSCRSTAT,
            "XSCRSTAT+" => Word::XSCRSTAT_PLUS,
            "XSCRSTAT-" => Word::XSCRSTAT_MINUS,
            "XTXTSTAT" => Word::XTXTSTAT,
            "XTXTSTAT+" => Word::XTXTSTAT_PLUS,
            "XTXTSTAT-" => Word::XTXTSTAT_MINUS,
            _ => return None,
        })
    }

    /// What a name means to the **16-bit** kernel. The eleven words whose
    /// handler differs answer their `_16` variant here; everything else
    /// answers the same value [`Word::of_m32`] does.
    pub(crate) fn of_m16(name: &str) -> Option<Word> {
        Some(match name {
            "+FONT" => Word::PLUS_FONT,
            "->SCRX" => Word::TO_SCRX,
            "->SCRY" => Word::TO_SCRY,
            "-FONT" => Word::MINUS_FONT,
            "." => Word::DOT,
            "320x200x256" => Word::MODE_320X200X256,
            "640x480x256" => Word::MODE_640X480X256,
            "640x480x32K" => Word::MODE_640X480X32K,
            "=>ERASE" => Word::RES_ERASE_16,
            "=>EXIST" => Word::RES_EXIST,
            "=>GET" => Word::RES_GET,
            "=>GETAS" => Word::RES_GETAS,
            "=>PUTAS" => Word::RES_PUTAS,
            "?ACTDESC" => Word::Q_ACTDESC,
            "?INSIDE" => Word::Q_INSIDE,
            "?INVINCL" => Word::Q_INVINCL,
            "?KEY" => Word::Q_KEY,
            "?SOUND" => Word::Q_SOUND,
            "?XINSIDE" => Word::Q_XINSIDE,
            "ACTDESC" => Word::ACTDESC,
            "ACTSCR" => Word::ACTSCR,
            "ADDMESSPIPE" => Word::ADDMESSPIPE,
            "ADDTOINV" => Word::ADDTOINV,
            "ANIMPLAY" => Word::ANIMPLAY,
            "ATMOUSE" => Word::ATMOUSE,
            "BLKSTAT" => Word::BLKSTAT,
            "BLKSTAT+" => Word::BLKSTAT_PLUS,
            "BLKSTAT-" => Word::BLKSTAT_MINUS,
            "BUFON" => Word::BUFON,
            "CCALCINV" => Word::CCALCINV,
            "CROUTE" => Word::CROUTE,
            "CTRL" => Word::CTRL,
            "CUTPAL" => Word::CUTPAL,
            "DEFTDT" => Word::DEFTDT,
            "DELAY" => Word::DELAY,
            "DOORDER" => Word::DOORDER,
            "DOWALK" => Word::DOWALK,
            "DRAWSCR" => Word::DRAWSCR,
            "DREQUEST" => Word::DREQUEST,
            "EMIT" => Word::EMIT,
            "ENDTUNE" => Word::ENDTUNE_16,
            "ERASESCR" => Word::ERASESCR,
            "ERRORLEVEL" => Word::ERRORLEVEL,
            "EXIST" => Word::EXIST,
            "FADEIN" => Word::FADEIN_16,
            "FADEOUT" => Word::FADEOUT_16,
            "FNTSTAT" => Word::FNTSTAT,
            "FNTSTAT+" => Word::FNTSTAT_PLUS,
            "FNTSTAT-" => Word::FNTSTAT_MINUS,
            "FREEZESCR" => Word::FREEZESCR,
            "FRESHSCREEN" => Word::FRESHSCREEN,
            "GDACTIVE" => Word::GDACTIVE,
            "GDBL" => Word::GDBL,
            "GDCOL" => Word::GDCOL,
            "GDCX" => Word::GDCX,
            "GDCY" => Word::GDCY,
            "GDHEIGHT" => Word::GDHEIGHT,
            "GDLEV" => Word::GDLEV,
            "GDLV" => Word::GDLV,
            "GDNR" => Word::GDNR,
            "GDOX" => Word::GDOX,
            "GDOY" => Word::GDOY,
            "GDSPR" => Word::GDSPR,
            "GDTB" => Word::GDTB,
            "GDTEXTLEN" => Word::GDTEXTLEN,
            "GDTXT" => Word::GDTXT,
            "GDTXTLEN" => Word::GDTXTLEN,
            "GDWIDTH" => Word::GDWIDTH,
            "GDX" => Word::GDX,
            "GDXLEN" => Word::GDXLEN,
            "GDY" => Word::GDY,
            "GDYLEN" => Word::GDYLEN,
            "GDZ" => Word::GDZ,
            "GET" => Word::GET,
            "GETANIM" => Word::GETANIM,
            "GFXCRUNCH" => Word::GFXCRUNCH,
            "GFXSTAT" => Word::GFXSTAT,
            "GFXSTAT+" => Word::GFXSTAT_PLUS,
            "GFXSTAT-" => Word::GFXSTAT_MINUS,
            "GFXTO" => Word::GFXTO,
            "GFXVFLIP" => Word::GFXVFLIP,
            "GSCRACT" => Word::GSCRACT,
            "GSCRPOS" => Word::GSCRPOS,
            "GSCRVSIZE" => Word::GSCRVSIZE,
            "GSCRX" => Word::GSCRX_16,
            "GSCRY" => Word::GSCRY_16,
            "HICOLOR" => Word::HICOLOR,
            "HIDEMOUSE" => Word::HIDEMOUSE,
            "KEY" => Word::KEY,
            "KILLDESC" => Word::KILLDESC,
            "KILLNBUF" => Word::KILLNBUF,
            "KILLNDESC" => Word::KILLNDESC,
            "MOUSEINFO" => Word::MOUSEINFO,
            "MOUSELK" => Word::MOUSELK,
            "MOUSERK" => Word::MOUSERK,
            "MOUSEX" => Word::MOUSEX,
            "MOUSEXY" => Word::MOUSEXY,
            "MOUSEY" => Word::MOUSEY,
            "NEWANIM" => Word::NEWANIM,
            "NEWDESC" => Word::NEWDESC,
            "NEWSCREEN" => Word::NEWSCREEN,
            "NEWSETDESC" => Word::NEWSETDESC,
            "NORMMOUSE" => Word::NORMMOUSE,
            "PALSTAT" => Word::PALSTAT,
            "PALSTAT+" => Word::PALSTAT_PLUS,
            "PALSTAT-" => Word::PALSTAT_MINUS,
            "PUT" => Word::PUT,
            "PUTANIM" => Word::PUTANIM,
            "QUITANIM" => Word::QUITANIM,
            "REMSCR" => Word::REMSCR,
            "REQUEST" => Word::REQUEST,
            "RESETANIM" => Word::RESETANIM,
            "RESETBUF" => Word::RESETBUF_16,
            "RESETFONT" => Word::RESETFONT,
            "RESETTI" => Word::RESETTI,
            "RGB->COL" => Word::RGB_TO_COL,
            "SCRACT" => Word::SCRACT,
            "SCRCTRL" => Word::SCRCTRL,
            "SCRFVSIZE" => Word::SCRFVSIZE,
            "SCRINACT" => Word::SCRINACT,
            "SCRPOS" => Word::SCRPOS_16,
            "SCRSIZE" => Word::SCRSIZE,
            "SCRSTAT" => Word::SCRSTAT,
            "SCRSTAT+" => Word::SCRSTAT_PLUS,
            "SCRSTAT-" => Word::SCRSTAT_MINUS,
            "SCRVPOS" => Word::SCRVPOS,
            "SCRVSIZE" => Word::SCRVSIZE,
            "SCRX" => Word::SCRX_16,
            "SCRY" => Word::SCRY,
            "SD%SHR" => Word::SD_PCT_SHR,
            "SDACTIVE" => Word::SDACTIVE,
            "SDALINES" => Word::SDALINES,
            "SDAUTOBUF" => Word::SDAUTOBUF,
            "SDBL" => Word::SDBL,
            "SDBLK" => Word::SDBLK,
            "SDBUF" => Word::SDBUF_16,
            "SDCEN" => Word::SDCEN,
            "SDCOL" => Word::SDCOL,
            "SDCX" => Word::SDCX,
            "SDCY" => Word::SDCY,
            "SDFNT" => Word::SDFNT,
            "SDH%SHR" => Word::SDH_PCT_SHR,
            "SDINACTIVE" => Word::SDINACTIVE,
            "SDINSERT" => Word::SDINSERT,
            "SDLEV" => Word::SDLEV,
            "SDLV" => Word::SDLV,
            "SDNORM" => Word::SDNORM,
            "SDOX" => Word::SDOX,
            "SDOY" => Word::SDOY,
            "SDPOS" => Word::SDPOS,
            "SDSHADE" => Word::SDSHADE,
            "SDSPR" => Word::SDSPR,
            "SDSTARTLINE" => Word::SDSTARTLINE,
            "SDTB" => Word::SDTB,
            "SDTDT" => Word::SDTDT,
            "SDTRANS" => Word::SDTRANS,
            "SDTXT" => Word::SDTXT,
            "SDV%SHR" => Word::SDV_PCT_SHR,
            "SDVCEN" => Word::SDVCEN,
            "SDWORD" => Word::SDWORD,
            "SDWAIT" => Word::SDWAIT,
            "SDX" => Word::SDX,
            "SDY" => Word::SDY,
            "SDZ" => Word::SDZ,
            "SETBUF" => Word::SETBUF_16,
            "SETCYCLE" => Word::SETCYCLE,
            "SETMOUSELB" => Word::SETMOUSELB,
            "SETMOUSERB" => Word::SETMOUSERB,
            "SETMOUSEX" => Word::SETMOUSEX,
            "SETMOUSEY" => Word::SETMOUSEY,
            "SETPAL" => Word::SETPAL,
            "SETRES" => Word::SETRES,
            "SETSHADE" => Word::SETSHADE,
            "SFT" => Word::SFT,
            "SHOWMOUSE" => Word::SHOWMOUSE,
            "SPEEDMODE" => Word::SPEEDMODE,
            "STARTTUNE" => Word::STARTTUNE_16,
            "STEPMULTI" => Word::STEPMULTI,
            "SUBFROMINV" => Word::SUBFROMINV,
            "SYSBC" => Word::SYSBC,
            "SYSFC" => Word::SYSFC,
            "TOGFX" => Word::TOGFX,
            "TXTSTAT" => Word::TXTSTAT,
            "TXTSTAT+" => Word::TXTSTAT_PLUS,
            "TXTSTAT-" => Word::TXTSTAT_MINUS,
            "UNFREEZESCR" => Word::UNFREEZESCR,
            "XATMOUSE" => Word::XATMOUSE,
            "XBLKSTAT" => Word::XBLKSTAT,
            "XBLKSTAT+" => Word::XBLKSTAT_PLUS,
            "XBLKSTAT-" => Word::XBLKSTAT_MINUS,
            "XFNTSTAT" => Word::XFNTSTAT,
            "XFNTSTAT+" => Word::XFNTSTAT_PLUS,
            "XFNTSTAT-" => Word::XFNTSTAT_MINUS,
            "XGFXCRUNCH" => Word::XGFXCRUNCH,
            "XGFXSTAT" => Word::XGFXSTAT,
            "XGFXSTAT+" => Word::XGFXSTAT_PLUS,
            "XGFXSTAT-" => Word::XGFXSTAT_MINUS,
            "XGFXVFLIP" => Word::XGFXVFLIP,
            "XPALSTAT" => Word::XPALSTAT,
            "XPALSTAT+" => Word::XPALSTAT_PLUS,
            "XPALSTAT-" => Word::XPALSTAT_MINUS,
            "XSCRSTAT" => Word::XSCRSTAT,
            "XSCRSTAT+" => Word::XSCRSTAT_PLUS,
            "XSCRSTAT-" => Word::XSCRSTAT_MINUS,
            "XTXTSTAT" => Word::XTXTSTAT,
            "XTXTSTAT+" => Word::XTXTSTAT_PLUS,
            "XTXTSTAT-" => Word::XTXTSTAT_MINUS,
            "_POOR" => Word::POOR,
            _ => return None,
        })
    }

    /// The kernel's own spelling, for a report or a stack-underflow message.
    ///
    /// Both variants of a split word answer the same name, because it *is* the
    /// same name — what differs is what the machine does with it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Word::PLUS_FONT => "+FONT",
            Word::TO_SCRX => "->SCRX",
            Word::TO_SCRY => "->SCRY",
            Word::MINUS_FONT => "-FONT",
            Word::DOT => ".",
            Word::MODE_320X200X256 => "320x200x256",
            Word::MODE_640X480X256 => "640x480x256",
            Word::MODE_640X480X32K => "640x480x32K",
            Word::RES_ERASE => "=>ERASE",
            Word::RES_ERASE_16 => "=>ERASE",
            Word::RES_EXIST => "=>EXIST",
            Word::RES_GET => "=>GET",
            Word::RES_GETAS => "=>GETAS",
            Word::RES_PUTAS => "=>PUTAS",
            Word::Q_ACTDESC => "?ACTDESC",
            Word::Q_INSIDE => "?INSIDE",
            Word::Q_INVINCL => "?INVINCL",
            Word::Q_KEY => "?KEY",
            Word::Q_SOUND => "?SOUND",
            Word::Q_XINSIDE => "?XINSIDE",
            Word::ACTDESC => "ACTDESC",
            Word::ACTSCR => "ACTSCR",
            Word::ADDMESSPIPE => "ADDMESSPIPE",
            Word::ADDTOINV => "ADDTOINV",
            Word::ANIMPLAY => "ANIMPLAY",
            Word::ATMOUSE => "ATMOUSE",
            Word::BLKSTAT => "BLKSTAT",
            Word::BLKSTAT_PLUS => "BLKSTAT+",
            Word::BLKSTAT_MINUS => "BLKSTAT-",
            Word::BUFON => "BUFON",
            Word::CCALCINV => "CCALCINV",
            Word::CROUTE => "CROUTE",
            Word::CTRL => "CTRL",
            Word::CUTPAL => "CUTPAL",
            Word::DEFTDT => "DEFTDT",
            Word::DELAY => "DELAY",
            Word::DOORDER => "DOORDER",
            Word::DOWALK => "DOWALK",
            Word::DRAWSCR => "DRAWSCR",
            Word::DREQUEST => "DREQUEST",
            Word::EMIT => "EMIT",
            Word::ENDTUNE => "ENDTUNE",
            Word::ENDTUNE_16 => "ENDTUNE",
            Word::ERASESCR => "ERASESCR",
            Word::ERRORLEVEL => "ERRORLEVEL",
            Word::EXIST => "EXIST",
            Word::FADEIN => "FADEIN",
            Word::FADEIN_16 => "FADEIN",
            Word::FADEOUT => "FADEOUT",
            Word::FADEOUT_16 => "FADEOUT",
            Word::FNTSTAT => "FNTSTAT",
            Word::FNTSTAT_PLUS => "FNTSTAT+",
            Word::FNTSTAT_MINUS => "FNTSTAT-",
            Word::FREEZESCR => "FREEZESCR",
            Word::FRESHSCREEN => "FRESHSCREEN",
            Word::GDACTIVE => "GDACTIVE",
            Word::GDBL => "GDBL",
            Word::GDCOL => "GDCOL",
            Word::GDCX => "GDCX",
            Word::GDCY => "GDCY",
            Word::GDHEIGHT => "GDHEIGHT",
            Word::GDLEV => "GDLEV",
            Word::GDLV => "GDLV",
            Word::GDNR => "GDNR",
            Word::GDOX => "GDOX",
            Word::GDOY => "GDOY",
            Word::GDSPR => "GDSPR",
            Word::GDTB => "GDTB",
            Word::GDTEXTLEN => "GDTEXTLEN",
            Word::GDTXT => "GDTXT",
            Word::GDTXTLEN => "GDTXTLEN",
            Word::GDWIDTH => "GDWIDTH",
            Word::GDX => "GDX",
            Word::GDXLEN => "GDXLEN",
            Word::GDY => "GDY",
            Word::GDYLEN => "GDYLEN",
            Word::GDZ => "GDZ",
            Word::GET => "GET",
            Word::GETANIM => "GETANIM",
            Word::GFXCRUNCH => "GFXCRUNCH",
            Word::GFXSTAT => "GFXSTAT",
            Word::GFXSTAT_PLUS => "GFXSTAT+",
            Word::GFXSTAT_MINUS => "GFXSTAT-",
            Word::GFXTO => "GFXTO",
            Word::GFXVFLIP => "GFXVFLIP",
            Word::GSCRACT => "GSCRACT",
            Word::GSCRPOS => "GSCRPOS",
            Word::GSCRVSIZE => "GSCRVSIZE",
            Word::GSCRX => "GSCRX",
            Word::GSCRX_16 => "GSCRX",
            Word::GSCRY => "GSCRY",
            Word::GSCRY_16 => "GSCRY",
            Word::HICOLOR => "HICOLOR",
            Word::HIDEMOUSE => "HIDEMOUSE",
            Word::KEY => "KEY",
            Word::KILLDESC => "KILLDESC",
            Word::KILLNBUF => "KILLNBUF",
            Word::KILLNDESC => "KILLNDESC",
            Word::MOUSEINFO => "MOUSEINFO",
            Word::MOUSELK => "MOUSELK",
            Word::MOUSERK => "MOUSERK",
            Word::MOUSEX => "MOUSEX",
            Word::MOUSEXY => "MOUSEXY",
            Word::MOUSEY => "MOUSEY",
            Word::NEWANIM => "NEWANIM",
            Word::NEWDESC => "NEWDESC",
            Word::NEWSCREEN => "NEWSCREEN",
            Word::NEWSETDESC => "NEWSETDESC",
            Word::NORMMOUSE => "NORMMOUSE",
            Word::PALSTAT => "PALSTAT",
            Word::PALSTAT_PLUS => "PALSTAT+",
            Word::PALSTAT_MINUS => "PALSTAT-",
            Word::PUT => "PUT",
            Word::PUTANIM => "PUTANIM",
            Word::QUITANIM => "QUITANIM",
            Word::REMSCR => "REMSCR",
            Word::REQUEST => "REQUEST",
            Word::RESETANIM => "RESETANIM",
            Word::RESETBUF => "RESETBUF",
            Word::RESETBUF_16 => "RESETBUF",
            Word::RESETFONT => "RESETFONT",
            Word::RESETTI => "RESETTI",
            Word::RGB_TO_COL => "RGB->COL",
            Word::SCRACT => "SCRACT",
            Word::SCRCTRL => "SCRCTRL",
            Word::SCRFVSIZE => "SCRFVSIZE",
            Word::SCRINACT => "SCRINACT",
            Word::SCRPOS => "SCRPOS",
            Word::SCRPOS_16 => "SCRPOS",
            Word::SCRSIZE => "SCRSIZE",
            Word::SCRSTAT => "SCRSTAT",
            Word::SCRSTAT_PLUS => "SCRSTAT+",
            Word::SCRSTAT_MINUS => "SCRSTAT-",
            Word::SCRVPOS => "SCRVPOS",
            Word::SCRVSIZE => "SCRVSIZE",
            Word::SCRX => "SCRX",
            Word::SCRX_16 => "SCRX",
            Word::SCRY => "SCRY",
            Word::SD_PCT_SHR => "SD%SHR",
            Word::SDACTIVE => "SDACTIVE",
            Word::SDALINES => "SDALINES",
            Word::SDAUTOBUF => "SDAUTOBUF",
            Word::SDBL => "SDBL",
            Word::SDBLK => "SDBLK",
            Word::SDBUF => "SDBUF",
            Word::SDBUF_16 => "SDBUF",
            Word::SDCEN => "SDCEN",
            Word::SDCOL => "SDCOL",
            Word::SDCX => "SDCX",
            Word::SDCY => "SDCY",
            Word::SDFNT => "SDFNT",
            Word::SDH_PCT_SHR => "SDH%SHR",
            Word::SDINACTIVE => "SDINACTIVE",
            Word::SDINSERT => "SDINSERT",
            Word::SDLEV => "SDLEV",
            Word::SDLV => "SDLV",
            Word::SDNORM => "SDNORM",
            Word::SDOX => "SDOX",
            Word::SDOY => "SDOY",
            Word::SDPOS => "SDPOS",
            Word::SDSHADE => "SDSHADE",
            Word::SDSPR => "SDSPR",
            Word::SDSTARTLINE => "SDSTARTLINE",
            Word::SDTB => "SDTB",
            Word::SDTDT => "SDTDT",
            Word::SDTRANS => "SDTRANS",
            Word::SDTXT => "SDTXT",
            Word::SDV_PCT_SHR => "SDV%SHR",
            Word::SDVCEN => "SDVCEN",
            Word::SDWORD => "SDWORD",
            Word::SDWAIT => "SDWAIT",
            Word::SDX => "SDX",
            Word::SDY => "SDY",
            Word::SDZ => "SDZ",
            Word::SETBUF => "SETBUF",
            Word::SETBUF_16 => "SETBUF",
            Word::SETCYCLE => "SETCYCLE",
            Word::SETMOUSELB => "SETMOUSELB",
            Word::SETMOUSERB => "SETMOUSERB",
            Word::SETMOUSEX => "SETMOUSEX",
            Word::SETMOUSEY => "SETMOUSEY",
            Word::SETPAL => "SETPAL",
            Word::SETRES => "SETRES",
            Word::SETSHADE => "SETSHADE",
            Word::SFT => "SFT",
            Word::SHOWMOUSE => "SHOWMOUSE",
            Word::SPEEDMODE => "SPEEDMODE",
            Word::STARTTUNE => "STARTTUNE",
            Word::STARTTUNE_16 => "STARTTUNE",
            Word::STEPMULTI => "STEPMULTI",
            Word::SUBFROMINV => "SUBFROMINV",
            Word::SYSBC => "SYSBC",
            Word::SYSFC => "SYSFC",
            Word::TOGFX => "TOGFX",
            Word::TXTSTAT => "TXTSTAT",
            Word::TXTSTAT_PLUS => "TXTSTAT+",
            Word::TXTSTAT_MINUS => "TXTSTAT-",
            Word::UNFREEZESCR => "UNFREEZESCR",
            Word::XATMOUSE => "XATMOUSE",
            Word::XBLKSTAT => "XBLKSTAT",
            Word::XBLKSTAT_PLUS => "XBLKSTAT+",
            Word::XBLKSTAT_MINUS => "XBLKSTAT-",
            Word::XFNTSTAT => "XFNTSTAT",
            Word::XFNTSTAT_PLUS => "XFNTSTAT+",
            Word::XFNTSTAT_MINUS => "XFNTSTAT-",
            Word::XGFXCRUNCH => "XGFXCRUNCH",
            Word::XGFXSTAT => "XGFXSTAT",
            Word::XGFXSTAT_PLUS => "XGFXSTAT+",
            Word::XGFXSTAT_MINUS => "XGFXSTAT-",
            Word::XGFXVFLIP => "XGFXVFLIP",
            Word::XPALSTAT => "XPALSTAT",
            Word::XPALSTAT_PLUS => "XPALSTAT+",
            Word::XPALSTAT_MINUS => "XPALSTAT-",
            Word::XSCRSTAT => "XSCRSTAT",
            Word::XSCRSTAT_PLUS => "XSCRSTAT+",
            Word::XSCRSTAT_MINUS => "XSCRSTAT-",
            Word::XTXTSTAT => "XTXTSTAT",
            Word::XTXTSTAT_PLUS => "XTXTSTAT+",
            Word::XTXTSTAT_MINUS => "XTXTSTAT-",
            Word::POOR => "_POOR",
        }
    }

    /// Whether the word genuinely has no effect in a silent, still-frame run.
    ///
    /// This list exists because the alternative bit twice. A stub that pops and
    /// pushes the right number of values but does nothing is indistinguishable from
    /// a working word — until something far downstream reads the state it should
    /// have written. `=>GET` left a stray value that became a jump address two
    /// hundred steps later, and `GET` looked harmless while being the resource
    /// loader the whole location system depends on.
    ///
    /// So a word may only be inert if there is a reason, stated here. Anything else
    /// is implemented or fails loudly with its name.
    ///
    /// * `RESETTI` — timers, and a still frame has no time passing.
    /// * The `…STAT`, `…STAT+` and `…STAT-` family, plain and `X`-prefixed — hints
    ///   about which resource ranges are wanted, may be dropped, or are pinned.
    ///   Nothing here evicts anything, so every one of them is inert. Their arities
    ///   are not uniform (`GFXSTAT+` takes three where its siblings take two, and
    ///   `XGFXSTAT+` four), which is why they are listed with counts rather than
    ///   handled by name pattern.
    /// * `RESETANIM`, `RESETFONT` — reset subsystems that start out reset.
    /// * `CUTPAL`, `SETBUF`, `RESETBUF` — palette and buffer bookkeeping for the
    ///   engine's double buffering, which a composed still frame does not use.
    /// * `SDNORM`, `SDPOS` — descriptor modes with no argument whose effect only
    ///   shows in animation.
    /// * `SDBLK` — sets bit 7 of the descriptor's byte +0x17 (its only writer, at
    ///   0x73099). That bit makes the output routine center the whole text block on
    ///   one width rather than each line on its own (0x25833). Nothing on the paths
    ///   walked so far calls it, so the behavior is unobserved; when something
    ///   does, this entry has to go and the two centering modes have to be built.
    /// * `ERRORLEVEL` — sets the exit code DOS would report. There is no DOS here,
    ///   and nothing in the game reads it back.
    /// * `DREQUEST` — hands three values to the diagnostic call at `0x5294d`
    ///   under category 0x1b, the same one `ANIMPLAY` uses to complain about being
    ///   entered twice. There is no debug channel here for it to reach.
    /// * `SPEEDMODE` — writes a flag at `0xd6608`. All 356 kernel words were
    ///   scanned for that address and `SPEEDMODE` is the only one that touches it,
    ///   so no word can read it back; whatever consumes it lives in the native
    ///   loop, which is replaced here by a frame clock of our own.
    /// * `GFXCRUNCH`, `XGFXCRUNCH` — read, not assumed: the routine behind them
    ///   (`0x59768`) walks a range of graphics and does nothing but `orb $8,7(%eax)`
    ///   or `andb $0xf7,7(%eax)` on a 22-byte record each. One bit, no pixels. The
    ///   sibling `GFXVFLIP` at `0x595be` does transform, which is why the two are
    ///   treated differently despite looking alike at the call site.
    /// * `FADEIN`, `FADEOUT` **in mode 2** — mode 1 is implemented as a curtain;
    ///   the handlers' second branch has not been read, and nothing reaches it.
    ///
    /// `SDINSERT` and `XATMOUSE` are deliberately absent: they do something.
    ///
    /// `=>GET` and `=>ERASE` are deliberately *not* here: they load a module and
    /// give one back, and they keep the descriptor-slot table that decides what
    /// a savegame contains.
    ///
    /// Was a list of names beside the code that used it, searched on every
    /// call. A match here is what makes [`crate::Engine::note_no_effect`]'s
    /// promise checkable: its argument is a `Word`, so a word this does not
    /// admit cannot be handed to it by mistake.
    pub(crate) fn inert(self) -> bool {
        matches!(
            self,
            Word::CUTPAL
                | Word::DREQUEST
                | Word::ERRORLEVEL
                | Word::GFXCRUNCH
                | Word::RESETANIM
                | Word::RESETBUF
                | Word::RESETFONT
                | Word::RESETTI
                | Word::SDBLK
                | Word::SDNORM
                | Word::SDPOS
                | Word::SETBUF
                | Word::SPEEDMODE
                | Word::XGFXCRUNCH
        )
    }

    /// How many arguments a resource status hint takes, or `None` for a word
    /// that is not one.
    ///
    /// One word family per row — the plain, the `-` and the `+` form of each. The
    /// layout is the table's index: reading down the first column lists the
    /// families, reading across a row lists the three forms of one. Left to itself
    /// rustfmt would set thirty-six entries one to a line and that is gone.
    ///
    /// They were a table searched by name on every call, which is also half of
    /// what made the group order load-bearing: the arm matched on membership
    /// rather than on a literal, so a group moving across it changed what it
    /// caught. Here the membership *is* the value.
    pub(crate) fn status_hint(self) -> Option<usize> {
        match self {
            Word::BLKSTAT => Some(2),
            Word::BLKSTAT_PLUS => Some(2),
            Word::BLKSTAT_MINUS => Some(2),
            Word::FNTSTAT => Some(2),
            Word::FNTSTAT_PLUS => Some(2),
            Word::FNTSTAT_MINUS => Some(2),
            Word::GFXSTAT => Some(2),
            Word::GFXSTAT_PLUS => Some(3),
            Word::GFXSTAT_MINUS => Some(2),
            Word::PALSTAT => Some(2),
            Word::PALSTAT_PLUS => Some(2),
            Word::PALSTAT_MINUS => Some(2),
            Word::SCRSTAT => Some(1),
            Word::SCRSTAT_PLUS => Some(2),
            Word::SCRSTAT_MINUS => Some(2),
            Word::TXTSTAT => Some(2),
            Word::TXTSTAT_PLUS => Some(2),
            Word::TXTSTAT_MINUS => Some(2),
            Word::XBLKSTAT => Some(3),
            Word::XBLKSTAT_PLUS => Some(3),
            Word::XBLKSTAT_MINUS => Some(3),
            Word::XFNTSTAT => Some(3),
            Word::XFNTSTAT_PLUS => Some(3),
            Word::XFNTSTAT_MINUS => Some(3),
            Word::XGFXSTAT => Some(3),
            Word::XGFXSTAT_PLUS => Some(4),
            Word::XGFXSTAT_MINUS => Some(3),
            Word::XPALSTAT => Some(3),
            Word::XPALSTAT_PLUS => Some(3),
            Word::XPALSTAT_MINUS => Some(3),
            Word::XSCRSTAT => Some(3),
            Word::XSCRSTAT_PLUS => Some(3),
            Word::XSCRSTAT_MINUS => Some(3),
            Word::XTXTSTAT => Some(3),
            Word::XTXTSTAT_PLUS => Some(3),
            Word::XTXTSTAT_MINUS => Some(3),
            _ => None,
        }
    }

    /// The descriptor field a one-argument setter writes, or `None`.
    ///
    /// The field map is keyed on these names, so the key comes from here and
    /// the arm and the key cannot drift apart. The other half of what made the
    /// group order load-bearing.
    pub(crate) fn descriptor_field(self) -> Option<Field> {
        match self {
            Word::SD_PCT_SHR => Some(Field::SD_PCT_SHR),
            Word::SDALINES => Some(Field::SDALINES),
            Word::SDBUF => Some(Field::SDBUF),
            Word::SDH_PCT_SHR => Some(Field::SDH_PCT_SHR),
            Word::SDSHADE => Some(Field::SDSHADE),
            Word::SDSTARTLINE => Some(Field::SDSTARTLINE),
            Word::SDTRANS => Some(Field::SDTRANS),
            Word::SDV_PCT_SHR => Some(Field::SDV_PCT_SHR),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Word;

    /// The two resolvers and [`Word::name`] agree.
    ///
    /// Three matches over two hundred values each have to say the same thing,
    /// and this is what says they do: every variant's name resolves back to
    /// that variant on at least one of the two machines. A variant left out of
    /// both resolvers, or given the wrong name, fails here.
    #[test]
    fn every_word_resolves_back_to_itself() {
        for &w in Word::ALL {
            let name = w.name();
            let via = [Word::of_m32(name), Word::of_m16(name)];
            assert!(
                via.contains(&Some(w)),
                "{name} resolves to {via:?}, neither of which is {w:?}"
            );
        }
    }

    /// A split word means one thing per machine, and the two halves agree
    /// about the name they answer to. Which is the whole point of there being
    /// two of them.
    #[test]
    fn a_split_word_means_one_thing_per_machine() {
        let split: Vec<Word> = Word::ALL.iter().copied().filter(|&w| is_16(w)).collect();
        assert_eq!(split.len(), 12, "the split set changed");
        for w in split {
            let name = w.name();
            assert_eq!(Word::of_m16(name), Some(w), "{name} on the 16-bit machine");
            let m32 = Word::of_m32(name).expect("a split word exists on both machines");
            assert_ne!(m32, w, "{name} resolves the same on both machines");
            assert_eq!(m32.name(), name, "the two halves answer to one name");
        }
    }

    /// Nothing is inert on one machine and a real word on the other without
    /// saying so: `SETBUF`, `RESETBUF` and `SDBUF` are inert or a field on the
    /// 32-bit machine and real buffer words on the 16-bit one, and it is their
    /// `_16` halves that are the real ones.
    #[test]
    fn the_buffer_words_are_inert_only_on_the_machine_that_ignores_them() {
        for name in ["SETBUF", "RESETBUF"] {
            let m32 = Word::of_m32(name).expect("a 32-bit word");
            let m16 = Word::of_m16(name).expect("a 16-bit word");
            assert!(m32.inert(), "{name} is inert on the 32-bit machine");
            assert!(!m16.inert(), "{name} does something on the 16-bit one");
        }
        assert_eq!(
            Word::of_m32("SDBUF").and_then(Word::descriptor_field),
            Some(crate::Field::SDBUF),
            "the 32-bit SDBUF is a descriptor field"
        );
        assert_eq!(
            Word::of_m16("SDBUF").and_then(Word::descriptor_field),
            None,
            "the 16-bit SDBUF is a buffer word"
        );
    }

    /// Whether a variant is the 16-bit half of a split word. By its own
    /// spelling, because that is the rule the file states.
    fn is_16(w: Word) -> bool {
        format!("{w:?}").ends_with("_16")
    }
}
