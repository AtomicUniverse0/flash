#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# Cloud-friendly NIC setup for AF_XDP/XDP (works with ens3/virtio-net style NICs)

set -u

usage() {
    echo "Usage: $0 -i <interface> [-q <queues>] [-m <mtu>] [--busy-poll on|off]"
    echo ""
    echo "Examples:"
    echo "  sudo $0 -i ens3"
    echo "  sudo $0 -i ens3 -q 1 --busy-poll off"
    echo ""
    echo "Notes:"
    echo "  - This script is best-effort and skips unsupported NIC features."
    echo "  - Intended for cloud NICs where many ethtool options are unsupported."
    exit 1
}

log() {
    echo "[setup_cloud_nic] $*"
}

warn() {
    echo "[setup_cloud_nic][warn] $*"
}

run_try() {
    local desc="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        log "OK: ${desc}"
        return 0
    fi
    warn "SKIP: ${desc} (unsupported or failed)"
    return 1
}

if [[ "${EUID}" -ne 0 ]]; then
    echo "Please run as root (sudo)."
    exit 1
fi

NIC=""
QUEUES=""
MTU="1500"
BUSY_POLL_MODE="off"

while [[ $# -gt 0 ]]; do
    case "$1" in
        -i|--interface)
            NIC="$2"
            shift 2
            ;;
        -q|--queues)
            QUEUES="$2"
            shift 2
            ;;
        -m|--mtu)
            MTU="$2"
            shift 2
            ;;
        -b|--busy-poll)
            BUSY_POLL_MODE="on"
            shift 1
            ;;
        -h|--help)
            usage
            ;;
        *)
            usage
            ;;
    esac
done

if [[ -z "${NIC}" ]]; then
    usage
fi

if ! ip link show dev "${NIC}" >/dev/null 2>&1; then
    echo "Interface ${NIC} not found."
    exit 1
fi

if ! command -v ethtool >/dev/null 2>&1; then
    echo "ethtool not found. Install it first."
    exit 1
fi

DRIVER="$(ethtool -i "${NIC}" 2>/dev/null | awk '/^driver:/ {print $2}')"
if [[ -z "${DRIVER}" ]]; then
    DRIVER="unknown"
fi

log "Interface: ${NIC}"
log "Driver: ${DRIVER}"

if [[ -z "${QUEUES}" ]]; then
    QUEUES="$(ethtool -l "${NIC}" 2>/dev/null | awk '/Combined:/ && !seen {seen=1; print $2}')"
    if [[ -z "${QUEUES}" ]]; then
        QUEUES="1"
    fi
fi

if [[ ! "${QUEUES}" =~ ^[0-9]+$ ]] || [[ "${QUEUES}" -lt 1 ]]; then
    echo "Invalid queue count: ${QUEUES}"
    exit 1
fi

log "Target queues: ${QUEUES}"
log "Setting link up and MTU"
run_try "ip link set dev ${NIC} up" ip link set dev "${NIC}" up
run_try "ip link set dev ${NIC} mtu ${MTU}" ip link set dev "${NIC}" mtu "${MTU}"

log "Applying safe ethtool tuning for XDP/AF_XDP"
run_try "set combined queues=${QUEUES}" ethtool -L "${NIC}" combined "${QUEUES}"
run_try "disable pause frames" ethtool -A "${NIC}" rx off tx off
run_try "disable adaptive coalesce and usecs" ethtool -C "${NIC}" adaptive-rx off adaptive-tx off rx-usecs 0 tx-usecs 0

# Disable common offloads (best-effort; some cloud NICs support only a subset)
for feat in gro gso tso lro rxvlan txvlan rx-checksumming tx-checksumming; do
    run_try "disable offload ${feat}" ethtool -K "${NIC}" "${feat}" off

done

log "System-level best-effort tuning"
run_try "sysctl vm.zone_reclaim_mode=0" sysctl -w vm.zone_reclaim_mode=0
run_try "sysctl vm.swappiness=0" sysctl -w vm.swappiness=0
run_try "sysctl net.core.bpf_jit_enable=1" sysctl -w net.core.bpf_jit_enable=1

# Busy-poll related knobs (optional and best-effort)
if [[ "${BUSY_POLL_MODE}" == "on" ]]; then
    run_try "sysctl net.core.busy_poll=50" sysctl -w net.core.busy_poll=50
    run_try "sysctl net.core.busy_read=50" sysctl -w net.core.busy_read=50
    if [[ -w "/sys/class/net/${NIC}/napi_defer_hard_irqs" ]]; then
        run_try "enable napi_defer_hard_irqs" sh -c "echo 2 > /sys/class/net/${NIC}/napi_defer_hard_irqs"
    fi
    if [[ -w "/sys/class/net/${NIC}/gro_flush_timeout" ]]; then
        run_try "set gro_flush_timeout" sh -c "echo 200000 > /sys/class/net/${NIC}/gro_flush_timeout"
    fi
else
    if [[ -w "/sys/class/net/${NIC}/napi_defer_hard_irqs" ]]; then
        run_try "disable napi_defer_hard_irqs" sh -c "echo 0 > /sys/class/net/${NIC}/napi_defer_hard_irqs"
    fi
    if [[ -w "/sys/class/net/${NIC}/gro_flush_timeout" ]]; then
        run_try "reset gro_flush_timeout" sh -c "echo 0 > /sys/class/net/${NIC}/gro_flush_timeout"
    fi
fi

log "Done. Current link settings:"
ethtool "${NIC}" 2>/dev/null | sed -n '1,25p' || true

cat <<'EOF'

Next steps to run XDP/AF_XDP with FLASH:
1. Ensure your FLASH config ifname matches this NIC (e.g., ens3).
2. Start monitor as root.
3. Load config and start NF process.

EOF
