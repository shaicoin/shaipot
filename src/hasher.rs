use hex;
use primitive_types::U256;
use sha2::{Digest, Sha256};
use super::vdf_solution::{HCGraphUtil, GRAPH_SIZE};

pub fn compute_hash_no_vdf(data: &str, hc_util: &mut HCGraphUtil) -> Option<(String, String)> {
    // Create the vdfSolution array with all values set to 0xFFFF (uint16_t max value)
    let vdf_solution: Vec<u16> = vec![0xFFFF; GRAPH_SIZE.into()];

    // Convert vdfSolution to a hex string
    let vdf_solution_hex: String = vdf_solution
        .iter()
        .map(|&val| format!("{:04x}", val))
        .collect();

    // Append vdfSolution hex to the input data
    let data_with_vdf = format!("{}{}", data, vdf_solution_hex);

    // Convert the hex string to bytes
    let data_bytes = hex::decode(data_with_vdf).expect("Invalid hex input");

    // First SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&data_bytes);
    let hash1 = hasher.finalize();
    
    let hash1_reversed = hex::encode(hash1.iter().rev().cloned().collect::<Vec<u8>>());
    let graph_hash_u256 = U256::from_str_radix(&hash1_reversed, 16).unwrap();
    let mut path = hc_util.find_hamiltonian_cycle_v2(graph_hash_u256);

    if path.is_empty() {
        return None;
    }

    if path.len() < GRAPH_SIZE.into() {
        path.resize(GRAPH_SIZE.into(), u16::MAX);
    }

    // Format path as little-endian u16
    let vdf_solution_hex_solved: String = path
        .iter()
        .map(|&val| {
            let little_endian_val = val.to_le_bytes();
            format!("{:02x}{:02x}", little_endian_val[0], little_endian_val[1])
        })
        .collect();
    
    let data_with_vdf_solved = format!("{}{}", data, vdf_solution_hex_solved);

    let data_bytes_solved = hex::decode(data_with_vdf_solved).expect("Invalid hex input");

    // Second SHA256 hash
    let mut hasher2 = Sha256::new();
    hasher2.update(&data_bytes_solved);
    let hash2 = hasher2.finalize();

    let final_hash_reversed = hex::encode(hash2.iter().rev().cloned().collect::<Vec<u8>>());

    Some((final_hash_reversed, vdf_solution_hex_solved))
}
