# Yuga: Automatically Detecting Lifetime Annotation Bugs in the Rust Language

**[NEW]** Try out our (WIP) [web demo](https://yuga-rust-1ec69edea65b.herokuapp.com) of Yuga!

**[INFO]** We have put together a [synthetically created database](https://github.com/vnrst/rust-lifetime-bugs) of 27 lifetime annotation bugs, based on patterns obtained from RustSec vulnerability reports.

Yuga is a tool to detect lifetime annotation bugs in Rust [[ArXiv](https://arxiv.org/pdf/2310.08507.pdf)]. It is adapted from a fork of [Rudra](https://github.com/sslab-gatech/Rudra).

To setup the code, clone the repository, `cd` into it, and run the following command (tested on Mac and Ubuntu):
```
./install-debug.sh
```
If you face errors, please refer to the instructions in the main Rudra repository for installing Rudra in debug mode.

Our tool can now be run using the `cargo-yuga` subcommand. For any Rust package that we want to analyze, run the following command from within the package folder:
```
cargo yuga
```
This will print the reported vulnerabilities, if any, to `stdout`.

Here is a list of bugs in public Rust projects detected by Yuga so far:

|       Project       |                           Issue/PR                            | Public/Private API |          Status              |
|---------------------|---------------------------------------------------------------|--------------------|------------------------------|
| alsa                | https://github.com/diwic/alsa-rs/issues/117                   |       Public       |   Unconfirmed                |           
| bv                  | https://github.com/tov/bv-rs/issues/16                        |       Public       |   Confirmed with Miri        |
| pulse-binding-rust  | https://github.com/jnqnfe/pulse-binding-rust/issues/53        |       Public       |   Confirmed with Valgrind    |
| cslice              | https://github.com/dherman/cslice/issues/5                    |       Public       |   Confirmed with Miri        |
| json-rust / jzon-rs | https://github.com/maciejhirsz/json-rust/pull/209             |       Private      |   Confirmed by dev           |
| sled                | https://github.com/spacejam/sled/issues/1442                  |       Private      |   Confirmed by dev           |
| tokio               | https://github.com/tokio-rs/tokio/issues/5113                 |       Private      |   Unconfirmed                |
