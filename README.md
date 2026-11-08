# The Lock-Keeper's Night

The loremaster demo story as a game of its own: a constable's notebook on a desk at night, written
in ink as you play, lit by a candle that follows the mouse. The story is compiled into the binary
at build time, together with the fonts and its [Treaty of Babel](https://babel.ifarchive.org/babel.html)
record.

```sh
task run PROFILE=release   # or: cargo run --release
target/release/lock-keeper --ifid       # the IFID; --ifiction prints the record
```

Type commands, or click the underlined words: a verb waits for its noun, a thing is examined, an exit walked. `hint`, `journal` and `undo` work.
Enter or a click finishes the pen's sentence; PageUp and the wheel read back.

The story comes from `../loremaster/examples/lock-keeper-slice/demo` for now (`build.rs`), with
the runtime from the same checkout. `tools/bake_fonts.py` made `assets/fonts` from Caveat and
Newsreader (SIL OFL, licences beside them).
