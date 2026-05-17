use vergen_git2::{Emitter, Git2Builder};

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/packed-refs");
    println!("cargo:rerun-if-changed=.git/refs/tags");

    Emitter::default()
        .add_instructions(
            &Git2Builder::default()
                .sha(true)
                .describe(true, false, None)
                .build()
                .expect("should get git status"),
        )
        .expect("should add instructions")
        .emit()
        .expect("should emit version info");
}
