#!/bin/bash
set -e

echo "==========================================="
echo "  AtomicOS — Build & Run"
echo "==========================================="

case "${1:-run}" in
    run)
        echo "[*] Building and booting in QEMU..."
        make run
        ;;
    debug)
        echo "[*] Building and booting in DEBUG mode..."
        echo "[*] Connect GDB: gdb build/kernel.bin -ex 'target remote :1234'"
        make debug
        ;;
    clean)
        echo "[*] Cleaning build artifacts..."
        make clean
        ;;
    iso)
        echo "[*] Building ISO only..."
        make iso
        ;;
    *)
        echo "Usage: ./run.sh [run|debug|clean|iso]"
        exit 1
        ;;
esac
