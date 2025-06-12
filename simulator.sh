#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}HFT Order Book Simulator${NC}"
echo -e "${BLUE}=======================${NC}"

# Check if project exists
if [ ! -d "hft-orderbook" ]; then
    echo -e "${RED}Error: Project directory 'hft-orderbook' not found.${NC}"
    echo -e "Please run ${YELLOW}./make.sh${NC} first to create the project."
    exit 1
fi

cd hft-orderbook

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Cargo is not installed.${NC}"
    echo -e "Please install Rust and Cargo from https://rustup.rs/"
    exit 1
fi

# Parse command-line arguments
LOAD="medium"
DURATION=60
OUTPUT=""
BENCHMARK=false
REPORT=false
MARKET_DEPTH=false
MARKET_DEPTH_INTERVAL=1000
MARKET_IMPACT_SIZES=""
COMMAND="simulate"

print_usage() {
    echo -e "\nUsage: $0 [options]"
    echo -e "\nOptions:"
    echo -e "  -l, --load LEVEL     Load level: low, medium, high, max (default: medium)"
    echo -e "  -d, --duration SECS  Duration in seconds (default: 60)"
    echo -e "  -o, --output FILE    Output file for results"
    echo -e "  -b, --benchmark      Run benchmark instead of simulation"
    echo -e "  -r, --report         Generate HTML report after run"
    echo -e "  -m, --market-depth   Enable market depth analysis"
    echo -e "  -i, --interval MS    Market depth sample interval in milliseconds (default: 1000)"
    echo -e "  -s, --sizes SIZES    Market impact sizes to analyze (comma-separated values)"
    echo -e "      --depth-example  Run the market depth analysis example"
    echo -e "  -h, --help           Show this help message"
    echo -e "\nExamples:"
    echo -e "  $0 --load high --duration 120 --output results.json --report"
    echo -e "  $0 --market-depth --interval 500 --sizes \"1.0,5.0,10.0,20.0\""
    echo -e "  $0 --depth-example"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -l|--load)
            LOAD="$2"
            shift 2
            ;;
        -d|--duration)
            DURATION="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT="$2"
            shift 2
            ;;
        -b|--benchmark)
            BENCHMARK=true
            shift
            ;;
        -r|--report)
            REPORT=true
            shift
            ;;
        -m|--market-depth)
            MARKET_DEPTH=true
            shift
            ;;
        -i|--interval)
            MARKET_DEPTH_INTERVAL="$2"
            shift 2
            ;;
        -s|--sizes)
            MARKET_IMPACT_SIZES="$2"
            shift 2
            ;;
        --depth-example)
            COMMAND="market-depth"
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

# Validate load level
case $LOAD in
    low|medium|high|max)
        # Valid load level
        ;;
    *)
        echo -e "${RED}Error: Invalid load level '$LOAD'.${NC}"
        echo -e "Valid load levels are: low, medium, high, max"
        exit 1
        ;;
esac

# Build the project
echo -e "${GREEN}Building project...${NC}"
cargo build --release

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to build project.${NC}"
    exit 1
fi

# Create results directory if it doesn't exist
RESULTS_DIR="results"
mkdir -p $RESULTS_DIR

# Generate timestamp for output files
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DEFAULT_OUTPUT="$RESULTS_DIR/simulation_${LOAD}_${TIMESTAMP}.json"

if [ -z "$OUTPUT" ]; then
    OUTPUT=$DEFAULT_OUTPUT
fi

if [ "$BENCHMARK" = true ]; then
    # Run benchmark
    echo -e "${GREEN}Running benchmark...${NC}"
    cargo bench
elif [ "$COMMAND" = "market-depth" ]; then
    # Run market depth example
    echo -e "${GREEN}Running market depth analysis example...${NC}"
    
    OUTPUT_PARAM=""
    if [ ! -z "$OUTPUT" ]; then
        OUTPUT_PARAM="--output $OUTPUT"
    fi
    
    cargo run --release -- market-depth $OUTPUT_PARAM
else
    # Run simulation
    echo -e "${GREEN}Running simulation with load level '${LOAD}' for ${DURATION} seconds...${NC}"
    
    COMMAND_ARGS="simulate --load $LOAD --duration $DURATION"
    
    if [ ! -z "$OUTPUT" ]; then
        COMMAND_ARGS="$COMMAND_ARGS --output $OUTPUT"
    fi
    
    if [ "$MARKET_DEPTH" = true ]; then
        COMMAND_ARGS="$COMMAND_ARGS --market-depth --market-depth-interval $MARKET_DEPTH_INTERVAL"
    fi
    
    if [ ! -z "$MARKET_IMPACT_SIZES" ]; then
        COMMAND_ARGS="$COMMAND_ARGS --market-impact-sizes \"$MARKET_IMPACT_SIZES\""
    fi
    
    cargo run --release -- $COMMAND_ARGS
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: Simulation failed.${NC}"
        exit 1
    fi
