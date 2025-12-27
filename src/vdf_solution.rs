// use primitive_types::U256;
use crate::graph_bridge::ffi;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use sha2::{Digest, Sha256};
use crate::models::{SubmitMessage, Job};
use crate::utils::meets_target;
use hex;
use serde_json;

pub const GRAPH_SIZE: u16 = 2008;

pub struct HCGraphUtil {
    // start_time: Instant,
    // vdf_bailout: u64
}

impl HCGraphUtil {
    // Helper function to reverse a subpath (2-opt optimization)
    // fn reverse_subpath(path: &mut Vec<u16>, i: usize, j: usize) {
    //     path[i..=j].reverse();
    // }
    pub fn check_and_submit_solution(
        &self,
        queen_path: &Vec<u16>,
        worker_path: &Vec<u16>,
        data: &str,
        job: &Job,
        miner_id: &str,
        nonce: &str,
        server_sender: &mpsc::Sender<String>,
        hash_count: &AtomicUsize,
        api_hash_count: &AtomicUsize
    ) -> Option<bool> {
        // 组合worker_path和queen_path
        let mut combined_path = worker_path.clone();
        combined_path.extend(queen_path.clone());

        // 确保combined_path大小匹配GRAPH_SIZE
        if combined_path.len() < GRAPH_SIZE.into() {
            combined_path.resize(GRAPH_SIZE.into(), u16::MAX);
        }

        // 格式化为little-endian hex字符串
        let vdf_solution_hex_solved: String = combined_path
            .iter()
            .map(|&val| {
                let little_endian_val = val.to_le_bytes();
                format!("{:02x}{:02x}", little_endian_val[0], little_endian_val[1])
            })
            .collect();
        
        let data_with_vdf_solved = format!("{}{}", data, vdf_solution_hex_solved);

        let data_bytes_solved = hex::decode(data_with_vdf_solved).expect("Invalid hex input");

        // 最终SHA256 hash
        let mut hasher2 = Sha256::new();
        hasher2.update(&data_bytes_solved);
        let hash2 = hasher2.finalize();

        let final_hash_reversed = hex::encode(hash2.iter().rev().cloned().collect::<Vec<u8>>());

        // 检查是否满足难度要求
        if meets_target(&final_hash_reversed, &job.target) {
            println!("SUBMITTING SHARE TO BACKEND!");
            println!("final_hash_reversed: {}", final_hash_reversed);
            
            // 创建提交消息
            let submit_msg = SubmitMessage {
                r#type: String::from("submit"),
                miner_id: miner_id.to_string(),
                nonce: nonce.to_string(),
                job_id: job.job_id.clone(),
                path: vdf_solution_hex_solved,
            };

            // 发送提交消息
            if let Ok(msg) = serde_json::to_string(&submit_msg) {
                let _ = server_sender.send(msg);
            }

            // 返回true表示找到有效解并已提交
            Some(true)
        } else {
            // 增加hash计数
            hash_count.fetch_add(1, Ordering::Relaxed);
            api_hash_count.fetch_add(1, Ordering::Relaxed);
            // 返回false表示找到解但不满足难度要求
            Some(false)
        }
    }

    pub fn new(_vdf_bailout: Option<u64>) -> Self {
        // let bailout_timer: u64 = match vdf_bailout {
        //     Some(timer) => { timer },
        //     None => { 1000 } // default to 1 second
        // };
        HCGraphUtil {
            // start_time: Instant::now(),
            // vdf_bailout: bailout_timer
        }
    }

    fn hex_to_u64(&self, hex_string: &str) -> u64 {
        u64::from_str_radix(hex_string, 16).expect("Failed to convert hex to u64")
    }

    // fn extract_seed_from_hash(&self, hash: &U256) -> u64 {
    //     hash.low_u64()
    // }

    fn extract_seed_from_hash_hex(&self, hash_hex: &str) -> u64 {
        let mut bytes = hex::decode(hash_hex).expect("invalid hex");
        bytes.reverse(); // Match C++ implementation that reverses hash bytes
        let arr: [u8; 8] = bytes[0..8].try_into().expect("slice len");
        u64::from_le_bytes(arr)
    }
    
    pub fn get_worker_grid_size(&self, hash_hex: &str) -> u16 {
        let grid_size_segment = &hash_hex[0..8];
        let grid_size: u64 = self.hex_to_u64(grid_size_segment);
        let min_grid_size = 1892u64;
        let max_grid_size = 1920u64;
        let grid_size_final = min_grid_size + (grid_size % (max_grid_size - min_grid_size));
        grid_size_final as u16
    }

    pub fn get_queen_bee_grid_size(&self, worker_size: u16) -> u16 {
        GRAPH_SIZE - worker_size
    }


