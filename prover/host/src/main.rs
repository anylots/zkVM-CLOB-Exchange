use common::state::StateDB;
use share::load_blocks;

mod gen_stark;
fn main() {
    let mut state_db = StateDB::new("state_db");
    state_db.load();
    let state = state_db.state;
    let blocks = load_blocks(101, 10).unwrap();

    let _ = gen_stark::prove(state, blocks);
    println!("Hello, world!");
}
