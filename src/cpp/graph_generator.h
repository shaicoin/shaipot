#pragma once
#include <cstdint>
#include <vector>
#include <random>
#include <memory>
#include "rust/cxx.h"

struct GraphGenerator {
    std::mt19937_64 prng;
    std::uniform_int_distribution<uint64_t> distribution;
    
    GraphGenerator(uint64_t seed, uint64_t range);
    uint64_t next_random();
};

std::unique_ptr<GraphGenerator> create_graph_generator(uint64_t seed, uint64_t range);