    fn generate_graph_v3_from_seed(&self, seed: u64, grid_size: u16, percentage_x10: u16) -> Vec<Vec<bool>> {
        let grid_size_usize = grid_size as usize;
        let mut graph = vec![vec![false; grid_size_usize]; grid_size_usize];

        let range: u64 = 1000;
        let threshold: u64 = (percentage_x10 as u64 * range) / 1000;

        // Use C++ std::uniform_int_distribution through FFI bridge
        let mut generator = ffi::create_graph_generator(seed, range);
        
        for i in 0..grid_size_usize {
            for j in (i + 1)..grid_size_usize {
                let random_value = generator.pin_mut().next_random();
                let edge_exists = random_value < threshold;
                
                graph[i][j] = edge_exists;
                graph[j][i] = edge_exists;
            }
        }

        graph
    }

    pub fn find_hamiltonian_cycle_v3_hex(&self, graph_hash_hex: &str, graph_size: u16, percentage_x10: u16, timeout_ms: u64) -> Vec<u16> {
        let mut path: Vec<u16> = Vec::with_capacity(graph_size as usize);
        let mut visited = vec![false; graph_size as usize];
        let seed = self.extract_seed_from_hash_hex(graph_hash_hex);
        
        let edges = self.generate_graph_v3_from_seed(seed, graph_size, percentage_x10);

        let start_node: u16 = 0;
        let start_time = Instant::now();

        fn dfs(
            current: u16,
            visited: &mut [bool],
            path: &mut Vec<u16>,
            edges: &Vec<Vec<bool>>,
            start_time: Instant,
            timeout_ms: u64,
            graph_size: u16,
        ) -> bool {
            if start_time.elapsed() > Duration::from_millis(timeout_ms) {
                return false;
            }

            path.push(current);
            visited[current as usize] = true;

            if path.len() == graph_size as usize && edges[current as usize][0] {
                return true;
            }

            for next in 0..graph_size as usize {
                if edges[current as usize][next] && !visited[next] {
                    if dfs(next as u16, visited, path, edges, start_time, timeout_ms, graph_size) {
                        return true;
                    }
                }
            }

            visited[current as usize] = false;
            path.pop();
            false
        }

        if dfs(start_node, &mut visited, &mut path, &edges, start_time, timeout_ms, graph_size) {
            return path;
        }

        Vec::new()
    }

    fn optimize_path(&self, path: &mut Vec<u16>, edges: &Vec<Vec<bool>>) {
        let n = path.len();
        let mut need_check = true;
        while need_check {
            need_check = false;
            for i in 1..(n - 1) {
                for j in (i + 1)..(n - 1) {
                    if edges[path[i - 1] as usize][path[j] as usize]
                        && edges[path[i] as usize][path[j + 1] as usize]
                        && path[i] > path[j]
                    {
                        // 2-opt swap to correct inversion
                        path[i..=j].reverse();
                        need_check = true;
                    }
                }
            }
        }
    }

