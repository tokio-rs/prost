# Kani
This document describes how to **locally** install and use Kani. Because of instability in
Kani internals, the GitHub action is the recommended option if you are
running in CI.

Kani is a software verification tool that complements testing by
proving the absence of certain classes of bugs like unwrap exceptions,
overflows, and assertion failures. See the [Kani
book](https://model-checking.github.io/kani/) for a full list of
capabilities and limitations.

## Installing Kani
-  The install instructions for Kani can be [found
   here](https://model-checking.github.io/kani/install-guide.html). Once
   Kani is installed, you can run with `cargo kani` for projects or
   `kani` for individual Rust files.

## Running Kani
After installing Kani, `cargo kani` should
automatically run `kani::proof` harnesses inside your crate. Use
`--harness` to run a specific harness, and `-p` for a specific
sub-crate.

If Kani returns with an error, you can use the concrete playback
feature using `--enable-unstable --concrete-playback print` and paste
in the code to your repository. Running this harness with `cargo test`
will replay the input found by Kani that produced this crash. Please
note that this feature is unstable.
