# HFT Order Book

A high-performance order book implementation for high-frequency trading (HFT) systems written in Rust. This implementation utilizes a Red-Black Tree for price levels and the LMAX Disruptor pattern with ring buffers for order queues.

## Table of Contents

- [Architecture](#architecture)
- [Performance Characteristics](#performance-characteristics)
- [Data Structures](#data-structures)
- [Building and Running](#building-and-running)
- [Simulation](#simulation)
- [Benchmarking](#benchmarking)
- [Testing](#testing)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

## Architecture

The HFT Order Book is designed for ultra-low latency and high throughput, suitable for high-frequency trading applications. The architecture follows a modular approach:

### Core Components

1. **Order Book**: The central component that manages the state of all orders. It uses Red-Black Trees to maintain sorted price levels for both bid and ask sides.

2. **Price Levels**: Each price level contains a queue of orders at that price, implemented using a ring buffer for optimal performance.

3. **Matching Engine**: Processes incoming orders and matches them against existing orders in the book according to price-time priority.

4. **Simulation**: Provides tools for performance testing and benchmarking the order book under various load conditions.

### Design Principles

- **Zero Allocation**: Critical paths avoid dynamic memory allocation to minimize GC pressure.
- **Mechanical Sympathy**: Data structures are designed to be cache-friendly and minimize CPU cache misses.
- **Lock-Free**: Uses lock-free data structures where possible to minimize contention.
- **Event-Sourced**: All state changes are driven by events, making the system easier to reason about and test.

## Performance Characteristics

The implementation is optimized for:

- **Ultra-Low Latency**: Order processing in microseconds or even nanoseconds.
- **High Throughput**: Capable of handling hundreds of thousands or millions of orders per second.
- **Predictable Performance**: Minimal variance in processing times, crucial for HFT systems.

## Data Structures

### Order Book Structure

```
OrderBook
├─ BidSide (Red-Black Tree of price levels, sorted descending)
│  ├─ PriceLevel 100.05
│  │  └─ OrderQueue (Ring Buffer)
│  │     ├─ Order 1
│  │     ├─ Order 2
│  │     └─ Order 3
│  ├─ PriceLevel 100.03
│  └─ PriceLevel 100.01
│
├─ AskSide (Red-Black Tree of price levels, sorted ascending)
│  ├─ PriceLevel 100.10
│  │  └─ OrderQueue (Ring Buffer)
│  │     ├─ Order 1
│  │     └─ Order 2
│  ├─ PriceLevel 100.15
│  └─ PriceLevel 100.20
│
└─ OrderMap (HashMap for O(1) lookup by OrderID)
   ├─ OrderID 1 → (BidSide, PriceLevel 100.05, Position 0)
   ├─ OrderID 2 → (BidSide, PriceLevel 100.05, Position 1)
   └─ OrderID 3 → (AskSide, PriceLevel 100.10, Position 0)
```

### LMAX Disruptor Pattern

The order queues use the LMAX Disruptor pattern, a high-performance inter-thread messaging library designed for low-latency systems. Key aspects:

- **Ring Buffer**: Pre-allocated, fixed-size buffer to avoid dynamic allocations
- **Single Writer Principle**: Each buffer has a single writer to avoid contention
- **Memory Barriers**: Careful use of memory barriers to ensure visibility
- **Padding**: Strategic padding to avoid false sharing

## Building and Running

### Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)

### Building

To build the project, run:

```bash
./make.sh
```

This will create a new directory called `hft-orderbook` with the complete project structure and all required dependencies.

### Running

To run the simulation, use:

```bash
./simulator.sh --load medium --duration 60
```

See the [Simulation](#simulation) section for more details on available options.

## Simulation

The simulator allows you to test the order book under different load conditions:

```bash
./simulator.sh [options]
```

### Options

- `-l, --load LEVEL`: Load level (low, medium, high, max)
- `-d, --duration SECS`: Duration in seconds
- `-o, --output FILE`: Output file for results
- `-b, --benchmark`: Run benchmark instead of simulation
- `-r, --report`: Generate HTML report after run
- `-h, --help`: Show help message

### Load Levels

- **Low**: ~1,000 orders per second
- **Medium**: ~10,000 orders per second
- **High**: ~100,000 orders per second
- **Max**: As fast as possible (limited by system performance)

### Example

```bash
./simulator.sh --load high --duration 120 --output results.json --report
```

This will run a high-load simulation for 120 seconds, save the results to `results.json`, and generate an HTML report.

## Benchmarking

The project includes benchmarks to measure performance:

```bash
./simulator.sh --benchmark
```

This uses Criterion.rs to run comprehensive benchmarks and generate reports.

## Testing

The project includes unit tests and integration tests. To run the tests:

```bash
cd hft-orderbook
cargo test
```

## Configuration

The order book and matching engine can be configured with the following parameters:

### OrderBookConfig

- `tick_size`: Minimum price increment
- `lot_size`: Minimum quantity increment
- `min_order_size`: Minimum order size
- `max_order_size`: Maximum order size
- `max_levels`: Maximum number of price levels
- `max_orders_per_level`: Maximum number of orders per price level
- `max_total_orders`: Maximum total orders in the book
- `buffer_size`: Size of the ring buffer for each price level

### MatchingEngineConfig

- `max_match_orders`: Maximum number of orders to match in a single pass
- `max_trades`: Maximum number of trades to generate in a single pass
- `enable_self_matching`: Enable self-matching (matching orders from the same client)

## Implementation Details

### Order Book Performance Optimizations

1. **Fixed-Point Representation**: Prices are stored as fixed-point numbers (multiplied by a constant factor) to avoid floating-point errors and improve performance.

2. **Cache-Friendly Data Structures**: Data is organized to maximize CPU cache efficiency:
   - Top-of-book data kept in L1 cache
   - Most active price levels prebuffered in L2 cache
   - Full order book structure in L3 cache/main memory

3. **Memory Pool Allocation**: Custom memory allocators reuse memory to reduce fragmentation and allocation overhead.

4. **Lock-Free Algorithms**: Critical paths use atomic operations and the LMAX Disruptor pattern to minimize contention.

5. **SIMD Instructions**: Where applicable, SIMD instructions are used to process multiple data elements in parallel.

### Matching Engine Strategy

The matching engine implements a price-time priority algorithm:

1. Incoming buy orders are matched against the ask side starting with the lowest price
2. Incoming sell orders are matched against the bid side starting with the highest price
3. Orders at the same price level are matched in time priority (FIFO)
4. Supports various order types: limit, market, IOC, FOK, etc.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.