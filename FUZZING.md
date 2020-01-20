# Fuzzing

Prost ships a few fuzz tests, using both libfuzzer and aflfuzz.


## afl

To run the afl fuzz tests, first install cargo-afl:

    cargo install -f afl

Then build a fuzz target and run afl on it:

    cd afl/<target>/
    cargo afl build
    cargo afl fuzz -i in -o out target/debug/<target>


## libfuzzer

TODO