fi

# Generate report if requested
if [ "$REPORT" = true ] && [ "$BENCHMARK" = false ] && [ "$COMMAND" = "simulate" ]; then
    echo -e "${GREEN}Generating HTML report...${NC}"
    
    REPORT_FILE="${OUTPUT%.json}_report.html"
    
    # Check if output file exists
    if [ ! -f "$OUTPUT" ]; then
        echo -e "${RED}Error: Output file '$OUTPUT' not found.${NC}"
        exit 1
    fi
    
    # Generate HTML report
    cat > $REPORT_FILE << EOF
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>HFT Order Book Simulation Report</title>
    <style>
        body {
            font-family: Arial, sans-serif;
            line-height: 1.6;
            margin: 0;
            padding: 20px;
            color: #333;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
        }
        h1 {
            color: #2c3e50;
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }
        h2 {
            color: #2980b9;
            margin-top: 30px;
        }
        .card {
            background-color: #f9f9f9;
            border-radius: 5px;
            padding: 20px;
            margin-bottom: 20px;
            box-shadow: 0 2px 5px rgba(0,0,0,0.1);
        }
        .metric {
            display: flex;
            justify-content: space-between;
            margin-bottom: 10px;
        }
        .metric-name {
            font-weight: bold;
        }
        .chart {
            background-color: white;
            border-radius: 5px;
            padding: 20px;
            box-shadow: 0 2px 5px rgba(0,0,0,0.1);
            height: 300px;
            margin-top: 20px;
        }
        .footer {
            margin-top: 50px;
            text-align: center;
            font-size: 0.8em;
            color: #7f8c8d;
        }
        .tab {
            overflow: hidden;
            border: 1px solid #ccc;
            background-color: #f1f1f1;
            border-radius: 5px 5px 0 0;
        }
        .tab button {
            background-color: inherit;
            float: left;
            border: none;
            outline: none;
            cursor: pointer;
            padding: 14px 16px;
            transition: 0.3s;
            font-size: 17px;
        }
        .tab button:hover {
            background-color: #ddd;
        }
        .tab button.active {
            background-color: #3498db;
            color: white;
        }
        .tabcontent {
            display: none;
            padding: 6px 12px;
            border: 1px solid #ccc;
            border-top: none;
            border-radius: 0 0 5px 5px;
            animation: fadeEffect 1s;
        }
        @keyframes fadeEffect {
            from {opacity: 0;}
            to {opacity: 1;}
        }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <div class="container">
        <h1>HFT Order Book Simulation Report</h1>
        
        <div class="tab">
            <button class="tablinks" onclick="openTab(event, 'Performance')" id="defaultOpen">Performance</button>
            <button class="tablinks" onclick="openTab(event, 'MarketDepth')">Market Depth</button>
            <button class="tablinks" onclick="openTab(event, 'MarketImpact')">Market Impact</button>
        </div>
        
        <div id="Performance" class="tabcontent">
            <div class="card">
                <h2>Simulation Parameters</h2>
                <div class="metric">
                    <span class="metric-name">Load Level:</span>
                    <span class="metric-value" id="load-level"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Duration:</span>
                    <span class="metric-value" id="duration"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Timestamp:</span>
                    <span class="metric-value" id="timestamp"></span>
                </div>
            </div>
            
            <div class="card">
                <h2>Performance Metrics</h2>
                <div class="metric">
                    <span class="metric-name">Orders Processed:</span>
                    <span class="metric-value" id="orders-processed"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Orders Per Second:</span>
                    <span class="metric-value" id="orders-per-second"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Memory Usage:</span>
                    <span class="metric-value" id="memory-usage"></span>
                </div>
            </div>
            
            <div class="card">
                <h2>Latency Metrics (microseconds)</h2>
                <div class="metric">
                    <span class="metric-name">Minimum Latency:</span>
                    <span class="metric-value" id="min-latency"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Maximum Latency:</span>
                    <span class="metric-value" id="max-latency"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Average Latency:</span>
                    <span class="metric-value" id="avg-latency"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">50th Percentile (P50):</span>
                    <span class="metric-value" id="p50-latency"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">99th Percentile (P99):</span>
                    <span class="metric-value" id="p99-latency"></span>
                </div>
                
                <div class="chart">
                    <canvas id="latency-chart"></canvas>
                </div>
            </div>
        </div>
        
        <div id="MarketDepth" class="tabcontent">
            <div class="card">
                <h2>Spread Statistics</h2>
                <div class="metric">
                    <span class="metric-name">Minimum Spread:</span>
                    <span class="metric-value" id="min-spread"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Maximum Spread:</span>
                    <span class="metric-value" id="max-spread"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Average Spread:</span>
                    <span class="metric-value" id="avg-spread"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Spread (bps):</span>
                    <span class="metric-value" id="avg-spread-bps"></span>
                </div>
                
                <div class="chart">
                    <canvas id="spread-chart"></canvas>
                </div>
            </div>
            
            <div class="card">
                <h2>Liquidity Statistics</h2>
                <div class="metric">
                    <span class="metric-name">Average Bid Volume:</span>
                    <span class="metric-value" id="avg-bid-volume"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Average Ask Volume:</span>
                    <span class="metric-value" id="avg-ask-volume"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Bid/Ask Ratio:</span>
                    <span class="metric-value" id="bid-ask-ratio"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Top Level Concentration:</span>
                    <span class="metric-value" id="top-concentration"></span>
                </div>
                
                <div class="chart">
                    <canvas id="volume-chart"></canvas>
                </div>
            </div>
            
            <div class="card">
                <h2>Imbalance Statistics</h2>
                <div class="metric">
                    <span class="metric-name">Minimum Imbalance:</span>
                    <span class="metric-value" id="min-imbalance"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Maximum Imbalance:</span>
                    <span class="metric-value" id="max-imbalance"></span>
                </div>
                <div class="metric">
                    <span class="metric-name">Average Imbalance:</span>
                    <span class="metric-value" id="avg-imbalance"></span>
                </div>
                
                <div class="chart">
                    <canvas id="imbalance-chart"></canvas>
                </div>
            </div>
        </div>
        
        <div id="MarketImpact" class="tabcontent">
            <div class="card">
                <h2>Market Impact Analysis</h2>
                <div id="impact-metrics">
                    <!-- Market impact metrics will be populated here -->
                </div>
                
                <div class="chart">
                    <canvas id="impact-chart"></canvas>
                </div>
            </div>
        </div>
        
        <div class="footer">
            Generated on <span id="generation-date"></span>
        </div>
    </div>
    
    <script>
        // Tab functionality
        function openTab(evt, tabName) {
            var i, tabcontent, tablinks;
            tabcontent = document.getElementsByClassName("tabcontent");
            for (i = 0; i < tabcontent.length; i++) {
                tabcontent[i].style.display = "none";
            }
            tablinks = document.getElementsByClassName("tablinks");
            for (i = 0; i < tablinks.length; i++) {
                tablinks[i].className = tablinks[i].className.replace(" active", "");
            }
            document.getElementById(tabName).style.display = "block";
            evt.currentTarget.className += " active";
        }
        
        // Load the JSON data
        fetch('$(basename $OUTPUT)')
            .then(response => response.json())
            .then(data => {
                // Populate simulation parameters
                document.getElementById('load-level').textContent = data.mode;
                document.getElementById('duration').textContent = data.duration.toFixed(2) + ' seconds';
                document.getElementById('timestamp').textContent = new Date().toLocaleString();
                
                // Populate performance metrics
                document.getElementById('orders-processed').textContent = data.orders_processed.toLocaleString();
                document.getElementById('orders-per-second').textContent = data.orders_per_second.toFixed(2).toLocaleString();
                document.getElementById('memory-usage').textContent = data.memory_usage.toFixed(2) + ' MB';
                
                // Populate latency metrics
                document.getElementById('min-latency').textContent = data.latency.min.toFixed(2);
                document.getElementById('max-latency').textContent = data.latency.max.toFixed(2);
                document.getElementById('avg-latency').textContent = data.latency.avg.toFixed(2);
                document.getElementById('p50-latency').textContent = data.latency.p50.toFixed(2);
                document.getElementById('p99-latency').textContent = data.latency.p99.toFixed(2);
                
                // Create latency chart
                const latencyCtx = document.getElementById('latency-chart').getContext('2d');
                new Chart(latencyCtx, {
                    type: 'bar',
                    data: {
                        labels: ['Min', 'Avg', 'P50', 'P99', 'Max'],
                        datasets: [{
                            label: 'Latency (microseconds)',
                            data: [
                                data.latency.min,
                                data.latency.avg,
                                data.latency.p50,
                                data.latency.p99,
                                data.latency.max
                            ],
                            backgroundColor: [
                                'rgba(46, 204, 113, 0.6)',
                                'rgba(52, 152, 219, 0.6)',
                                'rgba(241, 196, 15, 0.6)',
                                'rgba(230, 126, 34, 0.6)',
                                'rgba(231, 76, 60, 0.6)'
                            ],
                            borderColor: [
                                'rgba(46, 204, 113, 1)',
                                'rgba(52, 152, 219, 1)',
                                'rgba(241, 196, 15, 1)',
                                'rgba(230, 126, 34, 1)',
                                'rgba(231, 76, 60, 1)'
                            ],
                            borderWidth: 1
                        }]
                    },
                    options: {
                        scales: {
                            y: {
                                beginAtZero: true
                            }
                        }
                    }
                });
                
                // Populate market depth metrics if available
                if (data.market_depth) {
                    const md = data.market_depth;
                    
                    // Populate spread statistics
                    document.getElementById('min-spread').textContent = md.spread_stats.min.toFixed(2);
                    document.getElementById('max-spread').textContent = md.spread_stats.max.toFixed(2);
                    document.getElementById('avg-spread').textContent = md.spread_stats.avg.toFixed(2);
                    document.getElementById('avg-spread-bps').textContent = md.spread_stats.avg_bps.toFixed(2) + ' bps';
                    
                    // Create spread chart
                    const spreadCtx = document.getElementById('spread-chart').getContext('2d');
                    new Chart(spreadCtx, {
                        type: 'bar',
                        data: {
                            labels: ['Min', 'Avg', 'Max'],
                            datasets: [{
                                label: 'Spread',
                                data: [
                                    md.spread_stats.min,
                                    md.spread_stats.avg,
                                    md.spread_stats.max
                                ],
                                backgroundColor: [
                                    'rgba(46, 204, 113, 0.6)',
                                    'rgba(52, 152, 219, 0.6)',
                                    'rgba(231, 76, 60, 0.6)'
                                ],
                                borderColor: [
                                    'rgba(46, 204, 113, 1)',
                                    'rgba(52, 152, 219, 1)',
                                    'rgba(231, 76, 60, 1)'
                                ],
                                borderWidth: 1
                            }]
                        },
                        options: {
                            scales: {
                                y: {
                                    beginAtZero: true
                                }
                            }
                        }
                    });
                    
                    // Populate liquidity statistics
                    document.getElementById('avg-bid-volume').textContent = md.liquidity_stats.total_bid_volume.toFixed(4);
                    document.getElementById('avg-ask-volume').textContent = md.liquidity_stats.total_ask_volume.toFixed(4);
                    document.getElementById('bid-ask-ratio').textContent = md.imbalance_stats.bid_ask_ratio_avg.toFixed(2);
                    document.getElementById('top-concentration').textContent = (md.liquidity_stats.top_level_concentration * 100).toFixed(2) + '%';
                    
                    // Create volume chart
                    const volumeCtx = document.getElementById('volume-chart').getContext('2d');
                    new Chart(volumeCtx, {
                        type: 'bar',
                        data: {
                            labels: ['Bid Volume', 'Ask Volume'],
                            datasets: [{
                                label: 'Average Volume',
                                data: [
                                    md.liquidity_stats.total_bid_volume,
                                    md.liquidity_stats.total_ask_volume
                                ],
                                backgroundColor: [
                                    'rgba(46, 204, 113, 0.6)',
                                    'rgba(231, 76, 60, 0.6)'
                                ],
                                borderColor: [
                                    'rgba(46, 204, 113, 1)',
                                    'rgba(231, 76, 60, 1)'
                                ],
                                borderWidth: 1
                            }]
                        },
                        options: {
                            scales: {
                                y: {
                                    beginAtZero: true
                                }
                            }
                        }
                    });
                    
                    // Populate imbalance statistics
                    document.getElementById('min-imbalance').textContent = md.imbalance_stats.min.toFixed(4);
                    document.getElementById('max-imbalance').textContent = md.imbalance_stats.max.toFixed(4);
                    document.getElementById('avg-imbalance').textContent = md.imbalance_stats.avg.toFixed(4);
                    
                    // Create imbalance chart
                    const imbalanceCtx = document.getElementById('imbalance-chart').getContext('2d');
                    new Chart(imbalanceCtx, {
                        type: 'bar',
                        data: {
                            labels: ['Min', 'Avg', 'Max'],
                            datasets: [{
                                label: 'Imbalance',
                                data: [
                                    md.imbalance_stats.min,
                                    md.imbalance_stats.avg,
                                    md.imbalance_stats.max
                                ],
                                backgroundColor: [
                                    'rgba(46, 204, 113, 0.6)',
                                    'rgba(52, 152, 219, 0.6)',
                                    'rgba(231, 76, 60, 0.6)'
                                ],
                                borderColor: [
                                    'rgba(46, 204, 113, 1)',
                                    'rgba(52, 152, 219, 1)',
                                    'rgba(231, 76, 60, 1)'
                                ],
                                borderWidth: 1
                            }]
                        },
                        options: {
                            scales: {
                                y: {
                                    beginAtZero: true
                                }
                            }
                        }
                    });
                } else {
                    // Hide market depth tab if data not available
                    document.querySelector('button[onclick="openTab(event, \'MarketDepth\')"]').style.display = 'none';
                }
                
                // Populate market impact metrics if available
                if (data.market_impact) {
                    const mi = data.market_impact;
                    const container = document.getElementById('impact-metrics');
                    
                    // Create metrics for each order size
                    for (let i = 0; i < mi.order_sizes.length; i++) {
                        const size = mi.order_sizes[i];
                        const buy = mi.buy_impact[i];
                        const sell = mi.sell_impact[i];
                        
                        const div = document.createElement('div');
                        div.classList.add('card');
                        div.style.marginBottom = '20px';
                        
                        div.innerHTML = `
                            <h3>Order Size: ${size.toFixed(2)}</h3>
                            <div class="metric">
                                <span class="metric-name">Buy Execution Price:</span>
                                <span class="metric-value">${buy.expected_execution_price.toFixed(2)}</span>
                            </div>
                            <div class="metric">
                                <span class="metric-name">Buy Price Impact:</span>
                                <span class="metric-value">${buy.expected_price_impact_percent.toFixed(2)}%</span>
                            </div>
                            <div class="metric">
                                <span class="metric-name">Sell Execution Price:</span>
                                <span class="metric-value">${sell.expected_execution_price.toFixed(2)}</span>
                            </div>
                            <div class="metric">
                                <span class="metric-name">Sell Price Impact:</span>
                                <span class="metric-value">${sell.expected_price_impact_percent.toFixed(2)}%</span>
                            </div>
                            <div class="metric">
                                <span class="metric-name">Round-trip Cost:</span>
                                <span class="metric-value">${((buy.expected_execution_price - sell.expected_execution_price) * size).toFixed(2)}</span>
                            </div>
                        `;
                        
                        container.appendChild(div);
                    }
                    
                    // Create impact chart
                    const impactCtx = document.getElementById('impact-chart').getContext('2d');
                    new Chart(impactCtx, {
                        type: 'bar',
                        data: {
                            labels: mi.order_sizes.map(s => s.toFixed(2) + ' BTC'),
                            datasets: [{
                                label: 'Buy Price Impact (%)',
                                data: mi.buy_impact.map(b => b.expected_price_impact_percent),
                                backgroundColor: 'rgba(46, 204, 113, 0.6)',
                                borderColor: 'rgba(46, 204, 113, 1)',
                                borderWidth: 1
                            }, {
                                label: 'Sell Price Impact (%)',
                                data: mi.sell_impact.map(s => s.expected_price_impact_percent),
                                backgroundColor: 'rgba(231, 76, 60, 0.6)',
                                borderColor: 'rgba(231, 76, 60, 1)',
                                borderWidth: 1
                            }]
                        },
                        options: {
                            scales: {
                                y: {
                                    beginAtZero: true,
                                    title: {
                                        display: true,
                                        text: 'Price Impact (%)'
                                    }
                                },
                                x: {
                                    title: {
                                        display: true,
                                        text: 'Order Size (BTC)'
                                    }
                                }
                            }
                        }
                    });
                } else {
                    // Hide market impact tab if data not available
                    document.querySelector('button[onclick="openTab(event, \'MarketImpact\')"]').style.display = 'none';
                }
                
                // Set generation date
                document.getElementById('generation-date').textContent = new Date().toLocaleString();
                
                // Open default tab
                document.getElementById("defaultOpen").click();
            })
            .catch(error => {
                console.error('Error loading simulation data:', error);
                alert('Failed to load simulation data. See console for details.');
            });
    </script>
</body>
</html>
EOF
    
    echo -e "${GREEN}HTML report generated: ${REPORT_FILE}${NC}"
fi

echo -e "${GREEN}Done!${NC}"