    pub fn find_hamiltonian_cycle_v3_hex_second(
        &self, 
        graph_hash_hex: &str, 
        graph_size: u16, 
        percentage_x10: u16, 
        timeout_ms: u64,
        worker_path: &Vec<u16>,
        data: &str,
        job: &Job,
        miner_id: &str,
        nonce: &str,
        server_sender: &mpsc::Sender<String>,
        hash_count: &AtomicUsize,
        api_hash_count: &AtomicUsize
    ) -> Option<bool> {
        let mut path: Vec<u16> = Vec::with_capacity(graph_size as usize);
        let mut visited = vec![false; graph_size as usize];
        let seed = self.extract_seed_from_hash_hex(graph_hash_hex);
        
        let edges = self.generate_graph_v3_from_seed(seed, graph_size, percentage_x10);
        
        // 预计算nodeEdges数组：存储每个节点的邻居节点列表，避免重复查询edges矩阵
        // 同时这个可以直接让两重循环的100*100变成100*12.5 加速8倍
        let mut node_edges: Vec<Vec<u16>> = vec![Vec::new(); graph_size as usize];
        for i in 0..graph_size as usize {
            for j in 0..graph_size as usize {
                if edges[i][j] {
                    node_edges[i].push(j as u16);
                }
            }
        }

        let start_node: u16 = 0;
        let start_time = Instant::now();

        // fn dfs(
        //     current: u16,
        //     visited: &mut [bool],
        //     path: &mut Vec<u16>,
        //     node_edges: &Vec<Vec<u16>>,
        //     start_time: Instant,
        //     timeout_ms: u64,
        //     graph_size: u16,
        // ) -> bool {
        //     if start_time.elapsed() > Duration::from_millis(timeout_ms) {
        //         return false;
        //     }

        //     path.push(current);
        //     visited[current as usize] = true;

        //     if path.len() == graph_size as usize {
        //         // 检查是否回到起点
        //         if node_edges[current as usize].contains(&0) {
        //             return true;
        //         }
        //     } else {
        //         // 只遍历当前节点的邻居，避免无意义的判断
        //         for &next in &node_edges[current as usize] {
        //             if !visited[next as usize] {
        //                 if dfs(next, visited, path, node_edges, start_time, timeout_ms, graph_size) {
        //                     return true;
        //                 }
        //             }
        //         }
        //     }

        //     visited[current as usize] = false;
        //     path.pop();
        //     false
        // }

        /// Optimized DFS with pruning heuristics for Hamiltonian cycle finding
        fn dfs(
            current: u16,
            visited: &mut [bool],
            path: &mut Vec<u16>,
            node_edges: &Vec<Vec<u16>>,
            start_time: Instant,
            timeout_ms: u64,
            graph_size: u16,
        ) -> bool {
            if start_time.elapsed() > Duration::from_millis(timeout_ms) {
                return false;
            }
        
            path.push(current);
            visited[current as usize] = true;
        
            if path.len() == graph_size as usize {
                // Check if we can return to start
                if node_edges[current as usize].contains(&0) {
                    return true;
                }
            } else {
                // Early pruning: check if all unvisited nodes are still reachable
                if !is_feasible(current, visited, node_edges) {
                    visited[current as usize] = false;
                    path.pop();
                    return false;
                }
            
                // Collect unvisited neighbors and sort by degree (fewest connections first)
                let mut neighbors: Vec<(u16, usize)> = node_edges[current as usize]
                    .iter()
                    .filter(|&&n| !visited[n as usize])
                    .map(|&n| {
                        let degree = count_unvisited_neighbors(n, visited, node_edges);
                        (n, degree)
                    })
                    .collect();
                
                neighbors.sort_by_key(|&(_, degree)| degree);
                
                for (next, _) in neighbors {
                    if dfs(next, visited, path, node_edges, start_time, timeout_ms, graph_size) {
                        return true;
                    }
                }
            }
        
            visited[current as usize] = false;
            path.pop();
            false
        }

        /// Check if the remaining unvisited nodes can still form a valid path
        #[inline]
        fn is_feasible(current: u16, visited: &[bool], node_edges: &Vec<Vec<u16>>) -> bool {
            for (i, &is_visited) in visited.iter().enumerate() {
                if !is_visited && i != current as usize {
                    // Check if this unvisited node has at least one unvisited neighbor
                    let has_unvisited_neighbor = node_edges[i]
                        .iter()
                        .any(|&n| !visited[n as usize] || n == current);

                    if !has_unvisited_neighbor {
                        return false;
                    }
                }
            }
            true
        }

        /// Count how many unvisited neighbors a node has
        #[inline]
        fn count_unvisited_neighbors(node: u16, visited: &[bool], node_edges: &Vec<Vec<u16>>) -> usize {
            node_edges[node as usize]
                .iter()
                .filter(|&&n| !visited[n as usize])
                .count()
        }
        // 尝试找到queen路径
        if !dfs(start_node, &mut visited, &mut path, &node_edges, start_time, timeout_ms, graph_size) {
            return None; // 没有找到queen路径
        }

        // 对初始路径执行一次2-opt优化
        self.optimize_path(&mut path, &edges);
        if let Some(result) = self.check_and_submit_solution(&path, worker_path, data, job, miner_id, nonce, server_sender, hash_count, api_hash_count) {
            if result { return Some(true); } // 找到有效解，立即返回
        }

        // 继续尝试所有2-opt单点翻转（固定j为最后一个节点）
        let n = path.len();
        for i in 1..(n - 1) {
            let j = n - 1; // j固定为最后一个节点的索引
            
            // 检查翻转条件：
            // 1. path[i-1] 到 path[j] 的边存在
            // 2. path[i] 到 path[0] 的边存在
            let node_before_i = path[i - 1];
            let node_i = path[i];
            let node_j = path[j];
            let node_0 = path[0];
            
            if edges[node_before_i as usize][node_j as usize] && 
               edges[node_i as usize][node_0 as usize] {
                // 执行路径翻转：翻转 i 到 j 之间的子路径
                let mut new_path = path.clone();
                new_path[i..=j].reverse();
                
                // 检查新路径是否更优或提交解
                // 对新路径调用2-opt优化
                self.optimize_path(&mut new_path, &edges);

                // 检查新路径是否更优或提交解
                if let Some(result) = self.check_and_submit_solution(
                    &new_path, 
                    worker_path, 
                    data, 
                    job, 
                    miner_id, 
                    nonce, 
                    server_sender, 
                    hash_count, 
                    api_hash_count
                ) {
                    if result { 
                        return Some(true); // 找到有效解，立即返回
                    }
                }
            }
        }
                                   
        Some(false)
    }


}
