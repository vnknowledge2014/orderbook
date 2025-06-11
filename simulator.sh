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

print_usage() {
    echo -e "\nUsage: $0 [options]"
    echo -e "\nOptions:"
    echo -e "  -l, --load LEVEL     Load level: low, medium, high, max (default: medium)"
    echo -e "  -d, --duration SECS  Duration in seconds (default: 60)"
    echo -e "  -o, --output FILE    Output file for results"
    echo -e "  -b, --benchmark      Run benchmark instead of simulation"
    echo -e "  -r, --report         Generate HTML report after run"
    echo -e "  -h, --help           Show this help message"
    echo -e "\nExample:"
    echo -e "  $0 --load high --duration 120 --output results.json --report"
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
else
    # Run simulation
    echo -e "${GREEN}Running simulation with load level '${LOAD}' for ${DURATION} seconds...${NC}"
    
    OUTPUT_PARAM=""
    if [ ! -z "$OUTPUT" ]; then
        OUTPUT_PARAM="--output $OUTPUT"
    fi
    
    cargo run --release -- simulate --load $LOAD --duration $DURATION $OUTPUT_PARAM
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Error: Simulation failed.${NC}"
        exit 1
    fi
fi

# Generate report if requested
if [ "$REPORT" = true ] && [ "$BENCHMARK" = false ]; then
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
    </style>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
</head>
<body>
    <div class="container">
        <h1>HFT Order Book Simulation Report</h1>
        
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
        
        <div class="footer">
            Generated on <span id="generation-date"></span>
        </div>
    </div>
    
    <script>
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
                const ctx = document.getElementById('latency-chart').getContext('2d');
                new Chart(ctx, {
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
                
                // Set generation date
                document.getElementById('generation-date').textContent = new Date().toLocaleString();
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
