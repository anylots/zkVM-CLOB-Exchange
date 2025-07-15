use anyhow::anyhow;
use common::{block::Block, state::State};
use share::ZkVMInput;
use sp1_sdk::{HashableKey, ProverClient, SP1Stdin};
use std::time::Instant;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const BATCH_VERIFIER_ELF: &[u8] =
    include_bytes!("../../program/elf/riscv32im-succinct-zkvm-elf");
const MAX_PROVE_BLOCKS: usize = 4096;

pub fn prove(state: State, blocks: Vec<Block>) -> Result<Option<Vec<u8>>, anyhow::Error> {
    if blocks.len() > MAX_PROVE_BLOCKS {
        return Err(anyhow!(format!(
            "check block_tracs, blocks len = {:?} exceeds MAX_PROVE_BLOCKS = {:?}",
            blocks.len(),
            MAX_PROVE_BLOCKS
        )));
    }

    let input = ZkVMInput { blocks, state };

    // Execute the program in sp1-vm
    let mut stdin = SP1Stdin::new();
    stdin.write(&serde_json::to_string(&input).unwrap());
    let client = ProverClient::from_env();

    let (mut _public_values, execution_report) = client
        .execute(BATCH_VERIFIER_ELF, &stdin.clone())
        .run()
        .map_err(|e| anyhow!(format!("sp1-vm execution err: {:?}", e)))?;

    log::info!(
        "Program executed successfully, Number of cycles: {:?}",
        execution_report.total_instruction_count()
    );

    let (pk, vk) = client.setup(BATCH_VERIFIER_ELF);
    log::info!("Batch ELF Verification Key: {:?}", vk.vk.bytes32());

    // Generate the proof
    let start = Instant::now();
    let mut proof = client
        .prove(&pk, &stdin)
        .plonk()
        .run()
        .map_err(|e| anyhow!(format!("proving failed: {:?}", e)))?;

    let duration_mins = start.elapsed().as_secs() / 60;
    log::info!(
        "Successfully generated proof!, time use: {:?} minutes",
        duration_mins
    );

    // Verify the proof.
    client
        .verify(&proof, &vk)
        .map_err(|e| anyhow!(format!("failed to verify proof: {:?}", e)))?;
    log::info!("Successfully verified proof!");

    // Deserialize the public values.
    let pi_bytes = proof.public_values.read::<[u8; 32]>();
    log::info!("pi_hash generated with sp1-vm prove: {}", pi_bytes.len());

    Ok(Some(vec![]))
}
