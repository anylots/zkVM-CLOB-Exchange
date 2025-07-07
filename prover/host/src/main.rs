use share::{load_blocks, State};

mod gen_stark;
fn main() {
    let state = State::load();
    let blocks = load_blocks(101,10).unwrap();

    gen_stark::prove(state, blocks);
    println!("Hello, world!");
}
