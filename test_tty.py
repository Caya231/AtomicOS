import pexpect
import sys
import time

child = pexpect.spawn('qemu-system-x86_64 -drive format=raw,file=build/disk.img,if=ide,index=0 -cdrom build/atomic_os.iso -boot d -nographic', encoding='utf-8')
child.logfile_read = sys.stdout

try:
    # Wait for OS and pipe.elf to finish
    child.expect("IPC message from Child via Pipe!", timeout=5)
    print("\n--- TEST FINISHED. SENDING KEYSTROKES ---")
    time.sleep(1)
    
    # Send some characters to test TTY
    child.sendline("ls")
    child.expect("ls", timeout=2) # Echo
    print("\n--- KEBYOARD ALIVE ---")
except Exception as e:
    print(f"\n--- TIMEOUT OR ERROR: {e} ---")

child.terminate(force=True)
