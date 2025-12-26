
```
OPENSSL_DIR=/usr OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu OPENSSL_INCLUDE_DIR=/usr/include/openssl CC=gcc CXX=g++ cargo build --release
```
# Shaipot - Shaicoin Miner

Welcome to **Shaipot**, a Shaicoin miner written in Rust. Shaipot is designed for efficiency and speed, supporting multi-threaded mining with minimal setup.

## Getting Started

To start mining with **Shaipot**, you need to provide the necessary arguments to connect to a mining pool and specify your Shaicoin address. Let's walk through how to set up and start mining.

### Required Arguments

- `--address <shaicoin_address>`  
  Your **Shaicoin address** where you want your mining rewards to be sent.
  
- `--pool <POOL_URL>`  
  The **pool URL** to which your miner will connect for jobs. This should be a valid WebSocket URL for the pool.

### Optional Arguments

- `--threads <AMT>`  
  Specifies the number of threads to use for mining. By default, the miner will automatically detect the optimal number of threads based on your system's available cores, but you can override this by specifying a value manually.

- `--vdftime1 <MILLISECONDS>`  
  Specifies the timeout in milliseconds for the Hamiltonian path search in the first graph (worker graph). Default is 1000ms. This controls how long the miner will search for a valid path in the primary mining graph before giving up.

- `--vdftime2 <MILLISECONDS>`  
  Specifies the timeout in milliseconds for the Hamiltonian path search in the second graph (queen bee graph). Default is 10ms. This controls the timeout for the secondary graph used in the mining algorithm. 

## Compilation

To compile **Shaipot** with optimal performance, use the provided build script:

```bash
./build.sh
```

This script will compile the project with the highest optimization settings for your CPU, ensuring maximum performance during mining.

After compilation, the resulting executable will be located in the `target/release` directory. You can run it from there using the following command:

```bash
./target/release/shaipot --address <shaicoin_address> --pool <POOL_URL> [--threads <AMT>] [--vdftime1 <MILLISECONDS>] [--vdftime2 <MILLISECONDS>]
```

Make sure to replace `<shaicoin_address>` and `<POOL_URL>` with your actual Shaicoin address and the pool URL you're using.

## Running the Program

Once compiled, **Shaipot** is ready to run! Simply use the command provided above, specifying your Shaicoin address, the pool URL, and (optionally) the number of threads. Here's an example:

```bash
./target/release/shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --threads 4
```

Example usage with custom vdftime parameters:
```bash
./target/release/shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --threads 4 --vdftime1 1500 --vdftime2 15
```

You can also specify just one of the vdftime parameters:
```bash
./target/release/shaipot --address sh1qeexkz69dz6j4q0zt0pkn36650yevwc8eksqeuu --pool wss://pool.shaicoin.org --vdftime2 15
```

This will start the mining process, and you'll see output as **Shaipot** connects to the pool and begins mining.

```plaintext
                          __
                         // \
                         \\_/ // 
    brrr''-.._.-''-.._.. -(||)(')
                         '''  
        _
     __( )_
    (      (o____
     |          |
     |      (__/
       \     /   ___
       /     \  \___/
     /    ^    /     \
    |   |  |__|_ SHA  |
    |    \______)____/
     \         /
       \     /_
        |  ( __)
        (____)
```

Happy Mining!

# Update Log

**2025-11-24**

In the process of graph search, we first calculate the edges and then conduct the search, which can improve the speed of path search to a certain extent.
In addition, vdftime1 and vdftime2 can be specified by yourself based on the performance of the device