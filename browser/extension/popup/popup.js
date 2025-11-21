// AnonNet Extension Popup Script
// Fetches and displays credit balance, network status, and circuit information

// Try to discover the API port (daemon auto-selects a free port)
let API_BASE = null;
const POSSIBLE_PORTS = [19150, 19151, 19152, 19153, 19154, 19155, 9150, 9151, 8150];

// Discover which port the API is running on
async function discoverApiPort() {
    for (const port of POSSIBLE_PORTS) {
        try {
            const response = await fetch(`http://127.0.0.1:${port}/health`, {
                method: 'GET',
                signal: AbortSignal.timeout(500) // 500ms timeout
            });
            if (response.ok) {
                API_BASE = `http://127.0.0.1:${port}`;
                console.log('Discovered API on port', port);
                return true;
            }
        } catch (e) {
            // Port not responding, try next
            continue;
        }
    }
    return false;
}

// DOM Elements
const elements = {
    statusIndicator: document.getElementById('status-indicator'),
    statusText: document.getElementById('status-text'),
    creditBalance: document.getElementById('credit-balance'),
    creditsEarned: document.getElementById('credits-earned'),
    creditsSpent: document.getElementById('credits-spent'),
    earningRate: document.getElementById('earning-rate'),
    spendingRate: document.getElementById('spending-rate'),
    peerCount: document.getElementById('peer-count'),
    activePeers: document.getElementById('active-peers'),
    circuits: document.getElementById('circuits'),
    activeCircuits: document.getElementById('active-circuits'),
    bandwidth: document.getElementById('bandwidth'),
    nodeId: document.getElementById('node-id'),
    refreshButton: document.getElementById('refresh-button'),
    errorMessage: document.getElementById('error-message'),
    errorText: document.getElementById('error-text'),
};

// Format numbers with commas
function formatNumber(num) {
    return num.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

// Format bytes to human-readable format
function formatBytes(bytes) {
    if (bytes === 0) return '0 B/s';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
}

// Format rate (credits per hour)
function formatRate(rate) {
    if (rate === 0) return '0';
    return rate.toFixed(2);
}

// Truncate node ID for display
function truncateNodeId(nodeId) {
    if (nodeId.length <= 16) return nodeId;
    return nodeId.substring(0, 12) + '...' + nodeId.substring(nodeId.length - 4);
}

// Update UI with credit data
function updateCreditUI(data) {
    const balanceElement = elements.creditBalance.querySelector('.amount');
    balanceElement.textContent = formatNumber(data.balance);
    elements.creditsEarned.textContent = formatNumber(data.total_earned);
    elements.creditsSpent.textContent = formatNumber(data.total_spent);
    elements.earningRate.textContent = formatRate(data.earning_rate);
    elements.spendingRate.textContent = formatRate(data.spending_rate);
}

// Update UI with network data
function updateNetworkUI(data) {
    elements.peerCount.textContent = data.peer_count;
    elements.activePeers.textContent = data.active_peers;
    elements.circuits.textContent = data.total_circuits;
    elements.activeCircuits.textContent = data.active_circuits;
    elements.bandwidth.textContent = formatBytes(data.bandwidth);
    elements.nodeId.textContent = truncateNodeId(data.node_id);

    // Update circuit hops info
    const circuitHopsElement = document.getElementById('circuit-hops');
    if (circuitHopsElement) {
        const avgHops = data.average_circuit_hops || 0;
        circuitHopsElement.textContent = `${avgHops.toFixed(1)}-hop circuit (avg)`;

        // Show warning if insecure
        if (data.insecure_circuits) {
            circuitHopsElement.style.color = '#ff9800';
            circuitHopsElement.textContent += ' ⚠️';
        } else {
            circuitHopsElement.style.color = '';
        }
    }

    // Update status indicator and show security warning
    if (data.is_running) {
        elements.statusIndicator.classList.add('connected');
        elements.statusIndicator.classList.remove('disconnected');

        if (data.security_warning) {
            elements.statusText.textContent = data.security_warning;
            elements.statusIndicator.classList.add('warning');
        } else {
            elements.statusText.textContent = 'Connected to AnonNet';
            elements.statusIndicator.classList.remove('warning');
        }
    } else {
        elements.statusIndicator.classList.add('disconnected');
        elements.statusIndicator.classList.remove('connected');
        elements.statusText.textContent = 'Disconnected';
    }
}

// Show error message
function showError(message) {
    elements.errorText.textContent = message;
    elements.errorMessage.style.display = 'flex';

    // Update status
    elements.statusIndicator.classList.add('disconnected');
    elements.statusIndicator.classList.remove('connected');
    elements.statusText.textContent = 'Error connecting to daemon';
}

// Hide error message
function hideError() {
    elements.errorMessage.style.display = 'none';
}

// Fetch credit stats from API
async function fetchCreditStats() {
    try {
        const response = await fetch(`${API_BASE}/api/credits/stats`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (error) {
        throw new Error(`Failed to fetch credit stats: ${error.message}`);
    }
}

// Fetch network status from API
async function fetchNetworkStatus() {
    try {
        const response = await fetch(`${API_BASE}/api/network/status`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        return await response.json();
    } catch (error) {
        throw new Error(`Failed to fetch network status: ${error.message}`);
    }
}

// Fetch all data and update UI
async function refreshData() {
    try {
        hideError();

        // Discover API port if not already known
        if (!API_BASE) {
            const found = await discoverApiPort();
            if (!found) {
                throw new Error('Could not find AnonNet daemon. Is it running?');
            }
        }

        // Fetch both credit stats and network status in parallel
        const [creditData, networkData] = await Promise.all([
            fetchCreditStats(),
            fetchNetworkStatus(),
        ]);

        // Update UI with fetched data
        updateCreditUI(creditData);
        updateNetworkUI(networkData);

    } catch (error) {
        console.error('Error fetching data:', error);
        showError(error.message);
        // Reset API_BASE to retry discovery next time
        API_BASE = null;
    }
}

// Animate refresh icon during refresh
async function handleRefresh() {
    const refreshIcon = elements.refreshButton.querySelector('.refresh-icon');
    refreshIcon.style.transform = 'rotate(360deg)';

    await refreshData();

    setTimeout(() => {
        refreshIcon.style.transform = 'rotate(0deg)';
    }, 300);
}

// Event listeners
elements.refreshButton.addEventListener('click', handleRefresh);

// Auto-refresh every 5 seconds
setInterval(refreshData, 5000);

// Initial load
refreshData();
